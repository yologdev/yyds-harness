//! REPL slash-command routing.
//!
//! The [`dispatch_command`] function routes `/`-prefixed REPL commands to their
//! handlers. It was extracted from `repl.rs` to keep the REPL loop focused on
//! readline mechanics and the command table easy to navigate.
//!
//! CLI subcommand dispatch (`yoyo <subcmd>`) lives in [`crate::dispatch_sub`].

use std::time::Instant;

use crate::cli::{effective_context_tokens, parse_thinking_level, McpServerConfig};
use crate::commands::{
    self, auto_compact_if_needed, clear_confirmation_message, is_unknown_command,
    reset_compact_thrash, suggest_command, thinking_level_name,
};
use crate::format::*;
use crate::prompt::*;
use crate::AgentConfig;
use yoagent::context::total_tokens;
use yoagent::*;

/// Result of dispatching a slash command in the REPL.
#[derive(Debug)]
pub(crate) enum CommandResult {
    /// Command handled, go to next prompt.
    Continue,
    /// User wants to exit.
    Quit,
    /// Command produced a prompt to send to the agent.
    SendToAgent(String),
    /// Input isn't a slash command, fall through to agent.
    NotACommand,
}

/// Bundles the full REPL state needed by slash-command dispatch.
///
/// Introduced to replace the 20-parameter `dispatch_command` signature — adding
/// new state no longer requires touching every call-site.
pub(crate) struct DispatchContext<'a> {
    pub input: &'a str,
    pub agent: &'a mut yoagent::agent::Agent,
    pub agent_config: &'a mut AgentConfig,
    pub session_total: &'a mut Usage,
    pub session_changes: &'a SessionChanges,
    pub turn_history: &'a mut TurnHistory,
    pub bg_tracker: &'a commands::BackgroundJobTracker,
    pub spawn_tracker: &'a commands::SpawnTracker,
    pub undo_context: &'a mut Option<String>,
    pub last_input: &'a mut Option<String>,
    pub last_error: &'a mut Option<String>,
    pub bookmarks: &'a mut commands::Bookmarks,
    pub checkpoint_store: &'a mut commands::CheckpointStore,
    pub session_start: Instant,
    pub turn_count: usize,
    pub cwd: &'a str,
    pub mcp_cli_servers: &'a [String],
    pub mcp_server_configs: &'a [McpServerConfig],
    pub mcp_count: u32,
    pub openapi_count: u32,
}

