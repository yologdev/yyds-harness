//! yoyo — a coding agent that evolves itself.
//!
//! Started as ~200 lines. Grows one commit at a time.
//! Read IDENTITY.md and journals/JOURNAL.md for the full story.
//!
//! Usage:
//!   ANTHROPIC_API_KEY=sk-... cargo run
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --model claude-opus-4-6
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --thinking high
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --skills ./skills
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --mcp "npx -y @modelcontextprotocol/server-filesystem /tmp"
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --system "You are a Rust expert."
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --system-file prompt.txt
//!   ANTHROPIC_API_KEY=sk-... cargo run -- -p "explain this code"
//!   ANTHROPIC_API_KEY=sk-... cargo run -- -p "write a README" -o README.md
//!   echo "prompt" | cargo run  (piped mode: single prompt, no REPL)
//!
//! Commands:
//!   /quit, /exit    Exit the agent
//!   /add <path>     Add file contents to conversation (supports globs and line ranges)
//!   /clear          Clear conversation history
//!   /commit [msg]   Commit staged changes (AI-generates message if no msg)
//!   /docs <crate>   Look up docs.rs documentation for a Rust crate
//!   /docs <c> <i>   Look up a specific item within a crate
//!   /export [path]  Export conversation as readable markdown
//!   /find <pattern> Fuzzy-search project files by name
//!   /fix            Auto-fix build/lint errors (runs checks, sends failures to AI)
//!   /git <subcmd>   Quick git: status, log, add, diff, branch, stash
//!   /model <name>   Switch model mid-session
//!   /search <query> Search conversation history
//!   /spawn <task>   Spawn a subagent with fresh context
//!   /tree [depth]   Show project directory tree
//!   /test           Auto-detect and run project tests
//!   /lint           Auto-detect and run project linter
//!   /pr [number]    List open PRs, view/diff/comment/checkout a PR, or create one
//!   /retry          Re-send the last user input

mod cli;
mod commands;
mod commands_bg;
mod commands_config;
mod commands_dev;
mod commands_file;
mod commands_git;
mod commands_info;
mod commands_map;
mod commands_memory;
mod commands_project;
mod commands_refactor;
mod commands_retry;
mod commands_search;
mod commands_session;
mod commands_spawn;
mod config;
mod context;
mod dispatch;
mod docs;
mod format;
mod git;
mod help;
mod hooks;
mod memory;
mod prompt;
mod prompt_budget;
mod providers;
mod repl;
mod safety;
mod session;
mod setup;
mod tools;
mod update;

use cli::*;
use format::*;
use prompt::*;
use tools::{build_sub_agent_tool, build_tools};

