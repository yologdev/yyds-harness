//! CLI constants, configuration types, and session defaults.
//!
//! Extracted from `cli.rs` to separate configuration data from argument parsing
//! and display logic. Everything here is re-exported by `cli.rs` so downstream
//! `use crate::cli::*` imports continue to work unchanged.

use crate::config::{DirectoryRestrictions, McpServerConfig, PermissionConfig};
use yoagent::skills::SkillSet;
use yoagent::ThinkingLevel;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_CONTEXT_TOKENS: u64 = 200_000;
pub const AUTO_COMPACT_THRESHOLD: f64 = 0.80;
pub const PROACTIVE_COMPACT_THRESHOLD: f64 = 0.70;

/// Effective context window (tokens) for the current session.
/// Set once in configure_agent() based on model config + CLI override.
/// Read by /tokens and /status commands to show accurate budget.
static EFFECTIVE_CONTEXT_TOKENS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(DEFAULT_CONTEXT_TOKENS);

/// Set the effective context window size. Called once during agent setup.
pub fn set_effective_context_tokens(tokens: u64) {
    EFFECTIVE_CONTEXT_TOKENS.store(tokens, std::sync::atomic::Ordering::SeqCst);
}

/// Get the effective context window size for display purposes.
pub fn effective_context_tokens() -> u64 {
    EFFECTIVE_CONTEXT_TOKENS.load(std::sync::atomic::Ordering::SeqCst)
}

pub const DEFAULT_SESSION_PATH: &str = "yoyo-session.json";
pub const AUTO_SAVE_SESSION_PATH: &str = ".yoyo/last-session.json";
pub const SYSTEM_PROMPT_VERSION: &str = "coding_system_prompt@v3";
pub const LITE_SYSTEM_PROMPT_VERSION: &str = "lite_system_prompt@v1";

pub const SYSTEM_PROMPT: &str = r#"# Role

You are a coding assistant working in the user's terminal. You have access to
the filesystem and shell. Be direct, concise, and action-oriented. When the user
asks you to do something, do it instead of only explaining how.

# Evidence First

Ground conclusions in observable evidence: files, command output, git status,
git diff, tests, logs, and available state. Do not claim something is fixed,
implemented, or verified unless the artifact or command result supports it.

# Bounded Context Use

Search before reading. Use search and file listings to locate relevant code, then
read targeted sections. Avoid reading whole large files unless necessary. Prefer
specific functions, line ranges, and focused commands over broad scans.

Prefer repository-aware search (`rg`, project search tools, or exact file paths)
over raw recursive `grep`. Exclude generated/binary paths such as `target/`,
`.git/`, `node_modules/`, and large state artifacts. Escape regex metacharacters
or use literal/fixed-string search when searching for snippets that contain
parentheses, brackets, quotes, or backslashes.

# Work Integrity

Never fake completion. If work is incomplete, blocked, unverified, or only
partially done, say that plainly. Preserve user work: inspect git status/diff,
avoid unrelated edits, and do not overwrite or revert changes you did not make.
Confirm destructive operations before deleting files, resetting git state, or
running irreversible commands.

# Change Discipline

Keep edits narrow and aligned with existing project patterns. For multi-file
changes, plan the approach first, edit incrementally, and verify between steps
when useful. Avoid unrelated refactors.

# Verification

After changes, run focused build, test, lint, or smoke-check commands when
practical. If verification cannot be run, explain exactly why and what risk
remains.

Keep verification bounded. Start with the narrowest relevant test or check, then
broaden only when needed. If a command is likely to be long-running, state what it
proves and avoid repeating it without new evidence.

# Tool Use

Use tools proactively but efficiently. Prefer precise commands and structured
inspection. When a command fails or an edit does not apply, read the actual
error/output and current file content before retrying.

# Communication

Keep progress updates short and concrete. In final responses, summarize what
changed, where it changed, and what verification was run or skipped."#;

/// Minimal system prompt for --lite mode (small/local LLMs with limited context).
pub const LITE_SYSTEM_PROMPT: &str = "You are a coding assistant. Help the user with their code.\nYou have tools: bash (run commands), read_file, write_file, edit_file (find and replace text in files).\nAfter making changes, verify with the project's build or test commands when practical.";

/// The 4 essential tools available in --lite mode.
pub const LITE_TOOLS: &[&str] = &["bash", "read_file", "write_file", "edit_file"];

/// Default context window for --lite mode (suitable for 4B-8B parameter models).
pub const LITE_DEFAULT_CONTEXT_WINDOW: u32 = 8_000;

/// Context management strategy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ContextStrategy {
    /// Default: auto-compact conversation when approaching context limit
    #[default]
    Compaction,
    /// Write checkpoint file and exit with code 2 when approaching limit
    Checkpoint,
}

/// Output format for non-interactive modes (--prompt, piped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Default human-readable text output.
    Text,
    /// Single JSON blob at the end (--json / --output-format json).
    Json,
    /// Newline-delimited JSON events streamed in real-time (--output-format stream-json).
    StreamJson,
}