/// Dispatch a slash command entered at the REPL prompt.
///
/// Handles all `/`-prefixed commands, returning a [`CommandResult`] that tells
/// the main loop what to do next.  This was extracted from `run_repl` to keep
/// the outer loop small and the command table easy to navigate.
pub(crate) async fn dispatch_command(ctx: &mut DispatchContext<'_>) -> CommandResult {
    match ctx.input {
        "/quit" | "/exit" => CommandResult::Quit,
        s if s == "/help" || s.starts_with("/help ") => {
            if !commands::handle_help_command(s) {
                commands::handle_help();
            }
            CommandResult::Continue
        }
        "/version" => {
            commands::handle_version();
            CommandResult::Continue
        }
        "/status" => {
            let ctx_used = total_tokens(ctx.agent.messages()) as u64;
            let ctx_max = effective_context_tokens();
            commands::handle_status(
                &ctx.agent_config.model,
                ctx.cwd,
                ctx.session_total,
                ctx.session_start.elapsed(),
                ctx.turn_count,
                ctx_used,
                ctx_max,
            );
            CommandResult::Continue
        }
        "/tokens" => {
            commands::handle_tokens(ctx.agent, ctx.session_total, &ctx.agent_config.model);
            CommandResult::Continue
        }
        "/cost" => {
            commands::handle_cost(
                ctx.session_total,
                &ctx.agent_config.model,
                ctx.agent.messages(),
            );
            CommandResult::Continue
        }
        "/profile" => {
            commands::handle_profile(
                ctx.agent,
                &ctx.agent_config.model,
                &ctx.agent_config.provider,
                ctx.session_start,
                ctx.session_total,
            );
            CommandResult::Continue
        }
        s if s == "/changelog" || s.starts_with("/changelog ") => {
            commands::handle_changelog(ctx.input);
            CommandResult::Continue
        }
        s if s == "/evolution" || s.starts_with("/evolution ") => {
            commands::handle_evolution(ctx.input);
            CommandResult::Continue
        }
        "/clear" => {
            let messages = ctx.agent.messages();
            let msg_count = messages.len();
            let token_count = yoagent::context::total_tokens(messages) as u64;
            if let Some(prompt) = clear_confirmation_message(msg_count, token_count) {
                use std::io::Write;
                print!("{DIM}  {prompt}{RESET}");
                let _ = std::io::stdout().flush();
                let mut answer = String::new();
                if std::io::stdin().read_line(&mut answer).is_ok() {
                    let answer = answer.trim().to_lowercase();
                    if answer != "y" && answer != "yes" {
                        println!("{DIM}  (clear cancelled){RESET}\n");
                        return CommandResult::Continue;
                    }
                } else {
                    println!("{DIM}  (clear cancelled){RESET}\n");
                    return CommandResult::Continue;
                }
            }
            *ctx.agent = ctx.agent_config.build_agent();
            ctx.session_changes.clear();
            ctx.turn_history.clear();
            reset_compact_thrash();
            reset_context_budget_warning();
            println!("{DIM}  (conversation cleared){RESET}\n");
            CommandResult::Continue
        }
        "/clear!" => {
            *ctx.agent = ctx.agent_config.build_agent();
            ctx.session_changes.clear();
            ctx.turn_history.clear();
            reset_compact_thrash();
            reset_context_budget_warning();
            println!("{DIM}  (conversation force-cleared){RESET}\n");
            CommandResult::Continue
        }
        "/model" => {
            commands::handle_model_show(&ctx.agent_config.model);
            CommandResult::Continue
        }
        s if s.starts_with("/model ") => {
            let arg = s.trim_start_matches("/model ").trim();
            if arg.is_empty() {
                println!("{DIM}  current model: {}", ctx.agent_config.model);
                println!("  usage: /model <name>{RESET}\n");
                return CommandResult::Continue;
            }
            if arg == "list" || arg.starts_with("list ") {
                let filter = arg.strip_prefix("list").unwrap_or("").trim();
                commands::handle_model_list(
                    &ctx.agent_config.model,
                    &ctx.agent_config.provider,
                    filter,
                );
                return CommandResult::Continue;
            }
            if arg == "info" || arg.starts_with("info ") {
                let model_name = arg.strip_prefix("info").unwrap_or("").trim();
                let target = if model_name.is_empty() {
                    &ctx.agent_config.model
                } else {
                    model_name
                };
                commands::handle_model_info(target, &ctx.agent_config.model);
                return CommandResult::Continue;
            }
            let new_model = arg;
            ctx.agent_config.model = new_model.to_string();
            // Rebuild ctx.agent with new model, preserving conversation
            let saved = ctx.agent.save_messages().ok();
            *ctx.agent = ctx.agent_config.build_agent();
            let restored = if let Some(json) = saved {
                ctx.agent.restore_messages(&json).is_ok()
            } else {
                false
            };
            if restored {
                println!("{DIM}  (switched to {new_model}, conversation preserved){RESET}\n");
            } else {
                println!("{YELLOW}  (switched to {new_model}, conversation could not be preserved){RESET}\n");
            }
            CommandResult::Continue
        }
        "/provider" => {
            commands::handle_provider_show(&ctx.agent_config.provider);
            CommandResult::Continue
        }
        s if s.starts_with("/provider ") => {
            let new_provider = s.trim_start_matches("/provider ").trim();
            if new_provider.is_empty() {
                commands::handle_provider_show(&ctx.agent_config.provider);
                return CommandResult::Continue;
            }
            commands::handle_provider_switch(new_provider, ctx.agent_config, ctx.agent);
            CommandResult::Continue
        }
        "/think" => {
            commands::handle_think_show(ctx.agent_config.thinking);
            CommandResult::Continue
        }
        s if s.starts_with("/think ") => {
            let level_str = s.trim_start_matches("/think ").trim();
            if level_str.is_empty() {
                let current = thinking_level_name(ctx.agent_config.thinking);
                println!("{DIM}  thinking: {current}");
                println!("  usage: /think <off|minimal|low|medium|high>{RESET}\n");
                return CommandResult::Continue;
            }
            let new_thinking = parse_thinking_level(level_str);
            if new_thinking == ctx.agent_config.thinking {
                let current = thinking_level_name(ctx.agent_config.thinking);
                println!("{DIM}  thinking already set to {current}{RESET}\n");
                return CommandResult::Continue;
            }
            ctx.agent_config.thinking = new_thinking;
            // Rebuild ctx.agent with new thinking level, preserving conversation
            let saved = ctx.agent.save_messages().ok();
            *ctx.agent = ctx.agent_config.build_agent();
            let restored = if let Some(json) = saved {
                ctx.agent.restore_messages(&json).is_ok()
            } else {
                false
            };
            let level_name = thinking_level_name(ctx.agent_config.thinking);
            if restored {
                println!("{DIM}  (thinking set to {level_name}, conversation preserved){RESET}\n");
            } else {
                println!("{YELLOW}  (thinking set to {level_name}, conversation could not be preserved){RESET}\n");
            }
            CommandResult::Continue
        }
        s if s == "/save" || s.starts_with("/save ") => {
            commands::handle_save(ctx.agent, ctx.input);
            CommandResult::Continue
        }
        s if s == "/load" || s.starts_with("/load ") => {
            commands::handle_load(ctx.agent, ctx.input);
            reset_compact_thrash();
            CommandResult::Continue
        }
        s if s == "/stash" || s.starts_with("/stash ") => {
            let result = commands::handle_stash(ctx.agent, s);
            print!("{result}");
            CommandResult::Continue
        }
        s if s == "/checkpoint" || s.starts_with("/checkpoint ") => {
            commands::handle_checkpoint(s, ctx.checkpoint_store, ctx.session_changes);
            CommandResult::Continue
        }
        s if s == "/diff" || s.starts_with("/diff ") => {
            commands::handle_diff(s);
            CommandResult::Continue
        }
        s if s == "/blame" || s.starts_with("/blame ") => {
            commands::handle_blame(s);
            CommandResult::Continue
        }
        s if s == "/undo" || s.starts_with("/undo ") => {
            if let Some(undo_ctx) = commands::handle_undo(s, ctx.turn_history) {
                *ctx.undo_context = Some(undo_ctx);
            }
            CommandResult::Continue
        }
        "/health" => {
            commands::handle_health();
            CommandResult::Continue
        }
        "/doctor" => {
            commands::handle_doctor(&ctx.agent_config.provider, &ctx.agent_config.model);
            CommandResult::Continue
        }
        "/test" => {
            commands::handle_test();
            CommandResult::Continue
        }
        "/lint fix" => {
            if let Some(fix_prompt) =
                commands::handle_lint_fix(ctx.agent, ctx.session_total, &ctx.agent_config.model)
                    .await
            {
                *ctx.last_input = Some(fix_prompt);
            }
            CommandResult::Continue
        }
        s if s == "/lint" || s.starts_with("/lint ") => {
            if let Some(lint_result) = commands::handle_lint(s) {
                if lint_result.starts_with("Lint FAILED")
                    || lint_result.starts_with("Failed to run")
                {
                    *ctx.last_input = Some(lint_result);
                }
            }
            CommandResult::Continue
        }
        "/fix" => {
            if let Some(fix_prompt) =
                commands::handle_fix(ctx.agent, ctx.session_total, &ctx.agent_config.model).await
            {
                *ctx.last_input = Some(fix_prompt);
            }
            CommandResult::Continue
        }
        s if s == "/history" || s.starts_with("/history ") => {
            let sub = s.strip_prefix("/history").unwrap_or("").trim();
            if sub == "detail" {
                commands::handle_history_detail(ctx.agent);
            } else {
                commands::handle_history(ctx.agent);
            }
            CommandResult::Continue
        }
        "/search" => {
            commands::handle_search(ctx.agent, ctx.input);
            CommandResult::Continue
        }
        s if s.starts_with("/search ") => {
            commands::handle_search(ctx.agent, ctx.input);
            CommandResult::Continue
        }
        "/marks" => {
            commands::handle_marks(ctx.bookmarks);
            CommandResult::Continue
        }
        s if s == "/changes" || s.starts_with("/changes ") => {
            commands::handle_changes(ctx.session_changes, ctx.input);
            CommandResult::Continue
        }
        s if s == "/export" || s.starts_with("/export ") => {
            commands::handle_export(ctx.agent, ctx.input);
            CommandResult::Continue
        }
        s if s == "/mark" || s.starts_with("/mark ") => {
            commands::handle_mark(ctx.agent, ctx.input, ctx.bookmarks);
            CommandResult::Continue
        }
        s if s == "/jump" || s.starts_with("/jump ") => {
            commands::handle_jump(ctx.agent, ctx.input, ctx.bookmarks);
            CommandResult::Continue
        }
        "/config" => {
            commands::handle_config(
                &ctx.agent_config.provider,
                &ctx.agent_config.model,
                &ctx.agent_config.base_url,
                ctx.agent_config.thinking,
                ctx.agent_config.max_tokens,
                ctx.agent_config.max_turns,
                ctx.agent_config.temperature,
                &ctx.agent_config.skills,
                &ctx.agent_config.system_prompt,
                ctx.mcp_count,
                ctx.openapi_count,
                ctx.agent_config.shell_hooks.len(),
                ctx.agent,
                ctx.cwd,
            );
            CommandResult::Continue
        }
        s if s == "/config show" || s.starts_with("/config show ") => {
            commands::handle_config_show();
            CommandResult::Continue
        }
        s if s == "/config edit" || s.starts_with("/config edit ") => {
            commands::handle_config_edit();
            CommandResult::Continue
        }
        s if s.starts_with("/config set") => {
            commands::handle_config_set(ctx.input, ctx.agent_config, ctx.agent);
            CommandResult::Continue
        }
        s if s == "/config get" || s.starts_with("/config get ") => {
            commands::handle_config_get(ctx.input);
            CommandResult::Continue
        }
        "/hooks" => {
            commands::handle_hooks(&ctx.agent_config.shell_hooks);
            CommandResult::Continue
        }
        "/permissions" => {
            commands::handle_permissions(
                ctx.agent_config.auto_approve,
                &ctx.agent_config.permissions,
                &ctx.agent_config.dir_restrictions,
            );
            CommandResult::Continue
        }
        "/compact" => {
            commands::handle_compact(ctx.agent);
            CommandResult::Continue
        }
        s if s == "/commit" || s.starts_with("/commit ") => {
            commands::handle_commit(ctx.input);
            CommandResult::Continue
        }
        s if s == "/context" || s.starts_with("/context ") => {
            commands::handle_context(ctx.input, &ctx.agent_config.system_prompt, ctx.agent);
            CommandResult::Continue
        }
        s if s == "/add" || s.starts_with("/add ") => {
            let results = commands::handle_add(ctx.input);
            if !results.is_empty() {
                // Print summaries
                for result in &results {
                    match result {
                        commands::AddResult::Text { summary, .. } => println!("{summary}"),
                        commands::AddResult::Image { summary, .. } => println!("{summary}"),
                    }
                }
                // Build content blocks with proper text context for images
                let content_blocks = crate::conversations::build_add_content_blocks(&results);
                let word = crate::format::pluralize(results.len(), "file", "files");
                println!(
                    "{}  ({} {word} added to conversation){}\n",
                    DIM,
                    results.len(),
                    RESET
                );
                // Inject as a user message so the AI sees the file contents
                let msg = yoagent::types::AgentMessage::Llm(yoagent::types::Message::User {
                    content: content_blocks,
                    timestamp: yoagent::types::now_ms(),
                });
                ctx.agent.append_message(msg);
            }
            CommandResult::Continue
        }
        "/docs" => {
            commands::handle_docs(ctx.input);
            CommandResult::Continue
        }
        s if s.starts_with("/docs ") => {
            commands::handle_docs(ctx.input);
            CommandResult::Continue
        }
        "/find" => {
            commands::handle_find(ctx.input);
            CommandResult::Continue
        }
        s if s.starts_with("/find ") => {
            commands::handle_find(ctx.input);
            CommandResult::Continue
        }
        "/grep" => {
            commands::handle_grep(ctx.input);
            CommandResult::Continue
        }
        s if s.starts_with("/grep ") => {
            commands::handle_grep(ctx.input);
            CommandResult::Continue
        }
        "/init" => {
            commands::handle_init();
            CommandResult::Continue
        }
        s if s == "/rename" || s.starts_with("/rename ") => {
            commands::handle_rename(ctx.input);
            CommandResult::Continue
        }
        s if s == "/extract" || s.starts_with("/extract ") => {
            commands::handle_extract(ctx.input);
            CommandResult::Continue
        }
        s if s == "/move" || s.starts_with("/move ") => {
            commands::handle_move(ctx.input);
            CommandResult::Continue
        }
        s if s == "/refactor" || s.starts_with("/refactor ") => {
            commands::handle_refactor(ctx.input);
            CommandResult::Continue
        }
        s if s == "/remember" || s.starts_with("/remember ") => {
            commands::handle_remember(ctx.input);
            CommandResult::Continue
        }
        s if s == "/memories" || s.starts_with("/memories ") => {
            commands::handle_memories(ctx.input);
            CommandResult::Continue
        }
        s if s == "/forget" || s.starts_with("/forget ") => {
            commands::handle_forget(ctx.input);
            CommandResult::Continue
        }
        "/index" => {
            commands::handle_index();
            CommandResult::Continue
        }
        s if s == "/map" || s.starts_with("/map ") => {
            commands::handle_map(ctx.input);
            CommandResult::Continue
        }
        s if s == "/outline" || s.starts_with("/outline ") => {
            commands::handle_outline(ctx.input);
            CommandResult::Continue
        }
        "/retry" => {
            *ctx.last_error = commands::handle_retry(
                ctx.agent,
                ctx.last_input,
                ctx.last_error,
                ctx.session_total,
                &ctx.agent_config.model,
            )
            .await;
            CommandResult::Continue
        }
        s if s == "/tree" || s.starts_with("/tree ") => {
            commands::handle_tree(ctx.input);
            CommandResult::Continue
        }
        s if s == "/web" || s.starts_with("/web ") => {
            commands::handle_web(ctx.input);
            CommandResult::Continue
        }
        s if s == "/watch" || s.starts_with("/watch ") => {
            commands::handle_watch(ctx.input);
            CommandResult::Continue
        }
        s if s == "/loop" || s.starts_with("/loop ") => {
            commands::handle_loop(
                ctx.input,
                ctx.agent,
                ctx.session_total,
                ctx.agent_config,
                ctx.session_changes,
            )
            .await;
            CommandResult::Continue
        }
        s if s == "/todo" || s.starts_with("/todo ") => {
            let result = commands::handle_todo(ctx.input);
            println!("{result}\n");
            CommandResult::Continue
        }
        s if s == "/teach" || s.starts_with("/teach ") => {
            commands::handle_teach(ctx.input);
            CommandResult::Continue
        }
        s if s == "/architect" || s.starts_with("/architect ") => {
            commands::handle_architect(ctx.input);
            CommandResult::Continue
        }
        s if s == "/mcp" || s.starts_with("/mcp ") => {
            commands::handle_mcp(
                ctx.input,
                ctx.mcp_cli_servers,
                ctx.mcp_server_configs,
                ctx.mcp_count,
            );
            CommandResult::Continue
        }
        s if s == "/ast" || s.starts_with("/ast ") => {
            commands::handle_ast_grep(ctx.input);
            CommandResult::Continue
        }
        s if s == "/apply" || s.starts_with("/apply ") => {
            commands::handle_apply(ctx.input);
            CommandResult::Continue
        }
        s if s == "/bg" || s.starts_with("/bg ") => {
            let args = ctx.input.strip_prefix("/bg").unwrap_or("").trim();
            commands::handle_bg(args, ctx.bg_tracker).await;
            CommandResult::Continue
        }
        s if s.starts_with("/run ") || (s.starts_with('!') && s.len() > 1) => {
            commands::handle_run(ctx.input);
            CommandResult::Continue
        }
        "/run" => {
            commands::handle_run_usage();
            CommandResult::Continue
        }
        s if s == "/pr" || s.starts_with("/pr ") => {
            commands::handle_pr(
                ctx.input,
                ctx.agent,
                ctx.session_total,
                &ctx.agent_config.model,
            )
            .await;
            CommandResult::Continue
        }
        s if s == "/git" || s.starts_with("/git ") => {
            commands::handle_git(ctx.input);
            CommandResult::Continue
        }
        s if s == "/goal" || s.starts_with("/goal ") => commands::handle_goal(ctx.input),
        s if s == "/spawn" || s.starts_with("/spawn ") => {
            if let Some(context_msg) = commands::handle_spawn(
                ctx.input,
                ctx.agent_config,
                ctx.session_total,
                &ctx.agent_config.model,
                ctx.agent.messages(),
                ctx.spawn_tracker,
            )
            .await
            {
                *ctx.last_input = Some(context_msg.clone());
                let prompt_start = Instant::now();
                let outcome = run_prompt_with_changes(
                    ctx.agent,
                    &context_msg,
                    ctx.session_total,
                    &ctx.agent_config.model,
                    ctx.session_changes,
                )
                .await;
                crate::format::maybe_ring_bell(prompt_start.elapsed());
                *ctx.last_error = outcome.last_tool_error;
                auto_compact_if_needed(ctx.agent);
            }
            CommandResult::Continue
        }
        s if s == "/review" || s.starts_with("/review ") => {
            if let Some(review_prompt) = commands::handle_review(
                ctx.input,
                ctx.agent,
                ctx.session_total,
                &ctx.agent_config.model,
            )
            .await
            {
                *ctx.last_input = Some(review_prompt);
            }
            CommandResult::Continue
        }
        "/update" => {
            match commands::handle_update() {
                Ok(_) => println!(
                    "Update completed successfully. Please restart yoyo to use the new version."
                ),
                Err(e) => eprintln!("Update failed: {}", e),
            }
            CommandResult::Continue
        }
        s if s == "/skill" || s.starts_with("/skill ") => {
            commands::handle_skill(ctx.input, &ctx.agent_config.skills);
            CommandResult::Continue
        }
        s if s == "/explain" || s.starts_with("/explain ") => {
            if let Some(prompt) = commands::build_explain_prompt(ctx.input) {
                *ctx.last_input = Some(prompt.clone());
                let prompt_start = Instant::now();
                let outcome = run_prompt_with_changes(
                    ctx.agent,
                    &prompt,
                    ctx.session_total,
                    &ctx.agent_config.model,
                    ctx.session_changes,
                )
                .await;
                crate::format::maybe_ring_bell(prompt_start.elapsed());
                *ctx.last_error = outcome.last_tool_error;
                auto_compact_if_needed(ctx.agent);
            }
            CommandResult::Continue
        }
        s if s == "/plan" || s.starts_with("/plan ") => {
            if let Some(plan_prompt) = commands::handle_plan(
                ctx.input,
                ctx.agent,
                ctx.session_total,
                &ctx.agent_config.model,
            )
            .await
            {
                *ctx.last_input = Some(plan_prompt);
            }
            CommandResult::Continue
        }
        s if s == "/extended" || s.starts_with("/extended ") => {
            if let Some(extended_prompt) = crate::conversations::handle_extended(
                ctx.input,
                ctx.agent,
                ctx.session_total,
                &ctx.agent_config.model,
                ctx.session_changes,
            )
            .await
            {
                *ctx.last_input = Some(extended_prompt);
                *ctx.last_error = None; // Clear — handle_extended reports its own errors
                auto_compact_if_needed(ctx.agent);
            }
            CommandResult::Continue
        }
        s if s == "/side" || s.starts_with("/side ") => {
            crate::conversations::handle_side(ctx.input, ctx.agent_config).await;
            CommandResult::Continue
        }
        s if s == "/quick" || s.starts_with("/quick ") => {
            crate::conversations::handle_quick(ctx.input, ctx.agent_config).await;
            CommandResult::Continue
        }
        // Custom slash commands: loaded from .yoyo/commands/ and ~/.yoyo/commands/
        // Also catches unknown commands (anything starting with '/' not matched above)
        s if s.starts_with('/') => {
            let cmd_name = s[1..].split_whitespace().next().unwrap_or(&s[1..]);
            if let Some(content) = crate::commands::get_custom_command_content(cmd_name) {
                eprintln!("{DIM}  running custom command /{cmd_name}{RESET}");
                CommandResult::SendToAgent(content)
            } else if is_unknown_command(s) {
                let cmd = s.split_whitespace().next().unwrap_or(s);
                eprintln!("{RED}  unknown command: {cmd}{RESET}");
                if let Some(suggestion) = suggest_command(s) {
                    eprintln!("{YELLOW}  did you mean {suggestion}?{RESET}");
                }
                eprintln!("{DIM}  type /help for available commands{RESET}\n");
                CommandResult::Continue
            } else {
                // Shouldn't happen — known command not matched above
                CommandResult::Continue
            }
        }
        _ => CommandResult::NotACommand,
    }
}