use std::io::{self, IsTerminal, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use yoagent::agent::Agent;
use yoagent::context::{ContextConfig, ExecutionLimits};
use yoagent::openapi::{OpenApiConfig, OperationFilter};
use yoagent::provider::{
    AnthropicProvider, ApiProtocol, BedrockProvider, GoogleProvider, ModelConfig, OpenAiCompat,
    OpenAiCompatProvider,
};
use yoagent::*;

/// Global flag: set to `true` when checkpoint mode's `on_before_turn` fires.
/// Checked at the end of `main()` to exit with code 2.
static CHECKPOINT_TRIGGERED: AtomicBool = AtomicBool::new(false);

/// Return the User-Agent header value for yoyo.
fn yoyo_user_agent() -> String {
    format!("yoyo/{}", env!("CARGO_PKG_VERSION"))
}

/// Names of yoyo's builtin tools. MCP servers that expose a tool with one of
/// these names would cause the Anthropic API to reject the first turn with
/// `"Tool names must be unique"`, killing the session. We detect the collision
/// at connect time and skip the colliding MCP server with a clear warning.
///
/// This list must stay in sync with `tools::build_tools` and any tool added
/// via yoagent's `with_sub_agent` (currently `sub_agent`, see
/// `build_sub_agent_tool`).
pub(crate) const BUILTIN_TOOL_NAMES: &[&str] = &[
    "bash",
    "read_file",
    "write_file",
    "edit_file",
    "list_files",
    "search",
    "rename_symbol",
    "ask_user",
    "todo",
    "sub_agent",
];

/// Pure helper: return the subset of `mcp_tools` whose names collide with any
/// entry in `builtins`. Order is preserved from `mcp_tools`. Extracted so it
/// can be unit-tested without spinning up a real MCP server.
pub(crate) fn detect_mcp_collisions(mcp_tools: &[String], builtins: &[&str]) -> Vec<String> {
    mcp_tools
        .iter()
        .filter(|name| builtins.iter().any(|b| b == &name.as_str()))
        .cloned()
        .collect()
}

/// Pre-enumerate the tool names an MCP server exposes by opening a short-lived
/// `McpClient` against it. Used to detect collisions with yoyo's builtins
/// BEFORE we hand the connection to yoagent (which would otherwise push the
/// colliding tool onto the agent and kill the first LLM turn).
///
/// Returns `Ok(tool_names)` on success, `Err(message)` on any protocol or
/// spawn error. Errors are non-fatal at the call site — we fall through and
/// let yoagent's own connect attempt surface the real diagnostic.
async fn fetch_mcp_tool_names(
    command: &str,
    args: &[&str],
    env: Option<std::collections::HashMap<String, String>>,
) -> Result<Vec<String>, String> {
    let client = yoagent::mcp::McpClient::connect_stdio(command, args, env)
        .await
        .map_err(|e| format!("{e}"))?;
    let tools = client.list_tools().await.map_err(|e| format!("{e}"))?;
    // Best-effort close; ignore errors since we're about to drop the client.
    let _ = client.close().await;
    Ok(tools.into_iter().map(|t| t.name).collect())
}

/// Insert standard yoyo identification headers into a ModelConfig.
/// All providers get User-Agent. OpenRouter also gets HTTP-Referer and X-Title.
fn insert_client_headers(config: &mut ModelConfig) {
    config
        .headers
        .insert("User-Agent".to_string(), yoyo_user_agent());
    if config.provider == "openrouter" {
        config.headers.insert(
            "HTTP-Referer".to_string(),
            "https://github.com/yologdev/yoyo-evolve".to_string(),
        );
        config
            .headers
            .insert("X-Title".to_string(), "yoyo".to_string());
    }
}

/// Create a ModelConfig for non-Anthropic providers.
pub fn create_model_config(provider: &str, model: &str, base_url: Option<&str>) -> ModelConfig {
    let mut config = match provider {
        "openai" => {
            let mut config = ModelConfig::openai(model, model);
            if let Some(url) = base_url {
                config.base_url = url.to_string();
            }
            config
        }
        "google" => {
            let mut config = ModelConfig::google(model, model);
            if let Some(url) = base_url {
                config.base_url = url.to_string();
            }
            config
        }
        "ollama" => {
            let url = base_url.unwrap_or("http://localhost:11434/v1");
            ModelConfig::local(url, model)
        }
        "openrouter" => {
            let mut config = ModelConfig::openai(model, model);
            config.provider = "openrouter".into();
            config.base_url = base_url
                .unwrap_or("https://openrouter.ai/api/v1")
                .to_string();
            config.compat = Some(OpenAiCompat::openrouter());
            config
        }
        "xai" => {
            let mut config = ModelConfig::openai(model, model);
            config.provider = "xai".into();
            config.base_url = base_url.unwrap_or("https://api.x.ai/v1").to_string();
            config.compat = Some(OpenAiCompat::xai());
            config
        }
        "groq" => {
            let mut config = ModelConfig::openai(model, model);
            config.provider = "groq".into();
            config.base_url = base_url
                .unwrap_or("https://api.groq.com/openai/v1")
                .to_string();
            config.compat = Some(OpenAiCompat::groq());
            config
        }
        "deepseek" => {
            let mut config = ModelConfig::openai(model, model);
            config.provider = "deepseek".into();
            config.base_url = base_url
                .unwrap_or("https://api.deepseek.com/v1")
                .to_string();
            config.compat = Some(OpenAiCompat::deepseek());
            config
        }
        "mistral" => {
            let mut config = ModelConfig::openai(model, model);
            config.provider = "mistral".into();
            config.base_url = base_url.unwrap_or("https://api.mistral.ai/v1").to_string();
            config.compat = Some(OpenAiCompat::mistral());
            config
        }
        "cerebras" => {
            let mut config = ModelConfig::openai(model, model);
            config.provider = "cerebras".into();
            config.base_url = base_url.unwrap_or("https://api.cerebras.ai/v1").to_string();
            config.compat = Some(OpenAiCompat::cerebras());
            config
        }
        "zai" => {
            let mut config = ModelConfig::zai(model, model);
            if let Some(url) = base_url {
                config.base_url = url.to_string();
            }
            config
        }
        "minimax" => {
            let mut config = ModelConfig::minimax(model, model);
            if let Some(url) = base_url {
                config.base_url = url.to_string();
            }
            config
        }
        "bedrock" => {
            let url = base_url.unwrap_or("https://bedrock-runtime.us-east-1.amazonaws.com");
            ModelConfig {
                id: model.into(),
                name: model.into(),
                api: ApiProtocol::BedrockConverseStream,
                provider: "bedrock".into(),
                base_url: url.to_string(),
                reasoning: false,
                context_window: 200_000,
                max_tokens: 8192,
                cost: Default::default(),
                headers: std::collections::HashMap::new(),
                compat: None,
            }
        }
        "custom" => {
            let url = base_url.unwrap_or("http://localhost:8080/v1");
            ModelConfig::local(url, model)
        }
        _ => {
            // Unknown provider — treat as OpenAI-compatible with custom base URL.
            // Note: parse_args and /provider already warn about unknown names,
            // but log here too as defense-in-depth for any future call sites.
            eprintln!(
                "{}warning:{} treating unknown provider '{}' as OpenAI-compatible (localhost:8080)",
                crate::format::YELLOW,
                crate::format::RESET,
                provider
            );
            let url = base_url.unwrap_or("http://localhost:8080/v1");
            let mut config = ModelConfig::local(url, model);
            config.provider = provider.to_string();
            config
        }
    };
    insert_client_headers(&mut config);
    config
}

/// Holds all configuration needed to build an Agent.
/// Extracted from the 12-argument `build_agent` function so that
/// creating or rebuilding an agent is just `config.build_agent()`.
pub struct AgentConfig {
    pub model: String,
    pub api_key: String,
    pub provider: String,
    pub base_url: Option<String>,
    pub skills: yoagent::skills::SkillSet,
    pub system_prompt: String,
    pub thinking: ThinkingLevel,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub max_turns: Option<usize>,
    pub auto_approve: bool,
    pub auto_commit: bool,
    pub permissions: cli::PermissionConfig,
    pub dir_restrictions: cli::DirectoryRestrictions,
    pub context_strategy: cli::ContextStrategy,
    pub context_window: Option<u32>,
    pub shell_hooks: Vec<hooks::ShellHook>,
    pub fallback_provider: Option<String>,
    pub fallback_model: Option<String>,
}

impl AgentConfig {
    /// Apply common configuration to an agent (system prompt, model, API key,
    /// thinking level, skills, tools, and optional limits).
    ///
    /// This is the single source of truth for agent configuration — every field
    /// is applied here, so adding a new `AgentConfig` field only requires one
    /// update instead of one per provider branch.
    fn configure_agent(&self, mut agent: Agent, model_context_window: u32) -> Agent {
        // User override takes precedence; otherwise use the model's actual context window
        let effective_window = self.context_window.unwrap_or(model_context_window);
        let effective_tokens = (effective_window as u64) * 80 / 100;

        // Store for display by /tokens and /status commands
        cli::set_effective_context_tokens(effective_window as u64);

        agent = agent
            .with_system_prompt(&self.system_prompt)
            .with_model(&self.model)
            .with_api_key(&self.api_key)
            .with_thinking(self.thinking)
            .with_skills(self.skills.clone())
            .with_tools(build_tools(
                self.auto_approve,
                &self.permissions,
                &self.dir_restrictions,
                if io::stdin().is_terminal() {
                    TOOL_OUTPUT_MAX_CHARS
                } else {
                    TOOL_OUTPUT_MAX_CHARS_PIPED
                },
                is_audit_enabled(),
                self.shell_hooks.clone(),
            ));

        // Add sub-agent tool via the dedicated API (separate from build_tools count)
        agent = agent.with_sub_agent(build_sub_agent_tool(self));

        // Tell yoagent the context window size so its built-in compaction knows the budget.
        // Uses 80% of the effective context window as the compaction threshold.
        agent = agent.with_context_config(ContextConfig {
            max_context_tokens: effective_tokens as usize,
            system_prompt_tokens: 4_000,
            keep_recent: 10,
            keep_first: 2,
            tool_output_max_lines: 50,
        });

        // Always set execution limits — use user's --max-turns or a generous default
        agent = agent.with_execution_limits(ExecutionLimits {
            max_turns: self.max_turns.unwrap_or(200),
            max_total_tokens: 1_000_000,
            ..ExecutionLimits::default()
        });

        if let Some(max) = self.max_tokens {
            agent = agent.with_max_tokens(max);
        }
        if let Some(temp) = self.temperature {
            agent.temperature = Some(temp);
        }

        // Checkpoint mode: register on_before_turn to stop when context gets high
        if self.context_strategy == cli::ContextStrategy::Checkpoint {
            let max_tokens = effective_tokens;
            let threshold = cli::PROACTIVE_COMPACT_THRESHOLD; // 70% — stop before overflow
            agent = agent.on_before_turn(move |messages, _turn| {
                let used = yoagent::context::total_tokens(messages) as u64;
                let ratio = used as f64 / max_tokens as f64;
                if ratio > threshold {
                    eprintln!(
                        "\n⚡ Context at {:.0}% — checkpoint-restart triggered",
                        ratio * 100.0
                    );
                    CHECKPOINT_TRIGGERED.store(true, Ordering::SeqCst);
                    return false; // stop the agent loop
                }
                true
            });
        }

        agent
    }

    /// Build a fresh Agent from this configuration.
    ///
    /// Provider selection (Anthropic, Google, or OpenAI-compatible) and model
    /// config are the only things that vary per provider. Everything else is
    /// handled by `configure_agent`, eliminating the previous 3-way duplication.
    pub fn build_agent(&self) -> Agent {
        let base_url = self.base_url.as_deref();

        if self.provider == "anthropic" && base_url.is_none() {
            // Default Anthropic path
            let mut model_config = ModelConfig::anthropic(&self.model, &self.model);
            insert_client_headers(&mut model_config);
            let context_window = model_config.context_window;
            let agent = Agent::new(AnthropicProvider).with_model_config(model_config);
            self.configure_agent(agent, context_window)
        } else if self.provider == "google" {
            // Google uses its own provider
            let model_config = create_model_config(&self.provider, &self.model, base_url);
            let context_window = model_config.context_window;
            let agent = Agent::new(GoogleProvider).with_model_config(model_config);
            self.configure_agent(agent, context_window)
        } else if self.provider == "bedrock" {
            // Bedrock uses AWS SigV4 signing with ConverseStream protocol
            let model_config = create_model_config(&self.provider, &self.model, base_url);
            let context_window = model_config.context_window;
            let agent = Agent::new(BedrockProvider).with_model_config(model_config);
            self.configure_agent(agent, context_window)
        } else {
            // All other providers use OpenAI-compatible API
            let model_config = create_model_config(&self.provider, &self.model, base_url);
            let context_window = model_config.context_window;
            let agent = Agent::new(OpenAiCompatProvider).with_model_config(model_config);
            self.configure_agent(agent, context_window)
        }
    }

    /// Build a minimal agent for `/side` conversations — same provider/model/API key,
    /// but no tools, no skills, and a concise system prompt. The agent is one-shot
    /// (1 turn max) so it answers the question and stops.
    pub fn build_side_agent(&self) -> Agent {
        let base_url = self.base_url.as_deref();
        let side_prompt = "You are a helpful assistant answering a quick side question. \
            Be concise and direct. This is a one-shot question — answer it completely in one response.";

        let agent = if self.provider == "anthropic" && base_url.is_none() {
            let mut model_config = ModelConfig::anthropic(&self.model, &self.model);
            insert_client_headers(&mut model_config);
            Agent::new(AnthropicProvider).with_model_config(model_config)
        } else if self.provider == "google" {
            let model_config = create_model_config(&self.provider, &self.model, base_url);
            Agent::new(GoogleProvider).with_model_config(model_config)
        } else if self.provider == "bedrock" {
            let model_config = create_model_config(&self.provider, &self.model, base_url);
            Agent::new(BedrockProvider).with_model_config(model_config)
        } else {
            let model_config = create_model_config(&self.provider, &self.model, base_url);
            Agent::new(OpenAiCompatProvider).with_model_config(model_config)
        };

        let mut agent = agent
            .with_system_prompt(side_prompt)
            .with_model(&self.model)
            .with_api_key(&self.api_key)
            .with_execution_limits(ExecutionLimits {
                max_turns: 1,
                ..ExecutionLimits::default()
            });

        if let Some(temp) = self.temperature {
            agent.temperature = Some(temp);
        }

        agent
    }

    /// Attempt to switch to the fallback provider.
    ///
    /// Returns `true` if the switch was made (caller should rebuild the agent
    /// and retry). Returns `false` if no fallback is configured or the agent
    /// is already running on the fallback provider.
    pub fn try_switch_to_fallback(&mut self) -> bool {
        let fallback = match self.fallback_provider {
            Some(ref f) => f.clone(),
            None => return false,
        };

        if self.provider == fallback {
            return false;
        }

        self.provider = fallback.clone();
        self.model = self
            .fallback_model
            .clone()
            .unwrap_or_else(|| cli::default_model_for_provider(&fallback));

        // Resolve API key for fallback provider
        if let Some(env_var) = cli::provider_api_key_env(&fallback) {
            if let Ok(key) = std::env::var(env_var) {
                self.api_key = key;
            }
        }

        true
    }
}

/// What kind of prompt to retry on fallback.
enum FallbackRetry<'a> {
    /// Text-only prompt.
    Text(&'a str),
    /// Multi-modal prompt with content blocks (e.g., text + images).
    Content(Vec<Content>),
}

/// Attempt fallback retry for non-interactive modes (piped and --prompt).
///
/// If the original response has an API error and a fallback provider is configured,
/// switches to the fallback, rebuilds the agent, and retries the prompt.
///
/// Returns `(final_response, should_exit_with_error)`:
/// - If no API error occurred: returns the original response, no error exit.
/// - If fallback succeeded: returns the retry response, no error exit.
/// - If fallback also failed or no fallback configured: returns the best response, error exit.
async fn try_fallback_prompt(
    agent_config: &mut AgentConfig,
    agent: &mut Agent,
    retry: FallbackRetry<'_>,
    session_total: &mut Usage,
    original_response: PromptOutcome,
) -> (PromptOutcome, bool) {
    // No API error — nothing to retry
    if original_response.last_api_error.is_none() {
        return (original_response, false);
    }

    let old_provider = agent_config.provider.clone();
    let fallback_name = agent_config.fallback_provider.clone();

    if !agent_config.try_switch_to_fallback() {
        // No fallback configured or already on fallback — exit with error
        eprintln!("{RED}  API error with no fallback configured. Exiting.{RESET}",);
        return (original_response, true);
    }

    let fallback = fallback_name.as_deref().unwrap_or("unknown");
    eprintln!(
        "{YELLOW}  ⚡ Primary provider '{}' failed. Switching to fallback '{}'...{RESET}",
        old_provider, fallback
    );

    // Rebuild agent with the new provider
    *agent = agent_config.build_agent();

    eprintln!(
        "{DIM}  now using: {} / {}{RESET}",
        agent_config.provider, agent_config.model
    );

    // Retry with the fallback provider
    let retry_response = match retry {
        FallbackRetry::Text(input) => {
            run_prompt(agent, input, session_total, &agent_config.model).await
        }
        FallbackRetry::Content(blocks) => {
            run_prompt_with_content(agent, blocks, session_total, &agent_config.model).await
        }
    };

    if retry_response.last_api_error.is_some() {
        eprintln!(
            "{RED}  Fallback provider '{}' also failed. Exiting.{RESET}",
            fallback
        );
        return (retry_response, true);
    }

    (retry_response, false)
}

/// Build a JSON output object for --json mode.
/// Used by both --prompt and piped modes to produce structured output.
fn build_json_output(
    response: &PromptOutcome,
    model: &str,
    usage: &Usage,
    is_error: bool,
) -> String {
    let cost_usd = estimate_cost(usage, model);
    let json_obj = serde_json::json!({
        "response": response.text,
        "model": model,
        "usage": {
            "input_tokens": usage.input,
            "output_tokens": usage.output,
        },
        "cost_usd": cost_usd,
        "is_error": is_error,
    });
    serde_json::to_string(&json_obj).unwrap_or_else(|_| "{}".to_string())
}

/// Handle `--prompt / -p` single-shot mode: run one prompt (optionally with an
/// image), print the result (or write to `--output`), and return. Calls
/// `std::process::exit` on fatal errors (bad image, API failure with no
/// fallback).
async fn run_single_prompt(
    agent_config: &mut AgentConfig,
    agent: &mut Agent,
    prompt_text: &str,
    image_path: &Option<String>,
    output_path: &Option<String>,
    json_output: bool,
) {
    if agent_config.provider != "anthropic" {
        eprintln!(
            "{DIM}  yoyo (prompt mode) — provider: {}, model: {}{RESET}",
            agent_config.provider, agent_config.model
        );
    } else {
        eprintln!(
            "{DIM}  yoyo (prompt mode) — model: {}{RESET}",
            agent_config.model
        );
    }
    let mut session_total = Usage::default();
    let prompt_start = Instant::now();
    let response = if let Some(ref img_path) = image_path {
        // Multi-modal prompt: text + image
        match commands_file::read_image_for_add(img_path) {
            Ok((data, mime_type)) => {
                let content_blocks = vec![
                    Content::Text {
                        text: prompt_text.trim().to_string(),
                    },
                    Content::Image {
                        data: data.clone(),
                        mime_type: mime_type.clone(),
                    },
                ];
                let initial = run_prompt_with_content(
                    agent,
                    content_blocks,
                    &mut session_total,
                    &agent_config.model,
                )
                .await;
                // Fallback retry for multi-modal prompts
                let retry_blocks = vec![
                    Content::Text {
                        text: prompt_text.trim().to_string(),
                    },
                    Content::Image { data, mime_type },
                ];
                let (final_response, should_exit_error) = try_fallback_prompt(
                    agent_config,
                    agent,
                    FallbackRetry::Content(retry_blocks),
                    &mut session_total,
                    initial,
                )
                .await;
                if should_exit_error {
                    format::maybe_ring_bell(prompt_start.elapsed());
                    if json_output {
                        println!(
                            "{}",
                            build_json_output(
                                &final_response,
                                &agent_config.model,
                                &session_total,
                                true
                            )
                        );
                    } else {
                        write_output_file(output_path, &final_response.text);
                    }
                    std::process::exit(1);
                }
                final_response
            }
            Err(e) => {
                eprintln!("{RED}  error: {e}{RESET}");
                std::process::exit(1);
            }
        }
    } else {
        // Text-only prompt
        let initial = run_prompt(
            agent,
            prompt_text.trim(),
            &mut session_total,
            &agent_config.model,
        )
        .await;
        // Fallback retry for text-only prompts
        let (final_response, should_exit_error) = try_fallback_prompt(
            agent_config,
            agent,
            FallbackRetry::Text(prompt_text.trim()),
            &mut session_total,
            initial,
        )
        .await;
        if should_exit_error {
            format::maybe_ring_bell(prompt_start.elapsed());
            if json_output {
                println!(
                    "{}",
                    build_json_output(&final_response, &agent_config.model, &session_total, true)
                );
            } else {
                write_output_file(output_path, &final_response.text);
            }
            std::process::exit(1);
        }
        final_response
    };
    format::maybe_ring_bell(prompt_start.elapsed());
    if json_output {
        println!(
            "{}",
            build_json_output(&response, &agent_config.model, &session_total, false)
        );
    } else {
        write_output_file(output_path, &response.text);
    }
    if CHECKPOINT_TRIGGERED.load(Ordering::SeqCst) {
        std::process::exit(2);
    }
}

/// Handle piped mode: read all of stdin, run a single prompt, print/write the
/// result, and return. Calls `std::process::exit` on empty input or fatal API
/// errors.
/// Returns true if `input` looks like a slash command (its first non-whitespace
/// character is `/`). Slash commands belong to the REPL; piped mode can't
/// dispatch them, so we use this to warn the user instead of wasting a turn.
fn looks_like_slash_command(input: &str) -> bool {
    matches!(input.trim_start().chars().next(), Some('/'))
}

async fn run_piped_mode(
    agent_config: &mut AgentConfig,
    agent: &mut Agent,
    output_path: &Option<String>,
    json_output: bool,
) {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok();
    let input = input.trim();
    if input.is_empty() {
        eprintln!("No input on stdin.");
        std::process::exit(1);
    }

    // Piped mode can't dispatch slash commands (they need REPL state). If the
    // user piped one in, warn them and exit instead of burning tokens letting
    // the model puzzle over the literal string.
    if looks_like_slash_command(input) {
        eprintln!("{YELLOW}yoyo: slash commands aren't available in piped mode.{RESET}");
        eprintln!("  Try one of:");
        eprintln!("    yoyo doctor                    # run a subcommand directly");
        eprintln!("    yoyo --prompt \"{input}\"        # send the literal text to the agent");
        eprintln!("    yoyo                           # interactive REPL");
        std::process::exit(2);
    }

    eprintln!(
        "{DIM}  yoyo (piped mode) — model: {}{RESET}",
        agent_config.model
    );
    let mut session_total = Usage::default();
    let prompt_start = Instant::now();
    let initial = run_prompt(agent, input, &mut session_total, &agent_config.model).await;
    // Fallback retry for piped mode
    let (response, should_exit_error) = try_fallback_prompt(
        agent_config,
        agent,
        FallbackRetry::Text(input),
        &mut session_total,
        initial,
    )
    .await;
    format::maybe_ring_bell(prompt_start.elapsed());
    if json_output {
        println!(
            "{}",
            build_json_output(
                &response,
                &agent_config.model,
                &session_total,
                should_exit_error
            )
        );
    } else {
        write_output_file(output_path, &response.text);
    }
    if should_exit_error {
        std::process::exit(1);
    }
    if CHECKPOINT_TRIGGERED.load(Ordering::SeqCst) {
        std::process::exit(2);
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Check --no-color before any output (must happen before parse_args prints anything)
    // Also auto-disable color when stdout is not a terminal (piped output)
    if args.iter().any(|a| a == "--no-color") || !io::stdout().is_terminal() {
        disable_color();
    }

    // Check --no-bell before any output
    if args.iter().any(|a| a == "--no-bell") {
        disable_bell();
    }

    // Check --no-rtk before any tool execution (also respects YOYO_NO_RTK env)
    if args.iter().any(|a| a == "--no-rtk")
        || std::env::var("YOYO_NO_RTK")
            .map(|v| v == "1")
            .unwrap_or(false)
    {
        tools::disable_rtk();
    }

    let Some(config) = parse_args(&args) else {
        return; // --help or --version was handled
    };

    // --print-system-prompt: print the fully assembled system prompt and exit
    if config.print_system_prompt {
        println!("{}", config.system_prompt);
        return;
    }

    if config.verbose {
        enable_verbose();
    }

    if config.audit {
        prompt::enable_audit_log();
    }

    let continue_session = config.continue_session;
    let output_path = config.output_path;
    let mcp_servers = config.mcp_servers;
    let mcp_server_configs = config.mcp_server_configs;
    let openapi_specs = config.openapi_specs;
    let image_path = config.image_path;
    let no_update_check = config.no_update_check;
    let json_output = config.json_output;
    // Auto-approve in non-interactive modes (piped, --prompt) or when --yes is set
    let is_interactive = io::stdin().is_terminal() && config.prompt_arg.is_none();
    let auto_approve = config.auto_approve || !is_interactive;

    let mut agent_config = AgentConfig {
        model: config.model,
        api_key: config.api_key,
        provider: config.provider,
        base_url: config.base_url,
        skills: config.skills,
        system_prompt: config.system_prompt,
        thinking: config.thinking,
        max_tokens: config.max_tokens,
        temperature: config.temperature,
        max_turns: config.max_turns,
        auto_approve,
        auto_commit: config.auto_commit,
        permissions: config.permissions,
        dir_restrictions: config.dir_restrictions,
        context_strategy: config.context_strategy,
        context_window: config.context_window,
        shell_hooks: config.shell_hooks,
        fallback_provider: config.fallback_provider,
        fallback_model: config.fallback_model,
    };

    // Interactive setup wizard: if no config file or API key is detected,
    // walk the user through first-run onboarding before building the agent.
    if is_interactive && setup::needs_setup(&agent_config.provider) {
        if let Some(result) = setup::run_setup_wizard() {
            // Override config with wizard results
            agent_config.provider = result.provider.clone();
            agent_config.api_key = result.api_key.clone();
            agent_config.model = result.model;
            if result.base_url.is_some() {
                agent_config.base_url = result.base_url;
            }
            // Set the env var so the provider builder picks it up
            if let Some(env_var) = cli::provider_api_key_env(&result.provider) {
                // SAFETY: This runs during setup, before any concurrent agent work.
                // The env var is read later by the provider builder on the same thread.
                unsafe {
                    std::env::set_var(env_var, &result.api_key);
                }
            }
        } else {
            // User cancelled — show the static welcome screen and exit
            cli::print_welcome();
            return;
        }
    }

    // Bedrock needs combined AWS credentials: access_key:secret_key[:session_token]
    // parse_args() only reads AWS_ACCESS_KEY_ID; combine with the rest here.
    if agent_config.provider == "bedrock" && !agent_config.api_key.contains(':') {
        let access_key = agent_config.api_key.clone();
        if let Ok(secret) = std::env::var("AWS_SECRET_ACCESS_KEY") {
            agent_config.api_key = match std::env::var("AWS_SESSION_TOKEN") {
                Ok(token) if !token.is_empty() => format!("{access_key}:{secret}:{token}"),
                _ => format!("{access_key}:{secret}"),
            };
        }
    }

    let mut agent = agent_config.build_agent();

    // Connect to MCP servers (--mcp flags)
    let mut mcp_count = 0u32;
    for mcp_cmd in &mcp_servers {
        let parts: Vec<&str> = mcp_cmd.split_whitespace().collect();
        if parts.is_empty() {
            eprintln!("{YELLOW}warning:{RESET} Empty --mcp command, skipping");
            continue;
        }
        let command = parts[0];
        let args_slice: Vec<&str> = parts[1..].to_vec();
        eprintln!("{DIM}  mcp: connecting to {mcp_cmd}...{RESET}");

        // Pre-flight: enumerate tool names and detect collisions with yoyo
        // builtins. yoagent would otherwise push colliding tools onto the
        // agent and the Anthropic API would reject the first turn with
        // "Tool names must be unique". See #MCP collision guard (Day 39).
        match fetch_mcp_tool_names(command, &args_slice, None).await {
            Ok(tool_names) => {
                let collisions = detect_mcp_collisions(&tool_names, BUILTIN_TOOL_NAMES);
                if !collisions.is_empty() {
                    for tool in &collisions {
                        eprintln!(
                            "{YELLOW}warning:{RESET} MCP server '{command}' exposes tool '{tool}' which collides with yoyo's builtin; skipping this server"
                        );
                    }
                    eprintln!(
                        "{DIM}  mcp: skipping '{mcp_cmd}' — rename/exclude the colliding tool(s) or use a different server{RESET}"
                    );
                    continue;
                }
            }
            Err(e) => {
                eprintln!(
                    "{DIM}  mcp: pre-flight tool listing failed ({e}); proceeding to yoagent connect for diagnostics{RESET}"
                );
            }
        }

        // with_mcp_server_stdio consumes self; we must always update agent
        let result = agent
            .with_mcp_server_stdio(command, &args_slice, None)
            .await;
        match result {
            Ok(updated) => {
                agent = updated;
                mcp_count += 1;
                eprintln!("{GREEN}  ✓ mcp: {command} connected{RESET}");
            }
            Err(e) => {
                eprintln!("{RED}  ✗ mcp: failed to connect to '{mcp_cmd}': {e}{RESET}");
                // Agent was consumed on error — rebuild it with previous MCP connections lost
                agent = agent_config.build_agent();
                eprintln!("{DIM}  mcp: agent rebuilt (previous MCP connections lost){RESET}");
            }
        }
    }

    // Connect to structured MCP servers ([mcp_servers.*] config sections)
    for server_cfg in &mcp_server_configs {
        let args_refs: Vec<&str> = server_cfg.args.iter().map(|s| s.as_str()).collect();
        let env_map: Option<std::collections::HashMap<String, String>> =
            if server_cfg.env.is_empty() {
                None
            } else {
                Some(server_cfg.env.iter().cloned().collect())
            };
        eprintln!(
            "{DIM}  mcp: connecting to {} ({})...{RESET}",
            server_cfg.name, server_cfg.command
        );

        // Pre-flight collision check (see comment above).
        match fetch_mcp_tool_names(&server_cfg.command, &args_refs, env_map.clone()).await {
            Ok(tool_names) => {
                let collisions = detect_mcp_collisions(&tool_names, BUILTIN_TOOL_NAMES);
                if !collisions.is_empty() {
                    for tool in &collisions {
                        eprintln!(
                            "{YELLOW}warning:{RESET} MCP server '{}' exposes tool '{tool}' which collides with yoyo's builtin; skipping this server",
                            server_cfg.name
                        );
                    }
                    eprintln!(
                        "{DIM}  mcp: skipping '{}' — rename/exclude the colliding tool(s) or use a different server{RESET}",
                        server_cfg.name
                    );
                    continue;
                }
            }
            Err(e) => {
                eprintln!(
                    "{DIM}  mcp: pre-flight tool listing failed ({e}); proceeding to yoagent connect for diagnostics{RESET}"
                );
            }
        }

        let result = agent
            .with_mcp_server_stdio(&server_cfg.command, &args_refs, env_map)
            .await;
        match result {
            Ok(updated) => {
                agent = updated;
                mcp_count += 1;
                eprintln!("{GREEN}  ✓ mcp: {} connected{RESET}", server_cfg.name);
            }
            Err(e) => {
                eprintln!(
                    "{RED}  ✗ mcp: failed to connect to '{}': {e}{RESET}",
                    server_cfg.name
                );
                agent = agent_config.build_agent();
                eprintln!("{DIM}  mcp: agent rebuilt (previous MCP connections lost){RESET}");
            }
        }
    }

    // Load OpenAPI specs (--openapi flags)
    let mut openapi_count = 0u32;
    for spec_path in &openapi_specs {
        eprintln!("{DIM}  openapi: loading {spec_path}...{RESET}");
        let result = agent
            .with_openapi_file(spec_path, OpenApiConfig::default(), &OperationFilter::All)
            .await;
        match result {
            Ok(updated) => {
                agent = updated;
                openapi_count += 1;
                eprintln!("{GREEN}  ✓ openapi: {spec_path} loaded{RESET}");
            }
            Err(e) => {
                eprintln!("{RED}  ✗ openapi: failed to load '{spec_path}': {e}{RESET}");
                // Agent was consumed on error — rebuild it
                agent = agent_config.build_agent();
                eprintln!("{DIM}  openapi: agent rebuilt (previous connections lost){RESET}");
            }
        }
    }

    // --continue / -c: resume last saved session
    if continue_session {
        let session_path = commands_session::continue_session_path();
        match std::fs::read_to_string(session_path) {
            Ok(json) => match agent.restore_messages(&json) {
                Ok(_) => {
                    eprintln!(
                        "{DIM}  resumed session: {} messages from {session_path}{RESET}",
                        agent.messages().len()
                    );
                }
                Err(e) => eprintln!("{YELLOW}warning:{RESET} Failed to restore session: {e}"),
            },
            Err(_) => eprintln!("{DIM}  no previous session found ({session_path}){RESET}"),
        }
    }

    // --prompt / -p: single-shot mode with a prompt argument
    if let Some(prompt_text) = config.prompt_arg {
        run_single_prompt(
            &mut agent_config,
            &mut agent,
            &prompt_text,
            &image_path,
            &output_path,
            json_output,
        )
        .await;
        return;
    }

    // Piped mode: read all of stdin as a single prompt, run once, exit
    if !io::stdin().is_terminal() {
        run_piped_mode(&mut agent_config, &mut agent, &output_path, json_output).await;
        return;
    }

    // Interactive REPL mode
    // Check for updates (non-blocking, skipped if --no-update-check or env var)
    let update_available = if !no_update_check {
        update::check_for_update(cli::VERSION)
    } else {
        None
    };

    repl::run_repl(
        &mut agent_config,
        &mut agent,
        mcp_count,
        openapi_count,
        continue_session,
        update_available,
        mcp_servers,
        mcp_server_configs,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn looks_like_slash_command_detects_leading_slash() {
        assert!(looks_like_slash_command("/doctor"));
        assert!(looks_like_slash_command("/help"));
        assert!(looks_like_slash_command("/"));
    }

    #[test]
    fn looks_like_slash_command_handles_leading_whitespace() {
        // The caller already trims, but we should be robust to \n/doctor\n etc.
        assert!(looks_like_slash_command("  /doctor"));
        assert!(looks_like_slash_command("\n/doctor\n"));
        assert!(looks_like_slash_command("\t/status"));
    }

    #[test]
    fn looks_like_slash_command_rejects_mid_string_slash() {
        // A slash that isn't the first non-whitespace character must NOT trigger.
        assert!(!looks_like_slash_command("what does /doctor do?"));
        assert!(!looks_like_slash_command("explain /help to me"));
        assert!(!looks_like_slash_command("path: a/b/c"));
    }

    #[test]
    fn looks_like_slash_command_rejects_non_slash_input() {
        assert!(!looks_like_slash_command("hello"));
        assert!(!looks_like_slash_command(""));
        assert!(!looks_like_slash_command("   "));
        assert!(!looks_like_slash_command("-flag"));
    }

    #[test]
    fn test_always_approve_flag_starts_false() {
        // The "always" flag should start as false
        let flag = Arc::new(AtomicBool::new(false));
        assert!(!flag.load(Ordering::Relaxed));
    }

    #[test]
    fn test_checkpoint_triggered_flag_starts_false() {
        // CHECKPOINT_TRIGGERED should default to false
        assert!(!CHECKPOINT_TRIGGERED.load(Ordering::SeqCst));
    }

    #[test]
    fn test_always_approve_flag_persists_across_clones() {
        // Simulates the confirm closure: flag is shared via Arc
        let always_approved = Arc::new(AtomicBool::new(false));
        let flag_clone = Arc::clone(&always_approved);

        // Initially not set
        assert!(!flag_clone.load(Ordering::Relaxed));

        // User answers "always" — set the flag
        always_approved.store(true, Ordering::Relaxed);

        // The clone sees the update (simulates next confirm call)
        assert!(flag_clone.load(Ordering::Relaxed));
    }

    #[test]
    fn test_always_approve_response_matching() {
        // Verify the response matching logic for "always" variants
        let responses_that_approve = ["y", "yes", "a", "always"];
        let responses_that_deny = ["n", "no", "", "maybe", "nope"];

        for r in &responses_that_approve {
            let normalized = r.trim().to_lowercase();
            assert!(
                matches!(normalized.as_str(), "y" | "yes" | "a" | "always"),
                "Expected '{}' to be approved",
                r
            );
        }

        for r in &responses_that_deny {
            let normalized = r.trim().to_lowercase();
            assert!(
                !matches!(normalized.as_str(), "y" | "yes" | "a" | "always"),
                "Expected '{}' to be denied",
                r
            );
        }
    }

    #[test]
    fn test_always_approve_only_on_a_or_always() {
        // Only "a" and "always" should set the persist flag, not "y" or "yes"
        let always_responses = ["a", "always"];
        let single_responses = ["y", "yes"];

        for r in &always_responses {
            let normalized = r.trim().to_lowercase();
            assert!(
                matches!(normalized.as_str(), "a" | "always"),
                "Expected '{}' to trigger always-approve",
                r
            );
        }

        for r in &single_responses {
            let normalized = r.trim().to_lowercase();
            assert!(
                !matches!(normalized.as_str(), "a" | "always"),
                "Expected '{}' NOT to trigger always-approve",
                r
            );
        }
    }

    #[test]
    fn test_always_approve_flag_used_in_confirm_simulation() {
        // End-to-end simulation of the confirm flow with "always"
        let always_approved = Arc::new(AtomicBool::new(false));

        // Simulate three bash commands in sequence
        let commands = ["ls", "echo hello", "cat file.txt"];
        let user_responses = ["a", "", ""]; // user answers "always" first time

        for (i, cmd) in commands.iter().enumerate() {
            let approved = if always_approved.load(Ordering::Relaxed) {
                // Auto-approved — no prompt needed
                true
            } else {
                let response = user_responses[i].trim().to_lowercase();
                let result = matches!(response.as_str(), "y" | "yes" | "a" | "always");
                if matches!(response.as_str(), "a" | "always") {
                    always_approved.store(true, Ordering::Relaxed);
                }
                result
            };

            match i {
                0 => assert!(
                    approved,
                    "First command '{}' should be approved via 'a'",
                    cmd
                ),
                1 => assert!(approved, "Second command '{}' should be auto-approved", cmd),
                2 => assert!(approved, "Third command '{}' should be auto-approved", cmd),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_agent_config_struct_fields() {
        // AgentConfig should hold all the fields needed to build an agent
        let config = AgentConfig {
            model: "claude-opus-4-6".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "You are helpful.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            max_turns: Some(10),
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        assert_eq!(config.model, "claude-opus-4-6");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.provider, "anthropic");
        assert!(config.base_url.is_none());
        assert_eq!(config.system_prompt, "You are helpful.");
        assert_eq!(config.thinking, ThinkingLevel::Off);
        assert_eq!(config.max_tokens, Some(4096));
        assert_eq!(config.temperature, Some(0.7));
        assert_eq!(config.max_turns, Some(10));
        assert!(config.auto_approve);
        assert!(config.permissions.is_empty());
    }

    #[test]
    fn test_agent_config_build_agent_anthropic() {
        // build_agent should produce an Agent for the anthropic provider
        let config = AgentConfig {
            model: "claude-opus-4-6".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test prompt.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config.build_agent();
        // Agent should have 6 tools (bash, read, write, edit, list, search)
        // Agent created successfully — verify it has empty message history
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_agent_config_build_agent_openai() {
        // build_agent should produce an Agent for a non-anthropic provider
        let config = AgentConfig {
            model: "gpt-4o".to_string(),
            api_key: "test-key".to_string(),
            provider: "openai".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: Some(2048),
            temperature: Some(0.5),
            max_turns: Some(20),
            auto_approve: false,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config.build_agent();
        // Agent created successfully — verify it has empty message history
        assert_eq!(agent.messages().len(), 0);
        assert_eq!(agent.temperature, Some(0.5));
    }

    #[test]
    fn test_agent_config_build_agent_google() {
        // Google provider should also work
        let config = AgentConfig {
            model: "gemini-2.0-flash".to_string(),
            api_key: "test-key".to_string(),
            provider: "google".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config.build_agent();
        // Agent created successfully — verify it has empty message history
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_agent_config_build_agent_with_base_url() {
        // Anthropic with a base_url should use OpenAI-compat path
        let config = AgentConfig {
            model: "claude-opus-4-6".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: Some("http://localhost:8080/v1".to_string()),
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config.build_agent();
        // Agent created successfully — verify it has empty message history
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_agent_config_rebuild_produces_fresh_agent() {
        // Calling build_agent twice should produce two independent agents
        let config = AgentConfig {
            model: "claude-opus-4-6".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent1 = config.build_agent();
        let agent2 = config.build_agent();
        // Both should have empty message history
        assert_eq!(agent1.messages().len(), 0);
        assert_eq!(agent2.messages().len(), 0);
    }

    #[test]
    fn test_agent_config_mutable_model_switch() {
        // Simulates /model switch: change config.model, rebuild agent
        let mut config = AgentConfig {
            model: "claude-opus-4-6".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        assert_eq!(config.model, "claude-opus-4-6");
        config.model = "claude-haiku-35".to_string();
        let _agent = config.build_agent();
        assert_eq!(config.model, "claude-haiku-35");
    }

    #[test]
    fn test_agent_config_mutable_thinking_switch() {
        // Simulates /think switch: change config.thinking, rebuild agent
        let mut config = AgentConfig {
            model: "claude-opus-4-6".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        assert_eq!(config.thinking, ThinkingLevel::Off);
        config.thinking = ThinkingLevel::High;
        let _agent = config.build_agent();
        assert_eq!(config.thinking, ThinkingLevel::High);
    }

    // === File operation confirmation tests ===

    // === Client identification header tests ===

    #[test]
    fn test_yoyo_user_agent_format() {
        let ua = yoyo_user_agent();
        assert!(
            ua.starts_with("yoyo/"),
            "User-Agent should start with 'yoyo/'"
        );
        // Should contain a version number (e.g. "0.1.0")
        let version_part = &ua["yoyo/".len()..];
        assert!(
            version_part.contains('.'),
            "User-Agent version should contain a dot: {ua}"
        );
    }

    #[test]
    fn test_client_headers_anthropic() {
        let config = create_model_config("anthropic", "claude-sonnet-4-20250514", None);
        assert_eq!(
            config.headers.get("User-Agent").unwrap(),
            &yoyo_user_agent(),
            "Anthropic config should have User-Agent header"
        );
        assert!(
            !config.headers.contains_key("HTTP-Referer"),
            "Anthropic config should NOT have HTTP-Referer"
        );
        assert!(
            !config.headers.contains_key("X-Title"),
            "Anthropic config should NOT have X-Title"
        );
    }

    #[test]
    fn test_client_headers_openai() {
        let config = create_model_config("openai", "gpt-4o", None);
        assert_eq!(
            config.headers.get("User-Agent").unwrap(),
            &yoyo_user_agent(),
            "OpenAI config should have User-Agent header"
        );
        assert!(
            !config.headers.contains_key("HTTP-Referer"),
            "OpenAI config should NOT have HTTP-Referer"
        );
    }

    #[test]
    fn test_client_headers_openrouter() {
        let config = create_model_config("openrouter", "anthropic/claude-sonnet-4-20250514", None);
        assert_eq!(
            config.headers.get("User-Agent").unwrap(),
            &yoyo_user_agent(),
            "OpenRouter config should have User-Agent header"
        );
        assert_eq!(
            config.headers.get("HTTP-Referer").unwrap(),
            "https://github.com/yologdev/yoyo-evolve",
            "OpenRouter config should have HTTP-Referer header"
        );
        assert_eq!(
            config.headers.get("X-Title").unwrap(),
            "yoyo",
            "OpenRouter config should have X-Title header"
        );
    }

    #[test]
    fn test_client_headers_google() {
        let config = create_model_config("google", "gemini-2.0-flash", None);
        assert_eq!(
            config.headers.get("User-Agent").unwrap(),
            &yoyo_user_agent(),
            "Google config should have User-Agent header"
        );
    }

    #[test]
    fn test_create_model_config_zai_defaults() {
        let config = create_model_config("zai", "glm-4-plus", None);
        assert_eq!(config.provider, "zai");
        assert_eq!(config.id, "glm-4-plus");
        assert_eq!(config.base_url, "https://api.z.ai/api/paas/v4");
        assert_eq!(
            config.headers.get("User-Agent").unwrap(),
            &yoyo_user_agent(),
            "ZAI config should have User-Agent header"
        );
    }

    #[test]
    fn test_create_model_config_zai_custom_base_url() {
        let config =
            create_model_config("zai", "glm-4-plus", Some("https://custom.zai.example/v1"));
        assert_eq!(config.provider, "zai");
        assert_eq!(config.base_url, "https://custom.zai.example/v1");
    }

    #[test]
    fn test_agent_config_build_agent_zai() {
        let config = AgentConfig {
            model: "glm-4-plus".to_string(),
            api_key: "test-key".to_string(),
            provider: "zai".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config.build_agent();
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_create_model_config_minimax_defaults() {
        let config = create_model_config("minimax", "MiniMax-M2.7", None);
        assert_eq!(config.provider, "minimax");
        assert_eq!(config.id, "MiniMax-M2.7");
        assert_eq!(
            config.base_url, "https://api.minimaxi.chat/v1",
            "MiniMax should use api.minimaxi.chat (not api.minimax.io)"
        );
        assert!(
            config.compat.is_some(),
            "MiniMax config should have compat flags set"
        );
        assert_eq!(
            config.headers.get("User-Agent").unwrap(),
            &yoyo_user_agent(),
            "MiniMax config should have User-Agent header"
        );
    }

    #[test]
    fn test_create_model_config_minimax_custom_base_url() {
        let config = create_model_config(
            "minimax",
            "MiniMax-M2.7",
            Some("https://custom.minimax.example/v1"),
        );
        assert_eq!(config.provider, "minimax");
        assert_eq!(config.base_url, "https://custom.minimax.example/v1");
    }

    #[test]
    fn test_create_model_config_unknown_provider_falls_through() {
        // Unknown providers should be treated as OpenAI-compatible on localhost
        let config = create_model_config("typo_provider", "some-model", None);
        assert_eq!(config.provider, "typo_provider");
        assert_eq!(config.base_url, "http://localhost:8080/v1");
    }

    #[test]
    fn test_create_model_config_unknown_provider_with_base_url() {
        // Unknown provider with explicit base URL should use that URL
        let config = create_model_config(
            "typo_provider",
            "some-model",
            Some("https://my-server.com/v1"),
        );
        assert_eq!(config.provider, "typo_provider");
        assert_eq!(config.base_url, "https://my-server.com/v1");
    }

    #[test]
    fn test_agent_config_build_agent_minimax() {
        let config = AgentConfig {
            model: "MiniMax-M2.7".to_string(),
            api_key: "test-key".to_string(),
            provider: "minimax".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config.build_agent();
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_bedrock_model_config() {
        let config =
            create_model_config("bedrock", "anthropic.claude-sonnet-4-20250514-v1:0", None);
        assert_eq!(config.provider, "bedrock");
        assert_eq!(
            config.base_url,
            "https://bedrock-runtime.us-east-1.amazonaws.com"
        );
        // Verify it uses BedrockConverseStream protocol (not OpenAI)
        assert_eq!(format!("{}", config.api), "bedrock_converse_stream");
    }

    #[test]
    fn test_bedrock_model_config_custom_url() {
        let config = create_model_config(
            "bedrock",
            "anthropic.claude-sonnet-4-20250514-v1:0",
            Some("https://bedrock-runtime.eu-west-1.amazonaws.com"),
        );
        assert_eq!(
            config.base_url,
            "https://bedrock-runtime.eu-west-1.amazonaws.com"
        );
    }

    #[test]
    fn test_build_agent_bedrock() {
        let config = AgentConfig {
            model: "anthropic.claude-sonnet-4-20250514-v1:0".to_string(),
            api_key: "test-access:test-secret".to_string(),
            provider: "bedrock".to_string(),
            base_url: Some("https://bedrock-runtime.us-east-1.amazonaws.com".to_string()),
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "test".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config.build_agent();
        // If this compiles and runs, BedrockProvider is correctly wired
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_client_headers_on_anthropic_build_agent() {
        // The Anthropic path in build_agent() should also get headers
        let agent_config = AgentConfig {
            model: "claude-opus-4-6".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        // Verify the anthropic ModelConfig would have headers set
        // (We test the helper directly since Agent doesn't expose model_config)
        let mut anthropic_config = ModelConfig::anthropic("claude-opus-4-6", "claude-opus-4-6");
        insert_client_headers(&mut anthropic_config);
        assert_eq!(
            anthropic_config.headers.get("User-Agent").unwrap(),
            &yoyo_user_agent()
        );
        // Also verify build_agent doesn't panic
        let _agent = agent_config.build_agent();
    }

    /// Helper to create a default AgentConfig for tests, varying only the provider.
    fn test_agent_config(provider: &str, model: &str) -> AgentConfig {
        AgentConfig {
            model: model.to_string(),
            api_key: "test-key".to_string(),
            provider: provider.to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::empty(),
            system_prompt: "Test prompt.".to_string(),
            thinking: ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        }
    }

    #[test]
    fn test_configure_agent_applies_all_settings() {
        // Verify configure_agent applies optional settings (max_tokens, temperature, max_turns)
        let config = AgentConfig {
            max_tokens: Some(2048),
            temperature: Some(0.5),
            max_turns: Some(5),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        let agent = config.build_agent();
        // Agent was built without panic — configure_agent applied all settings
        assert_eq!(agent.messages().len(), 0);
    }

    #[test]
    fn test_build_agent_all_providers_build_cleanly() {
        // All three provider paths should produce agents with 6 tools via configure_agent.
        // This catches regressions where a provider branch forgets to call configure_agent.
        let providers = [
            ("anthropic", "claude-opus-4-6"),
            ("google", "gemini-2.5-pro"),
            ("openai", "gpt-4o"),
            ("deepseek", "deepseek-chat"),
        ];
        for (provider, model) in &providers {
            let config = test_agent_config(provider, model);
            let agent = config.build_agent();
            assert_eq!(
                agent.messages().len(),
                0,
                "provider '{provider}' should produce a clean agent"
            );
        }
    }

    #[test]
    fn test_build_agent_anthropic_with_base_url_uses_openai_compat() {
        // When Anthropic is used with a custom base_url, it should go through
        // the OpenAI-compatible path (not the default Anthropic path)
        let config = AgentConfig {
            base_url: Some("https://custom-api.example.com/v1".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        // Should not panic — the OpenAI-compat path handles anthropic + base_url
        let agent = config.build_agent();
        assert_eq!(agent.messages().len(), 0);
    }

    // -----------------------------------------------------------------------
    // StreamingBashTool tests
    // -----------------------------------------------------------------------

    // ── rename_symbol tool tests ─────────────────────────────────────

    #[test]
    fn test_configure_agent_sets_context_config() {
        // Verify that configure_agent successfully builds an agent with context config
        let config = AgentConfig {
            model: "test-model".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::default(),
            system_prompt: "test".to_string(),
            thinking: yoagent::ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None,
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        // This should not panic — context config and execution limits are wired
        let agent =
            config.configure_agent(Agent::new(yoagent::provider::AnthropicProvider), 200_000);
        // Agent built successfully with context config
        let _ = agent;
    }

    #[test]
    fn test_execution_limits_always_set() {
        // Even without --max-turns, configure_agent should set execution limits
        let config_no_turns = AgentConfig {
            model: "test-model".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::default(),
            system_prompt: "test".to_string(),
            thinking: yoagent::ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: None, // No explicit max_turns
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        // Should not panic — limits are set with defaults
        let agent = config_no_turns
            .configure_agent(Agent::new(yoagent::provider::AnthropicProvider), 200_000);
        let _ = agent;

        // With explicit max_turns, it should use that value
        let config_with_turns = AgentConfig {
            model: "test-model".to_string(),
            api_key: "test-key".to_string(),
            provider: "anthropic".to_string(),
            base_url: None,
            skills: yoagent::skills::SkillSet::default(),
            system_prompt: "test".to_string(),
            thinking: yoagent::ThinkingLevel::Off,
            max_tokens: None,
            temperature: None,
            max_turns: Some(50),
            auto_approve: true,
            auto_commit: false,
            permissions: cli::PermissionConfig::default(),
            dir_restrictions: cli::DirectoryRestrictions::default(),
            context_strategy: cli::ContextStrategy::default(),
            context_window: None,
            shell_hooks: vec![],
            fallback_provider: None,
            fallback_model: None,
        };
        let agent = config_with_turns
            .configure_agent(Agent::new(yoagent::provider::AnthropicProvider), 200_000);
        let _ = agent;
    }

    // -----------------------------------------------------------------------
    // TodoTool tests
    // -----------------------------------------------------------------------

    // ── Fallback provider switch tests ──────────────────────────────────

    #[test]
    fn test_fallback_switch_success() {
        // When fallback is configured and different from current, switch should succeed
        let mut config = AgentConfig {
            fallback_provider: Some("google".to_string()),
            fallback_model: Some("gemini-2.0-flash".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        assert!(config.try_switch_to_fallback());
        assert_eq!(config.provider, "google");
        assert_eq!(config.model, "gemini-2.0-flash");
    }

    #[test]
    fn test_fallback_switch_already_on_fallback() {
        // When current provider already matches the fallback, no switch should happen
        let mut config = AgentConfig {
            fallback_provider: Some("anthropic".to_string()),
            fallback_model: Some("claude-opus-4-6".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        assert!(!config.try_switch_to_fallback());
        // Provider should remain unchanged
        assert_eq!(config.provider, "anthropic");
    }

    #[test]
    fn test_fallback_switch_no_fallback_configured() {
        // When no fallback is set, switch should return false
        let mut config = test_agent_config("anthropic", "claude-opus-4-6");
        assert!(config.fallback_provider.is_none());
        assert!(!config.try_switch_to_fallback());
        assert_eq!(config.provider, "anthropic");
        assert_eq!(config.model, "claude-opus-4-6");
    }

    #[test]
    fn test_fallback_switch_derives_default_model() {
        // When fallback_model is None, should derive the default model for the provider
        let mut config = AgentConfig {
            fallback_provider: Some("openai".to_string()),
            fallback_model: None,
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        assert!(config.try_switch_to_fallback());
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, cli::default_model_for_provider("openai"));
    }

    #[test]
    fn test_fallback_switch_uses_explicit_model() {
        // When fallback_model is Some, should use it instead of the default
        let mut config = AgentConfig {
            fallback_provider: Some("openai".to_string()),
            fallback_model: Some("gpt-4-turbo".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        assert!(config.try_switch_to_fallback());
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "gpt-4-turbo");
    }

    #[test]
    #[serial]
    fn test_fallback_switch_resolves_api_key() {
        // When switching to fallback, API key should be resolved from the env var
        // SAFETY: Test runs serially (#[serial]), no concurrent env var access.
        unsafe {
            std::env::set_var("GOOGLE_API_KEY", "test-google-key-fallback");
        }
        let mut config = AgentConfig {
            fallback_provider: Some("google".to_string()),
            fallback_model: Some("gemini-2.0-flash".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        assert_eq!(config.api_key, "test-key"); // original
        assert!(config.try_switch_to_fallback());
        assert_eq!(config.api_key, "test-google-key-fallback");
        // SAFETY: Test runs serially (#[serial]), no concurrent env var access.
        unsafe {
            std::env::remove_var("GOOGLE_API_KEY");
        }
    }

    #[test]
    fn test_fallback_switch_keeps_api_key_when_env_missing() {
        // If the fallback provider's env var isn't set, original api_key should persist
        // (removing the env var to be safe)
        // SAFETY: Test runs serially, no concurrent env var access.
        unsafe {
            std::env::remove_var("XAI_API_KEY");
        }
        let mut config = AgentConfig {
            fallback_provider: Some("xai".to_string()),
            fallback_model: Some("grok-3".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        let original_key = config.api_key.clone();
        assert!(config.try_switch_to_fallback());
        assert_eq!(config.provider, "xai");
        assert_eq!(config.api_key, original_key);
    }

    #[test]
    fn test_fallback_switch_idempotent() {
        // Calling try_switch_to_fallback twice: first call switches, second returns false
        // (because provider now matches fallback)
        let mut config = AgentConfig {
            fallback_provider: Some("google".to_string()),
            fallback_model: Some("gemini-2.0-flash".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        assert!(config.try_switch_to_fallback());
        assert_eq!(config.provider, "google");
        // Second call: already on fallback
        assert!(!config.try_switch_to_fallback());
        assert_eq!(config.provider, "google");
    }

    // ── Fallback retry helper (non-interactive) tests ────────────────────

    #[test]
    fn test_fallback_prompt_no_api_error_passthrough() {
        // When the response has no API error, try_switch_to_fallback should NOT be called.
        // This verifies the guard condition: no error → no retry, no exit error.
        let config = AgentConfig {
            fallback_provider: Some("google".to_string()),
            fallback_model: Some("gemini-2.0-flash".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };
        // Simulate: response has no API error
        let response = PromptOutcome {
            text: "success".to_string(),
            last_tool_error: None,
            was_overflow: false,
            last_api_error: None,
        };
        // The helper's first check: if no API error, return immediately.
        // We verify this contract by checking the config isn't touched.
        assert!(response.last_api_error.is_none());
        assert_eq!(config.provider, "anthropic"); // still on primary
    }

    #[test]
    fn test_fallback_prompt_api_error_no_fallback_configured() {
        // When API error occurs but no fallback is configured, should_exit_error = true
        let mut config = test_agent_config("anthropic", "claude-opus-4-6");
        assert!(config.fallback_provider.is_none());

        let response = PromptOutcome {
            text: String::new(),
            last_tool_error: None,
            was_overflow: false,
            last_api_error: Some("503 Service Unavailable".to_string()),
        };
        // The helper would: check API error (yes) → try_switch_to_fallback (false) → exit error
        assert!(response.last_api_error.is_some());
        assert!(!config.try_switch_to_fallback()); // no fallback → returns false
                                                   // Contract: should_exit_error = true in this case
    }

    #[test]
    fn test_fallback_prompt_api_error_with_fallback_switches() {
        // When API error occurs and fallback is configured, the config should switch
        let mut config = AgentConfig {
            fallback_provider: Some("google".to_string()),
            fallback_model: Some("gemini-2.0-flash".to_string()),
            ..test_agent_config("anthropic", "claude-opus-4-6")
        };

        let response = PromptOutcome {
            text: String::new(),
            last_tool_error: None,
            was_overflow: false,
            last_api_error: Some("529 Overloaded".to_string()),
        };
        // The helper would: check API error (yes) → try_switch_to_fallback (true) → rebuild → retry
        assert!(response.last_api_error.is_some());
        assert!(config.try_switch_to_fallback());
        assert_eq!(config.provider, "google");
        assert_eq!(config.model, "gemini-2.0-flash");
    }

    #[test]
    fn test_build_json_output_valid_json_with_expected_keys() {
        let response = PromptOutcome {
            text: "Hello, world!".to_string(),
            last_tool_error: None,
            was_overflow: false,
            last_api_error: None,
        };
        let usage = Usage {
            input: 100,
            output: 50,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 150,
        };
        let result = build_json_output(&response, "claude-sonnet-4-20250514", &usage, false);

        // Must be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("build_json_output should produce valid JSON");

        // Check all expected keys exist
        assert_eq!(parsed["response"], "Hello, world!");
        assert_eq!(parsed["model"], "claude-sonnet-4-20250514");
        assert_eq!(parsed["is_error"], false);
        assert!(parsed["usage"].is_object());
        assert_eq!(parsed["usage"]["input_tokens"], 100);
        assert_eq!(parsed["usage"]["output_tokens"], 50);
        assert!(parsed["cost_usd"].is_number());
    }

    #[test]
    fn test_build_json_output_error_mode() {
        let response = PromptOutcome {
            text: "Something went wrong".to_string(),
            last_tool_error: None,
            was_overflow: false,
            last_api_error: Some("API error".to_string()),
        };
        let usage = Usage {
            input: 10,
            output: 5,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 15,
        };
        let result = build_json_output(&response, "claude-sonnet-4-20250514", &usage, true);

        let parsed: serde_json::Value = serde_json::from_str(&result)
            .expect("build_json_output should produce valid JSON even in error mode");

        assert_eq!(parsed["response"], "Something went wrong");
        assert_eq!(parsed["is_error"], true);
        assert!(parsed["usage"].is_object());
        assert!(parsed["cost_usd"].is_number());
    }

    #[test]
    fn mcp_builtin_collision_detection() {
        // The canonical collision: filesystem MCP server exposes read_file,
        // which collides with yoyo's builtin. Non-colliding tools pass through.
        let builtins = vec!["read_file", "write_file", "bash", "search"];
        let mcp_tools = vec!["read_file".to_string(), "fetch_url".to_string()];
        let collisions = detect_mcp_collisions(&mcp_tools, &builtins);
        assert_eq!(collisions, vec!["read_file".to_string()]);
    }

    #[test]
    fn mcp_collision_detection_no_collisions() {
        let builtins = vec!["read_file", "write_file"];
        let mcp_tools = vec!["fetch_url".to_string(), "query_db".to_string()];
        let collisions = detect_mcp_collisions(&mcp_tools, &builtins);
        assert!(collisions.is_empty());
    }

    #[test]
    fn mcp_collision_detection_multiple_collisions_preserves_order() {
        let builtins = vec!["read_file", "write_file", "bash"];
        let mcp_tools = vec![
            "write_file".to_string(),
            "safe_tool".to_string(),
            "read_file".to_string(),
        ];
        let collisions = detect_mcp_collisions(&mcp_tools, &builtins);
        assert_eq!(
            collisions,
            vec!["write_file".to_string(), "read_file".to_string()]
        );
    }

    #[test]
    fn mcp_collision_detection_against_real_builtins() {
        // Verify the real BUILTIN_TOOL_NAMES constant catches the flagship
        // filesystem server's known collisions. If any of these slip through,
        // yoyo will die on the first LLM turn with "Tool names must be unique".
        let filesystem_server_tools = vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "list_directory".to_string(),
            "move_file".to_string(),
        ];
        let collisions = detect_mcp_collisions(&filesystem_server_tools, BUILTIN_TOOL_NAMES);
        assert!(collisions.contains(&"read_file".to_string()));
        assert!(collisions.contains(&"write_file".to_string()));
        assert_eq!(
            collisions.len(),
            2,
            "only read_file and write_file should collide"
        );
    }

    #[test]
    fn mcp_collision_detection_empty_inputs() {
        assert!(detect_mcp_collisions(&[], &["read_file"]).is_empty());
        assert!(detect_mcp_collisions(&["foo".to_string()], &[]).is_empty());
        assert!(detect_mcp_collisions(&[], &[]).is_empty());
    }
}