/// Parsed CLI configuration.
pub struct Config {
    pub model: String,
    pub api_key: String,
    pub provider: String,
    pub base_url: Option<String>,
    pub skills: SkillSet,
    pub system_prompt: String,
    pub system_prompt_version: String,
    pub thinking: ThinkingLevel,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub max_turns: Option<usize>,
    pub continue_session: bool,
    pub output_path: Option<String>,
    pub prompt_arg: Option<String>,
    pub image_path: Option<String>,
    pub verbose: bool,
    pub mcp_servers: Vec<String>,
    pub mcp_server_configs: Vec<McpServerConfig>,
    pub openapi_specs: Vec<String>,
    pub auto_approve: bool,
    pub auto_commit: bool,
    pub permissions: PermissionConfig,
    pub dir_restrictions: DirectoryRestrictions,
    pub context_strategy: ContextStrategy,
    pub context_window: Option<u32>,
    pub shell_hooks: Vec<crate::hooks::ShellHook>,
    pub fallback_provider: Option<String>,
    pub fallback_model: Option<String>,
    pub no_update_check: bool,
    pub json_output: bool,
    pub output_format: OutputFormat,
    pub audit: bool,
    pub print_system_prompt: bool,
    pub print_mode: bool,
    pub auto_watch: bool,
    pub allowed_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub no_tools: bool,
    pub deepseek_native: bool,
    pub deepseek_fim_route: bool,
    pub deepseek_fim_response: Option<String>,
    pub state: crate::state::StateConfig,
    pub lite: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_config_constants() {
        // VERSION is set at compile time — just verify it's non-empty
        assert!(!VERSION.is_empty());
        assert_eq!(DEFAULT_CONTEXT_TOKENS, 200_000);
        assert!((AUTO_COMPACT_THRESHOLD - 0.80).abs() < f64::EPSILON);
        assert!((PROACTIVE_COMPACT_THRESHOLD - 0.70).abs() < f64::EPSILON);
        assert_eq!(DEFAULT_SESSION_PATH, "yoyo-session.json");
        assert_eq!(AUTO_SAVE_SESSION_PATH, ".yoyo/last-session.json");
        assert_eq!(SYSTEM_PROMPT_VERSION, "coding_system_prompt@v3");
        assert_eq!(LITE_SYSTEM_PROMPT_VERSION, "lite_system_prompt@v1");
        assert!(SYSTEM_PROMPT.contains("coding assistant"));
        assert!(SYSTEM_PROMPT.contains("Evidence First"));
        assert!(SYSTEM_PROMPT.contains("available state"));
        assert!(SYSTEM_PROMPT.contains("Bounded Context Use"));
        assert!(SYSTEM_PROMPT.contains("repository-aware search"));
        assert!(SYSTEM_PROMPT.contains("target/"));
        assert!(SYSTEM_PROMPT.contains("literal/fixed-string search"));
        assert!(SYSTEM_PROMPT.contains("Never fake completion"));
        assert!(SYSTEM_PROMPT.contains("Preserve user work"));
        assert!(SYSTEM_PROMPT.contains("Confirm destructive operations"));
        assert!(SYSTEM_PROMPT.contains("Verification"));
        assert!(SYSTEM_PROMPT.contains("Keep verification bounded"));
    }

    #[test]
    fn test_effective_context_tokens_roundtrip() {
        // Save the original value to restore later (tests run concurrently)
        let original = effective_context_tokens();
        set_effective_context_tokens(128_000);
        assert_eq!(effective_context_tokens(), 128_000);
        // Restore
        set_effective_context_tokens(original);
    }

    #[test]
    fn test_context_strategy_default() {
        let strategy = ContextStrategy::default();
        assert_eq!(strategy, ContextStrategy::Compaction);
    }

    #[test]
    fn test_output_format_equality() {
        assert_eq!(OutputFormat::Text, OutputFormat::Text);
        assert_ne!(OutputFormat::Text, OutputFormat::Json);
        assert_ne!(OutputFormat::Json, OutputFormat::StreamJson);
    }

    #[test]
    fn test_lite_constants() {
        // LITE_SYSTEM_PROMPT should be minimal — much shorter than the default
        assert!(LITE_SYSTEM_PROMPT.contains("coding assistant"));
        assert!(LITE_SYSTEM_PROMPT.contains("verify"));
        assert!(LITE_SYSTEM_PROMPT.len() < SYSTEM_PROMPT.len());

        // LITE_TOOLS should have exactly the 4 essential tools
        assert_eq!(LITE_TOOLS.len(), 4);
        assert!(LITE_TOOLS.contains(&"bash"));
        assert!(LITE_TOOLS.contains(&"read_file"));
        assert!(LITE_TOOLS.contains(&"write_file"));
        assert!(LITE_TOOLS.contains(&"edit_file"));

        // LITE_DEFAULT_CONTEXT_WINDOW should be reasonable for small models
        assert_eq!(LITE_DEFAULT_CONTEXT_WINDOW, 8_000);
    }
}
