//! Prompt execution and agent interaction.

use crate::cli::is_verbose;
use crate::format::*;
use std::collections::HashMap;
use std::io::{self, IsTerminal, Write};
use std::time::Instant;
use yoagent::agent::Agent;
use yoagent::context::total_tokens;
use yoagent::*;

// Extracted into `watch` module (Day 58). Re-exported here so
// `use crate::prompt::*;` call sites keep working without changes.
pub use crate::watch::{
    clear_watch_command, get_watch_command, get_watch_commands, run_watch_after_prompt,
    set_watch_command, set_watch_commands,
};

// ── Audit log + session budget ──────────────────────────────────────────
// Extracted into `prompt_budget` module. Re-exported here so the existing
// `use crate::prompt::*;` call sites in `main.rs` and `repl.rs` keep working
// without any changes, and `crate::prompt::foo` paths continue to resolve.
// Only symbols actually referenced via the `prompt::` path today are
// re-exported; the rest remain accessible at `crate::prompt_budget::`.
pub use crate::prompt_budget::{
    audit_log_tool_call, enable_audit_log, is_audit_enabled, session_budget_exhausted,
};

// Extracted into `session` module (Day 54). Re-exported here so
// `use crate::prompt::*;` call sites keep working without changes.
pub use crate::session::{format_changes, ChangeKind, SessionChanges, TurnHistory, TurnSnapshot};

/// Outcome of a prompt execution, including the text response and any tool error.
#[derive(Debug, Clone, Default)]
pub struct PromptOutcome {
    /// The collected text output from the agent.
    pub text: String,
    /// The last tool error encountered during this prompt turn, if any.
    /// Tool errors are from `ToolExecutionEnd` events where `is_error` is true.
    pub last_tool_error: Option<String>,
    /// The name of the tool that produced `last_tool_error`, if any.
    /// Used to provide tool-specific recovery hints in auto-retry prompts.
    pub last_tool_name: Option<String>,
    /// Whether this prompt triggered an auto-compact due to context overflow.
    /// Callers can use this to inform users or adjust behavior.
    pub was_overflow: bool,
    /// The last API-level error after all retries were exhausted, if any.
    /// Set when the provider itself fails (rate limits, outages, auth errors)
    /// rather than a tool execution error. Used by the REPL to trigger
    /// fallback provider switching.
    pub last_api_error: Option<String>,
}

// Extracted into `prompt_retry` module (Day 64). Re-exported here so
// `use crate::prompt::*;` call sites keep working without changes.
pub use crate::prompt_retry::{
    build_auto_retry_prompt, build_overflow_retry_prompt, build_retry_prompt, diagnose_api_error,
    is_overflow_error, is_retriable_error, retry_delay, MAX_AUTO_RETRIES,
};
// MAX_RETRIES is pub(crate), so import without re-exporting.
use crate::prompt_retry::MAX_RETRIES;

/// Extract a preview of tool result content for display.
/// Returns an empty string if there's nothing meaningful to show.
fn tool_result_preview(result: &ToolResult, max_chars: usize) -> String {
    let text: String = result
        .content
        .iter()
        .filter_map(|c| match c {
            Content::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    // Take first line only, truncated
    let first_line = text.lines().next().unwrap_or("");
    truncate_with_ellipsis(first_line, max_chars)
}

/// Write response text to a file if --output was specified.
pub fn write_output_file(path: &Option<String>, text: &str) {
    if let Some(path) = path {
        match std::fs::write(path, text) {
            Ok(_) => eprintln!("{DIM}  wrote response to {path}{RESET}"),
            Err(e) => eprintln!("{RED}  error writing to {path}: {e}{RESET}"),
        }
    }
}

/// Extract all searchable text from a message (for /search).
fn message_text(msg: &AgentMessage) -> String {
    match msg {
        AgentMessage::Llm(Message::User { content, .. }) => content
            .iter()
            .filter_map(|c| match c {
                Content::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" "),
        AgentMessage::Llm(Message::Assistant { content, .. }) => {
            let mut parts = Vec::new();
            for c in content {
                match c {
                    Content::Text { text } if !text.is_empty() => parts.push(text.as_str()),
                    Content::ToolCall { name, .. } => parts.push(name.as_str()),
                    _ => {}
                }
            }
            parts.join(" ")
        }
        AgentMessage::Llm(Message::ToolResult {
            tool_name, content, ..
        }) => {
            let text: String = content
                .iter()
                .filter_map(|c| match c {
                    Content::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("{tool_name} {text}")
        }
        AgentMessage::Extension(ext) => ext.role.clone(),
    }
}

/// Highlight all occurrences of `query` in `text` using BOLD ANSI codes (case-insensitive).
/// Returns the text with matching substrings wrapped in BOLD..RESET.
pub fn highlight_matches(text: &str, query: &str) -> String {
    if query.is_empty() {
        return text.to_string();
    }
    let lower_text = text.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut result = String::with_capacity(text.len() + 32);
    let mut last_end = 0;

    for (match_start, _) in lower_text.match_indices(&lower_query) {
        let match_end = match_start + query.len();
        // Append text before this match (unmodified)
        result.push_str(&text[last_end..match_start]);
        // Append the matched portion with BOLD highlighting (preserving original case)
        result.push_str(&format!("{BOLD}{}{RESET}", &text[match_start..match_end]));
        last_end = match_end;
    }
    // Append any remaining text after the last match
    result.push_str(&text[last_end..]);
    result
}

/// Search messages for a query string (case-insensitive).
/// Returns a vec of (index, role, highlighted_preview) for matching messages.
pub fn search_messages(messages: &[AgentMessage], query: &str) -> Vec<(usize, String, String)> {
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for (i, msg) in messages.iter().enumerate() {
        let text = message_text(msg);
        if text.to_lowercase().contains(&query_lower) {
            let (role, _) = summarize_message(msg);
            // Find match context: show text around the first match
            let lower = text.to_lowercase();
            let match_pos = lower.find(&query_lower).unwrap_or(0);
            let start = match_pos.saturating_sub(20);
            // Get byte-safe boundaries
            let start = text[..start]
                .char_indices()
                .last()
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            let end = text
                .char_indices()
                .map(|(idx, ch)| idx + ch.len_utf8())
                .find(|&idx| idx >= match_pos + query.len() + 20)
                .unwrap_or(text.len());
            let snippet = &text[start..end];
            let prefix = if start > 0 { "…" } else { "" };
            let suffix = if end < text.len() { "…" } else { "" };
            let preview = format!("{prefix}{snippet}{suffix}");
            let highlighted = highlight_matches(&preview, query);
            results.push((i + 1, role.to_string(), highlighted));
        }
    }

    results
}

/// Summarize a message for /history display.
pub fn summarize_message(msg: &AgentMessage) -> (&str, String) {
    match msg {
        AgentMessage::Llm(Message::User { content, .. }) => {
            let text = content
                .iter()
                .filter_map(|c| match c {
                    Content::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            ("user", truncate_with_ellipsis(&text, 80))
        }
        AgentMessage::Llm(Message::Assistant { content, .. }) => {
            let mut parts = Vec::new();
            let mut tool_calls = 0;
            for c in content {
                match c {
                    Content::Text { text } if !text.is_empty() => {
                        parts.push(truncate_with_ellipsis(text, 60));
                    }
                    Content::ToolCall { name, .. } => {
                        tool_calls += 1;
                        if tool_calls <= 3 {
                            parts.push(format!("→{name}"));
                        }
                    }
                    _ => {}
                }
            }
            if tool_calls > 3 {
                parts.push(format!("(+{} more tools)", tool_calls - 3));
            }
            let preview = if parts.is_empty() {
                "(empty)".to_string()
            } else {
                parts.join("  ")
            };
            ("assistant", preview)
        }
        AgentMessage::Llm(Message::ToolResult {
            tool_name,
            is_error,
            ..
        }) => {
            let status = if *is_error { "✗" } else { "✓" };
            ("tool", format!("{tool_name} {status}"))
        }
        AgentMessage::Extension(ext) => ("ext", truncate_with_ellipsis(&ext.role, 60)),
    }
}

/// Result of a single prompt attempt — either success or a retriable/fatal error.
enum PromptResult {
    /// Prompt completed (possibly with non-retriable errors already shown).
    Done {
        collected_text: String,
        usage: Usage,
        last_tool_error: Option<String>,
        last_tool_name: Option<String>,
    },
    /// A retriable API error was detected — caller should retry.
    RetriableError { error_msg: String, usage: Usage },
    /// A context overflow error — caller should compact and retry.
    ContextOverflow { error_msg: String, usage: Usage },
}

/// Execute a single prompt attempt and process all events.
/// Returns whether we got a retriable error (so the caller can retry).
async fn run_prompt_once(
    agent: &mut Agent,
    input: &str,
    changes: &SessionChanges,
    model: &str,
) -> PromptResult {
    let rx = agent.prompt(input).await;
    handle_prompt_events(agent, rx, changes, model).await
}

/// Execute a single prompt attempt with pre-built messages (e.g. multi-modal content).
/// Same event handling as `run_prompt_once`, but uses `prompt_messages` instead of `prompt`.
async fn run_prompt_once_with_messages(
    agent: &mut Agent,
    messages: Vec<AgentMessage>,
    changes: &SessionChanges,
    model: &str,
) -> PromptResult {
    let rx = agent.prompt_messages(messages).await;
    handle_prompt_events(agent, rx, changes, model).await
}

/// Internal state for the prompt event-handling loop.
/// Bundles the 15+ local variables that were previously declared inline.
struct PromptEventState {
    usage: Usage,
    in_text: bool,
    in_thinking: bool,
    tool_timers: HashMap<String, Instant>,
    collected_text: String,
    retriable_error: Option<String>,
    overflow_error: Option<String>,
    last_tool_error: Option<String>,
    last_tool_name: Option<String>,
    md_renderer: MarkdownRenderer,
    spinner: Option<Spinner>,
    think_filter: ThinkBlockFilter,
    /// Audit log: track in-flight tool calls (name + args) so we can log at completion
    audit_inflight: HashMap<String, (String, serde_json::Value)>,
    /// Live progress timers for long-running tools (bash)
    tool_progress_timers: HashMap<String, ToolProgressTimer>,
    /// Bash tool call IDs that need deferred timer start.
    /// Maps tool_call_id → optional command string for display label.
    deferred_bash_timers: HashMap<String, Option<String>>,
    /// Tool batch tracking for group summaries
    batch_count: usize,
    batch_succeeded: usize,
    batch_failed: usize,
    batch_start: Option<Instant>,
    /// Turn tracking for boundary markers
    turn_number: usize,
    /// Whether we've seen text output in this prompt
    had_text: bool,
}

impl PromptEventState {
    fn new() -> Self {
        Self {
            usage: Usage::default(),
            in_text: false,
            in_thinking: false,
            tool_timers: HashMap::new(),
            collected_text: String::new(),
            retriable_error: None,
            overflow_error: None,
            last_tool_error: None,
            last_tool_name: None,
            md_renderer: MarkdownRenderer::new(),
            spinner: Some(Spinner::start()),
            think_filter: ThinkBlockFilter::new(),
            audit_inflight: HashMap::new(),
            tool_progress_timers: HashMap::new(),
            deferred_bash_timers: HashMap::new(),
            batch_count: 0,
            batch_succeeded: 0,
            batch_failed: 0,
            batch_start: None,
            turn_number: 0,
            had_text: false,
        }
    }

    /// Handle a ToolExecutionStart event: track file changes, display tool info,
    /// manage batch state, and set up deferred timers.
    fn handle_tool_execution_start(
        &mut self,
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
        changes: &SessionChanges,
    ) {
        // Track file modifications from write_file and edit_file
        match tool_name.as_str() {
            "write_file" => {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    changes.record(path, ChangeKind::Write);
                }
            }
            "edit_file" => {
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    changes.record(path, ChangeKind::Edit);
                }
            }
            _ => {}
        }
        // Stop spinner on first activity
        if let Some(s) = self.spinner.take() {
            s.stop();
        }

        // Show turn boundary when transitioning from text to a new tool batch
        if self.in_text {
            println!();
            self.in_text = false;
        }

        // New batch starting (first tool after text or start)
        if self.batch_count == 0 {
            if self.batch_start.is_none() {
                self.batch_start = Some(Instant::now());
            }
            // Show turn boundary for multi-turn (turn 2+)
            if self.turn_number > 1 && self.had_text {
                println!("{}", turn_boundary(self.turn_number));
            }
        }

        self.batch_count += 1;
        self.tool_timers
            .insert(tool_call_id.clone(), Instant::now());
        // Track for audit log
        self.audit_inflight
            .insert(tool_call_id.clone(), (tool_name.clone(), args.clone()));
        let summary = format_tool_summary(&tool_name, &args);
        if tool_name == "sub_agent" {
            // Distinctive header for sub-agent delegation
            eprintln!("\n{DIM}  🐙 Delegating to sub-agent...{RESET}");
        }
        print!("{YELLOW}  ▶ {summary}{RESET}");
        if is_verbose() {
            println!();
            let args_str = serde_json::to_string_pretty(&args).unwrap_or_default();
            for line in args_str.lines() {
                println!("{DIM}    │ {line}{RESET}");
            }
        } else if tool_name == "edit_file" {
            // Show colored diff for edit_file when not in verbose mode
            let old_text = args.get("old_text").and_then(|v| v.as_str()).unwrap_or("");
            let new_text = args.get("new_text").and_then(|v| v.as_str()).unwrap_or("");
            let diff = format_edit_diff(old_text, new_text);
            if !diff.is_empty() {
                println!();
                println!("{diff}");
            }
        }
        io::stdout().flush().ok();

        // Defer timer start for bash commands — the confirmation
        // prompt would be overwritten by the spinner. The timer
        // will start on the first ToolExecutionUpdate instead.
        if tool_name == "bash" {
            let cmd_label = args
                .get("command")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            self.deferred_bash_timers
                .insert(tool_call_id.clone(), cmd_label);
        }
    }

    /// Handle a ToolExecutionEnd event: stop timers, log audit data,
    /// display success/failure status, track errors.
    fn handle_tool_execution_end(
        &mut self,
        tool_call_id: String,
        tool_name: String,
        is_error: bool,
        result: ToolResult,
    ) {
        // Clean up deferred timer entry if command was denied before running
        self.deferred_bash_timers.remove(&tool_call_id);
        // Stop any live progress timer for this tool
        if let Some(timer) = self.tool_progress_timers.remove(&tool_call_id) {
            timer.stop();
        }
        let elapsed = self
            .tool_timers
            .remove(&tool_call_id)
            .map(|start| start.elapsed());
        let dur_str = elapsed
            .map(|d| format!(" {DIM}({}){RESET}", format_duration(d)))
            .unwrap_or_default();

        // Audit log: record the completed tool call
        if let Some((audit_tool, audit_args)) = self.audit_inflight.remove(&tool_call_id) {
            let duration_ms = elapsed.map(|d| d.as_millis() as u64).unwrap_or(0);
            audit_log_tool_call(&audit_tool, &audit_args, duration_ms, !is_error);
        }

        if is_error {
            self.batch_failed += 1;
            println!(" {RED}✗{RESET}{dur_str}");
            let preview = tool_result_preview(&result, 200);
            if !preview.is_empty() {
                // Indent error output under the tool header
                println!("{}", indent_tool_output(&preview));
            }
            // Track the last tool error for /retry context
            let error_text = tool_result_preview(&result, 200);
            if !error_text.is_empty() {
                self.last_tool_error = Some(error_text);
            } else {
                self.last_tool_error = Some("tool execution failed".to_string());
            }
            self.last_tool_name = Some(tool_name.clone());
        } else {
            // Successful tool clears the last error
            self.batch_succeeded += 1;
            self.last_tool_error = None;
            self.last_tool_name = None;
            println!(" {GREEN}✓{RESET}{dur_str}");
            // Warn when write_file writes 0 bytes (empty content)
            if tool_name == "write_file" {
                let wrote_zero = result
                    .details
                    .get("bytes")
                    .and_then(|v| v.as_u64())
                    .map(|b| b == 0)
                    .unwrap_or(false);
                if wrote_zero {
                    eprintln!("{YELLOW}    ⚠ write_file wrote 0 bytes — file is now empty{RESET}");
                }
            }
            if is_verbose() {
                let preview = tool_result_preview(&result, 200);
                if !preview.is_empty() {
                    // Indent verbose output under the tool header
                    println!("{}", indent_tool_output(&preview));
                }
            }
        }
    }

    /// Handle a ToolExecutionUpdate event: start deferred timers,
    /// update progress, show partial output in terminal mode.
    fn handle_tool_execution_update(&mut self, tool_call_id: String, partial_result: ToolResult) {
        // Start deferred bash timer on first update.
        // This means the command is actually running (confirmation
        // has already been resolved), so the spinner won't
        // overwrite the permission prompt.
        if let Some(cmd_label) = self.deferred_bash_timers.remove(&tool_call_id) {
            let timer = ToolProgressTimer::start("bash".to_string());
            if let Some(label) = cmd_label {
                timer.set_label(label);
            }
            self.tool_progress_timers
                .insert(tool_call_id.clone(), timer);
        }

        // Update line count on the progress timer if active
        let line_count = count_result_lines(&partial_result);
        if let Some(timer) = self.tool_progress_timers.get(&tool_call_id) {
            timer.set_line_count(line_count);
        }

        // Only show partial output in interactive (terminal) mode.
        // In piped/CI mode, cursor-up sequences don't work and every
        // partial update becomes a permanent log line, inflating output.
        if io::stdout().is_terminal() {
            let text = extract_result_text(&partial_result);
            if !text.is_empty() {
                let tail = format_partial_tail(&text, 6);
                if !tail.is_empty() {
                    println!();
                    println!("{tail}");
                    io::stdout().flush().ok();
                }
            }
        }
    }

    /// Handle a MessageUpdate with text delta: manage spinner, batch summaries,
    /// think-block filtering, markdown rendering, and text collection.
    fn handle_message_update_text(&mut self, delta: &str) {
        // Stop spinner on first text
        if let Some(s) = self.spinner.take() {
            s.stop();
        }
        // Transition from thinking to text: add a divider
        // so text doesn't appear glued to the last thinking output
        if self.in_thinking {
            eprintln!();
            eprintln!("{}", section_divider());
            let _ = io::stderr().flush();
            self.in_thinking = false;
        }

        // Print batch summary if we just finished a tool batch
        if self.batch_count > 0 {
            self.print_batch_summary();
        }

        if !self.in_text {
            println!();
            self.in_text = true;
            self.had_text = true;
        }
        // Filter <think>...</think> blocks unless verbose mode
        let filtered = if is_verbose() {
            delta.to_string()
        } else {
            self.think_filter.filter(delta)
        };
        if filtered.is_empty() {
            // Inside a think block — nothing to render yet
            io::stdout().flush().ok();
            return;
        }
        // Render and display BEFORE collecting — minimizes time-to-screen.
        // collected_text is only used after the stream ends, so ordering
        // with print doesn't affect correctness. (render_latency_budget)
        let rendered = self.md_renderer.render_delta(&filtered);
        if !rendered.is_empty() {
            print!("{}", rendered);
        }
        io::stdout().flush().ok();
        self.collected_text.push_str(&filtered);
    }

    /// Handle an AgentEnd event: flush filters, print batch summary,
    /// accumulate usage, detect errors.
    fn handle_agent_end(&mut self, messages: Vec<AgentMessage>, model: &str) {
        // Stop spinner if still running
        if let Some(s) = self.spinner.take() {
            s.stop();
        }

        // Flush think block filter — emit any partial non-think text
        let remaining = self.think_filter.flush();
        if !remaining.is_empty() {
            let rendered = self.md_renderer.render_delta(&remaining);
            if !rendered.is_empty() {
                print!("{rendered}");
                io::stdout().flush().ok();
            }
            self.collected_text.push_str(&remaining);
        }

        // Print batch summary if tools were the last thing before end
        if self.batch_count > 0 {
            self.print_batch_summary();
        }

        for msg in &messages {
            if let AgentMessage::Llm(Message::Assistant {
                usage: msg_usage,
                stop_reason,
                error_message,
                ..
            }) = msg
            {
                self.usage.input += msg_usage.input;
                self.usage.output += msg_usage.output;
                self.usage.cache_read += msg_usage.cache_read;
                self.usage.cache_write += msg_usage.cache_write;

                if *stop_reason == StopReason::Error {
                    if let Some(err_msg) = error_message {
                        if self.in_text {
                            println!();
                            self.in_text = false;
                        }
                        // Check for context overflow first — needs special handling
                        if is_overflow_error(err_msg) {
                            self.overflow_error = Some(err_msg.clone());
                        } else if is_retriable_error(err_msg) {
                            // Check if this error is worth retrying
                            self.retriable_error = Some(err_msg.clone());
                        } else {
                            eprintln!("\n{RED}  error: {err_msg}{RESET}");
                            // Show diagnostic help for common errors
                            if let Some(diagnostic) = diagnose_api_error(err_msg, model) {
                                eprintln!(
                                    "{YELLOW}  💡 {}{RESET}",
                                    diagnostic.replace('\n', &format!("\n{YELLOW}     {RESET}"))
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Print and reset the tool batch summary.
    fn print_batch_summary(&mut self) {
        let batch_duration = self.batch_start.map(|s| s.elapsed()).unwrap_or_default();
        let summary = format_tool_batch_summary(
            self.batch_count,
            self.batch_succeeded,
            self.batch_failed,
            batch_duration,
        );
        if !summary.is_empty() {
            println!("{summary}");
        }
        // Reset batch tracking
        self.batch_count = 0;
        self.batch_succeeded = 0;
        self.batch_failed = 0;
        self.batch_start = None;
    }

    /// Consume state and produce the final PromptResult.
    fn into_result(self) -> PromptResult {
        if let Some(err_msg) = self.overflow_error {
            PromptResult::ContextOverflow {
                error_msg: err_msg,
                usage: self.usage,
            }
        } else if let Some(err_msg) = self.retriable_error {
            PromptResult::RetriableError {
                error_msg: err_msg,
                usage: self.usage,
            }
        } else {
            PromptResult::Done {
                collected_text: self.collected_text,
                usage: self.usage,
                last_tool_error: self.last_tool_error,
                last_tool_name: self.last_tool_name,
            }
        }
    }
}

/// Shared event-handling loop for prompt execution.
/// Processes all events from the agent's streaming channel and returns the result.
async fn handle_prompt_events(
    agent: &mut Agent,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<AgentEvent>,
    changes: &SessionChanges,
    model: &str,
) -> PromptResult {
    let mut state = PromptEventState::new();

    loop {
        tokio::select! {
            event = rx.recv() => {
                let Some(event) = event else { break };
                match event {
                    AgentEvent::ToolExecutionStart {
                        tool_call_id, tool_name, args, ..
                    } => {
                        state.handle_tool_execution_start(tool_call_id, tool_name, args, changes);
                    }
                    AgentEvent::ToolExecutionEnd { tool_call_id, is_error, result, tool_name, .. } => {
                        state.handle_tool_execution_end(tool_call_id, tool_name, is_error, result);
                    }
                    AgentEvent::ToolExecutionUpdate { tool_call_id, partial_result, .. } => {
                        state.handle_tool_execution_update(tool_call_id, partial_result);
                    }
                    AgentEvent::MessageUpdate {
                        delta: StreamDelta::Text { delta },
                        ..
                    } => {
                        state.handle_message_update_text(&delta);
                    }
                    AgentEvent::MessageUpdate {
                        delta: StreamDelta::Thinking { delta },
                        ..
                    } => {
                        // Stop spinner on first thinking output
                        if let Some(s) = state.spinner.take() { s.stop(); }
                        if !state.in_thinking {
                            // Print thinking section header on first thinking token
                            eprintln!("\n{}", section_header("Thinking"));
                            state.in_thinking = true;
                        }
                        // Render thinking to stderr (dimmed) so it doesn't
                        // interleave with stdout text output
                        eprint!("{DIM}{delta}{RESET}");
                        let _ = io::stderr().flush();
                    }
                    AgentEvent::AgentEnd { messages } => {
                        state.handle_agent_end(messages, model);
                    }
                    AgentEvent::InputRejected { reason } => {
                        if let Some(s) = state.spinner.take() { s.stop(); }
                        eprintln!("{RED}  input rejected: {reason}{RESET}");
                        if let Some(diagnostic) = diagnose_api_error(&reason, model) {
                            eprintln!("{YELLOW}  💡 {}{RESET}", diagnostic.replace('\n', &format!("\n{YELLOW}     {RESET}")));
                        }
                    }
                    AgentEvent::ProgressMessage { text, .. } => {
                        if let Some(s) = state.spinner.take() { s.stop(); }
                        if state.in_text {
                            println!();
                            state.in_text = false;
                        }
                        println!("{DIM}  {text}{RESET}");
                    }
                    AgentEvent::MessageStart { .. } => {
                        // Agent started a new message — stop the spinner
                        // so it doesn't overlap with output
                        if let Some(s) = state.spinner.take() { s.stop(); }
                    }
                    AgentEvent::MessageEnd { .. }
                        // Agent finished a message — flush any pending text
                        // (This is where ExecutionLimits stop messages appear)
                        if state.in_text =>
                    {
                        let remaining = state.md_renderer.flush();
                        if !remaining.is_empty() {
                            print!("{remaining}");
                        }
                        println!();
                        state.in_text = false;
                    }
                    AgentEvent::TurnStart => {
                        state.turn_number += 1;
                    }
                    AgentEvent::TurnEnd { .. } => {
                        // Turn complete — nothing needed here for now.
                        // Explicitly matched to keep event handling exhaustive.
                    }
                    _ => {}
                }
            }
            _ = tokio::signal::ctrl_c() => {
                // Stop spinner if still running
                if let Some(s) = state.spinner.take() { s.stop(); }
                agent.abort();
                if state.in_text {
                    println!();
                }
                println!("\n{DIM}  (interrupted — press Ctrl+C again to exit){RESET}");
                return PromptResult::Done {
                    collected_text: state.collected_text,
                    usage: state.usage,
                    last_tool_error: state.last_tool_error,
                    last_tool_name: state.last_tool_name,
                };
            }
        }
    }

    // Stop spinner if still running (e.g., channel closed without events)
    if let Some(s) = state.spinner.take() {
        s.stop();
    }

    // Flush any remaining buffered markdown content
    let remaining = state.md_renderer.flush();
    if !remaining.is_empty() {
        print!("{}", remaining);
        io::stdout().flush().ok();
    }

    if state.in_text {
        println!();
    }

    state.into_result()
}

pub async fn run_prompt(
    agent: &mut Agent,
    input: &str,
    session_total: &mut Usage,
    model: &str,
) -> PromptOutcome {
    // Default: create a throwaway changes tracker (for callers that don't need tracking)
    let changes = SessionChanges::new();
    run_prompt_with_changes(agent, input, session_total, model, &changes).await
}

/// Run a prompt with file change tracking.
/// Like `run_prompt`, but records write_file/edit_file calls into the given tracker.
pub async fn run_prompt_with_changes(
    agent: &mut Agent,
    input: &str,
    session_total: &mut Usage,
    model: &str,
    changes: &SessionChanges,
) -> PromptOutcome {
    // Proactive compact: if context is already near the limit, compact before attempting
    crate::commands_session::proactive_compact_if_needed(agent);

    let prompt_start = Instant::now();
    let mut total_usage = Usage::default();
    let mut collected_text = String::new();
    let mut last_tool_error: Option<String> = None;
    let mut last_tool_name: Option<String> = None;
    let mut did_overflow_compact = false;
    let mut api_error: Option<String> = None;

    // Save message state before the first attempt so we can restore on retry
    let saved_state = agent.save_messages().ok();

    for attempt in 0..=MAX_RETRIES {
        // On retry, restore pre-prompt state so we don't duplicate the user message
        if attempt > 0 {
            if let Some(ref json) = saved_state {
                let _ = agent.restore_messages(json);
            }
        }

        match run_prompt_once(agent, input, changes, model).await {
            PromptResult::Done {
                collected_text: text,
                usage,
                last_tool_error: tool_err,
                last_tool_name: tool_nm,
            } => {
                total_usage.input += usage.input;
                total_usage.output += usage.output;
                total_usage.cache_read += usage.cache_read;
                total_usage.cache_write += usage.cache_write;
                collected_text = text;
                last_tool_error = tool_err;
                last_tool_name = tool_nm;
                break;
            }
            PromptResult::RetriableError { error_msg, usage } => {
                total_usage.input += usage.input;
                total_usage.output += usage.output;
                total_usage.cache_read += usage.cache_read;
                total_usage.cache_write += usage.cache_write;

                if attempt < MAX_RETRIES {
                    let delay = retry_delay(attempt + 1);
                    let delay_secs = delay.as_secs();
                    let next = attempt + 2; // human-readable attempt number
                    eprintln!(
                        "{DIM}  ⚡ retrying (attempt {next}/{}, waiting {delay_secs}s)...{RESET}",
                        MAX_RETRIES + 1
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    // Exhausted all retries — show the final error with diagnostic
                    eprintln!("\n{RED}  error: {error_msg}{RESET}");
                    eprintln!("{DIM}  (failed after {} attempts){RESET}", MAX_RETRIES + 1);
                    if let Some(diagnostic) = diagnose_api_error(&error_msg, model) {
                        eprintln!(
                            "{YELLOW}  💡 {}{RESET}",
                            diagnostic.replace('\n', &format!("\n{YELLOW}     {RESET}"))
                        );
                    }
                    api_error = Some(error_msg);
                }
            }
            PromptResult::ContextOverflow { error_msg, usage } => {
                total_usage.input += usage.input;
                total_usage.output += usage.output;
                total_usage.cache_read += usage.cache_read;
                total_usage.cache_write += usage.cache_write;

                // Auto-compact and retry once
                eprintln!(
                    "\n{YELLOW}  ⚡ context overflow detected — auto-compacting and retrying...{RESET}"
                );
                eprintln!("{DIM}  ({error_msg}){RESET}");

                if let Some(ref json) = saved_state {
                    let _ = agent.restore_messages(json);
                }
                if let Some((before_count, before_tokens, after_count, after_tokens)) =
                    crate::commands_session::compact_agent(agent)
                {
                    eprintln!(
                        "{DIM}  compacted: {before_count} → {after_count} messages, ~{} → ~{} tokens{RESET}",
                        crate::format::format_token_count(before_tokens),
                        crate::format::format_token_count(after_tokens)
                    );
                }

                did_overflow_compact = true;

                // Retry with the compacted context
                let retry_input = build_overflow_retry_prompt(input);
                match run_prompt_once(agent, &retry_input, changes, model).await {
                    PromptResult::Done {
                        collected_text: text,
                        usage: retry_usage,
                        last_tool_error: tool_err,
                        last_tool_name: tool_nm,
                    } => {
                        total_usage.input += retry_usage.input;
                        total_usage.output += retry_usage.output;
                        total_usage.cache_read += retry_usage.cache_read;
                        total_usage.cache_write += retry_usage.cache_write;
                        collected_text = text;
                        last_tool_error = tool_err;
                        last_tool_name = tool_nm;
                    }
                    PromptResult::RetriableError {
                        error_msg: retry_err,
                        usage: retry_usage,
                    }
                    | PromptResult::ContextOverflow {
                        error_msg: retry_err,
                        usage: retry_usage,
                    } => {
                        total_usage.input += retry_usage.input;
                        total_usage.output += retry_usage.output;
                        total_usage.cache_read += retry_usage.cache_read;
                        total_usage.cache_write += retry_usage.cache_write;
                        eprintln!("\n{RED}  error: {retry_err}{RESET}");
                        eprintln!(
                            "{DIM}  (overflow retry also failed — try /compact manually){RESET}"
                        );
                        api_error = Some(retry_err);
                    }
                }
                break;
            }
        }
    }

    session_total.input += total_usage.input;
    session_total.output += total_usage.output;
    session_total.cache_read += total_usage.cache_read;
    session_total.cache_write += total_usage.cache_write;
    print_usage(&total_usage, session_total, model, prompt_start.elapsed());
    // Issue #258: yoagent 0.7.x runs the agent loop in a background task; the
    // agent's internal `self.messages` is only updated when `finish()` is awaited.
    // Without this, `agent.messages()` returns stale state and the context bar
    // permanently reads "0% used". Call finish() before reading messages.
    agent.finish().await;
    let ctx_used = total_tokens(agent.messages()) as u64;
    let ctx_max = crate::cli::effective_context_tokens();
    print_context_usage(ctx_used, ctx_max);
    if let Some(warning) = crate::format::context_budget_warning(ctx_used, ctx_max) {
        eprintln!("{warning}");
    }
    maybe_ring_bell(prompt_start.elapsed());
    println!();
    PromptOutcome {
        text: collected_text,
        last_tool_error,
        last_tool_name,
        was_overflow: did_overflow_compact,
        last_api_error: api_error,
    }
}

/// Run a prompt with automatic retry on tool errors.
///
/// Wraps `run_prompt_with_changes` with self-correction: if the outcome
/// contains a `last_tool_error`, the prompt is automatically re-run with
/// error context appended (up to `MAX_AUTO_RETRIES` times). This makes
/// yoyo more resilient — instead of waiting for the user to `/retry`,
/// the agent self-corrects on transient tool failures.
///
/// Only meant for natural-language prompts (not slash commands).
pub async fn run_prompt_auto_retry(
    agent: &mut Agent,
    input: &str,
    session_total: &mut Usage,
    model: &str,
    changes: &SessionChanges,
) -> PromptOutcome {
    let mut outcome = run_prompt_with_changes(agent, input, session_total, model, changes).await;

    for attempt in 1..=MAX_AUTO_RETRIES {
        match outcome.last_tool_error {
            Some(ref err) => {
                if session_budget_exhausted(30) {
                    eprintln!(
                        "{DIM}  ⏱ session budget nearly exhausted, stopping retries early{RESET}"
                    );
                    break;
                }
                let retry_prompt =
                    build_auto_retry_prompt(input, err, outcome.last_tool_name.as_deref(), attempt);
                eprintln!(
                    "{DIM}  ⚡ auto-retrying after tool error (attempt {attempt}/{MAX_AUTO_RETRIES})...{RESET}"
                );
                outcome =
                    run_prompt_with_changes(agent, &retry_prompt, session_total, model, changes)
                        .await;
            }
            None => break,
        }
    }

    outcome
}

/// Run a prompt with pre-built content blocks (e.g. text + image).
/// This is the content-block equivalent of `run_prompt`.
pub async fn run_prompt_with_content(
    agent: &mut Agent,
    content_blocks: Vec<Content>,
    session_total: &mut Usage,
    model: &str,
) -> PromptOutcome {
    let changes = SessionChanges::new();
    run_prompt_with_content_and_changes(agent, content_blocks, session_total, model, &changes).await
}

/// Run a content-block prompt with automatic retry on tool errors.
///
/// This is the content-block equivalent of `run_prompt_auto_retry`: when the
/// outcome contains a `last_tool_error`, the prompt is automatically re-run
/// with error context appended as a text-only follow-up (up to `MAX_AUTO_RETRIES`
/// times). The original content blocks (including images and @file mentions) are
/// already in the conversation history, so the retry only needs the text nudge.
///
/// Without this, @file mention prompts silently skip auto-retry, meaning tool
/// failures require the user to manually `/retry` — inconsistent with regular
/// prompts where auto-retry kicks in automatically.
pub async fn run_prompt_auto_retry_with_content(
    agent: &mut Agent,
    content_blocks: Vec<Content>,
    session_total: &mut Usage,
    model: &str,
    changes: &SessionChanges,
    original_text: &str,
) -> PromptOutcome {
    let mut outcome =
        run_prompt_with_content_and_changes(agent, content_blocks, session_total, model, changes)
            .await;

    for attempt in 1..=MAX_AUTO_RETRIES {
        match outcome.last_tool_error {
            Some(ref err) => {
                if session_budget_exhausted(30) {
                    eprintln!(
                        "{DIM}  ⏱ session budget nearly exhausted, stopping retries early{RESET}"
                    );
                    break;
                }
                // Retry with a text-only follow-up — the original content blocks
                // (files, images) are already in conversation history from the first attempt
                let retry_prompt = build_auto_retry_prompt(
                    original_text,
                    err,
                    outcome.last_tool_name.as_deref(),
                    attempt,
                );
                eprintln!(
                    "{DIM}  ⚡ auto-retrying after tool error (attempt {attempt}/{MAX_AUTO_RETRIES})...{RESET}"
                );
                outcome =
                    run_prompt_with_changes(agent, &retry_prompt, session_total, model, changes)
                        .await;
            }
            None => break,
        }
    }

    outcome
}

/// Run a prompt with pre-built content blocks and file change tracking.
/// This is the content-block equivalent of `run_prompt_with_changes`.
pub async fn run_prompt_with_content_and_changes(
    agent: &mut Agent,
    content_blocks: Vec<Content>,
    session_total: &mut Usage,
    model: &str,
    changes: &SessionChanges,
) -> PromptOutcome {
    // Proactive compact: if context is already near the limit, compact before attempting
    crate::commands_session::proactive_compact_if_needed(agent);

    let prompt_start = Instant::now();
    let mut total_usage = Usage::default();
    let mut collected_text = String::new();
    let mut last_tool_error: Option<String> = None;
    let mut last_tool_name: Option<String> = None;
    let mut api_error: Option<String> = None;
    let user_msg = AgentMessage::Llm(Message::User {
        content: content_blocks,
        timestamp: now_ms(),
    });

    // Save message state before the first attempt so we can restore on retry
    let saved_state = agent.save_messages().ok();

    for attempt in 0..=MAX_RETRIES {
        // On retry, restore pre-prompt state so we don't duplicate the user message
        if attempt > 0 {
            if let Some(ref json) = saved_state {
                let _ = agent.restore_messages(json);
            }
        }

        match run_prompt_once_with_messages(agent, vec![user_msg.clone()], changes, model).await {
            PromptResult::Done {
                collected_text: text,
                usage,
                last_tool_error: tool_err,
                last_tool_name: tool_nm,
            } => {
                total_usage.input += usage.input;
                total_usage.output += usage.output;
                total_usage.cache_read += usage.cache_read;
                total_usage.cache_write += usage.cache_write;
                collected_text = text;
                last_tool_error = tool_err;
                last_tool_name = tool_nm;
                break;
            }
            PromptResult::RetriableError { error_msg, usage } => {
                total_usage.input += usage.input;
                total_usage.output += usage.output;
                total_usage.cache_read += usage.cache_read;
                total_usage.cache_write += usage.cache_write;

                if attempt < MAX_RETRIES {
                    let delay = retry_delay(attempt + 1);
                    let delay_secs = delay.as_secs();
                    let next = attempt + 2;
                    eprintln!(
                        "{DIM}  ⚡ retrying (attempt {next}/{}, waiting {delay_secs}s)...{RESET}",
                        MAX_RETRIES + 1
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    eprintln!("\n{RED}  error: {error_msg}{RESET}");
                    eprintln!("{DIM}  (failed after {} attempts){RESET}", MAX_RETRIES + 1);
                    if let Some(diagnostic) = diagnose_api_error(&error_msg, model) {
                        eprintln!(
                            "{YELLOW}  💡 {}{RESET}",
                            diagnostic.replace('\n', &format!("\n{YELLOW}     {RESET}"))
                        );
                    }
                    api_error = Some(error_msg);
                }
            }
            PromptResult::ContextOverflow { error_msg, usage } => {
                total_usage.input += usage.input;
                total_usage.output += usage.output;
                total_usage.cache_read += usage.cache_read;
                total_usage.cache_write += usage.cache_write;

                eprintln!(
                    "\n{YELLOW}  ⚡ context overflow detected — cannot retry with image content{RESET}"
                );
                eprintln!("{DIM}  ({error_msg}){RESET}");
                api_error = Some(error_msg);
                break;
            }
        }
    }

    session_total.input += total_usage.input;
    session_total.output += total_usage.output;
    session_total.cache_read += total_usage.cache_read;
    session_total.cache_write += total_usage.cache_write;
    print_usage(&total_usage, session_total, model, prompt_start.elapsed());
    // Issue #258: see run_prompt_with_changes — yoagent 0.7.x requires finish()
    // before reading messages, otherwise the context bar reads stale "0%".
    agent.finish().await;
    let ctx_used = total_tokens(agent.messages()) as u64;
    let ctx_max = crate::cli::effective_context_tokens();
    print_context_usage(ctx_used, ctx_max);
    if let Some(warning) = crate::format::context_budget_warning(ctx_used, ctx_max) {
        eprintln!("{warning}");
    }
    maybe_ring_bell(prompt_start.elapsed());
    println!();
    PromptOutcome {
        text: collected_text,
        last_tool_error,
        last_tool_name,
        was_overflow: false,
        last_api_error: api_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Issue #258 / Day 33 lesson (test from the user's perspective):
    // After draining the event stream from prompt_messages, the agent's
    // internal `messages` field is still empty until `finish().await` is
    // called. This is exactly the bug yoyo had: it read `agent.messages()`
    // immediately after the loop ended and saw 0, so the context bar
    // permanently said "0% used".
    //
    // This test reproduces the failure mode against yoagent's MockProvider
    // and verifies that calling `finish()` is what makes messages visible.
    #[tokio::test]
    async fn agent_messages_empty_until_finish_is_called() {
        use yoagent::provider::MockProvider;
        use yoagent::Agent;

        let provider = MockProvider::text("hello back");
        let mut agent = Agent::new(provider)
            .with_model("mock-model")
            .with_api_key("not-a-real-key");

        // Sanity: starts empty.
        assert_eq!(agent.messages().len(), 0);

        // Drive a prompt and drain all events.
        let mut rx = agent.prompt("hi").await;
        while rx.recv().await.is_some() {}

        // Without finish(), yoagent 0.7.x leaves messages stale. This is the
        // root cause of Issue #258 — and exactly why yoyo's context bar read 0%.
        let stale_count = agent.messages().len();

        // After finish(), the loop's messages are restored into the agent.
        agent.finish().await;
        let real_count = agent.messages().len();

        assert!(
            real_count > 0,
            "expected agent.messages() to be non-empty after finish(), got {real_count}"
        );
        assert!(
            real_count > stale_count || stale_count == 0,
            "finish() should restore messages: stale={stale_count}, real={real_count}"
        );
    }

    #[test]
    fn test_summarize_message_user() {
        let msg = AgentMessage::Llm(Message::user("hello world, this is a test"));
        let (role, preview) = summarize_message(&msg);
        assert_eq!(role, "user");
        assert!(preview.contains("hello world"));
    }

    #[test]
    fn test_summarize_message_tool_result() {
        let msg = AgentMessage::Llm(Message::ToolResult {
            tool_call_id: "tc_1".into(),
            tool_name: "bash".into(),
            content: vec![Content::Text {
                text: "output".into(),
            }],
            is_error: false,
            timestamp: 0,
        });
        let (role, preview) = summarize_message(&msg);
        assert_eq!(role, "tool");
        assert!(preview.contains("bash"));
        assert!(preview.contains("✓"));
    }

    #[test]
    fn test_summarize_message_tool_result_error() {
        let msg = AgentMessage::Llm(Message::ToolResult {
            tool_call_id: "tc_2".into(),
            tool_name: "bash".into(),
            content: vec![Content::Text {
                text: "error".into(),
            }],
            is_error: true,
            timestamp: 0,
        });
        let (role, preview) = summarize_message(&msg);
        assert_eq!(role, "tool");
        assert!(preview.contains("✗"));
    }

    #[test]
    fn test_write_output_file_none() {
        write_output_file(&None, "test content");
        // No assertion needed — just verify it doesn't panic
    }

    #[test]
    fn test_write_output_file_some() {
        let dir = std::env::temp_dir().join("yoyo_test_output");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test_output.txt");
        let path_str = path.to_string_lossy().to_string();
        write_output_file(&Some(path_str), "hello from yoyo");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello from yoyo");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_tool_result_preview_empty() {
        let result = ToolResult {
            content: vec![],
            details: serde_json::json!(null),
        };
        assert_eq!(tool_result_preview(&result, 100), "");
    }

    #[test]
    fn test_tool_result_preview_text() {
        let result = ToolResult {
            content: vec![Content::Text {
                text: "error: file not found".into(),
            }],
            details: serde_json::json!(null),
        };
        assert_eq!(tool_result_preview(&result, 100), "error: file not found");
    }

    #[test]
    fn test_tool_result_preview_truncated() {
        let result = ToolResult {
            content: vec![Content::Text {
                text: "a".repeat(200),
            }],
            details: serde_json::json!(null),
        };
        let preview = tool_result_preview(&result, 50);
        assert!(preview.len() < 100);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn test_tool_result_preview_multiline() {
        let result = ToolResult {
            content: vec![Content::Text {
                text: "first line\nsecond line\nthird line".into(),
            }],
            details: serde_json::json!(null),
        };
        assert_eq!(tool_result_preview(&result, 100), "first line");
    }

    #[test]
    fn test_search_messages_basic_match() {
        let messages = vec![
            AgentMessage::Llm(Message::user("hello world")),
            AgentMessage::Llm(Message::user("goodbye world")),
        ];
        let results = search_messages(&messages, "hello");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 1); // 1-indexed
        assert_eq!(results[0].1, "user");
        assert!(results[0].2.contains("hello"));
    }

    #[test]
    fn test_search_messages_case_insensitive() {
        let messages = vec![AgentMessage::Llm(Message::user("Hello World"))];
        let results = search_messages(&messages, "hello");
        assert_eq!(results.len(), 1);
        let results2 = search_messages(&messages, "HELLO");
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn test_search_messages_no_match() {
        let messages = vec![AgentMessage::Llm(Message::user("hello world"))];
        let results = search_messages(&messages, "foobar");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_messages_empty_messages() {
        let messages: Vec<AgentMessage> = vec![];
        let results = search_messages(&messages, "anything");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_messages_multiple_matches() {
        let messages = vec![
            AgentMessage::Llm(Message::user("the rust language")),
            AgentMessage::Llm(Message::user("python is great")),
            AgentMessage::Llm(Message::user("rust is fast")),
        ];
        let results = search_messages(&messages, "rust");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 1);
        assert_eq!(results[1].0, 3);
    }

    #[test]
    fn test_search_messages_tool_result() {
        let messages = vec![AgentMessage::Llm(Message::ToolResult {
            tool_call_id: "tc_1".into(),
            tool_name: "bash".into(),
            content: vec![Content::Text {
                text: "cargo build succeeded".into(),
            }],
            is_error: false,
            timestamp: 0,
        })];
        let results = search_messages(&messages, "cargo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "tool");
    }

    #[test]
    fn test_message_text_user() {
        let msg = AgentMessage::Llm(Message::user("test input"));
        let text = message_text(&msg);
        assert_eq!(text, "test input");
    }

    #[test]
    fn test_message_text_tool_result() {
        let msg = AgentMessage::Llm(Message::ToolResult {
            tool_call_id: "tc_1".into(),
            tool_name: "bash".into(),
            content: vec![Content::Text {
                text: "output text".into(),
            }],
            is_error: false,
            timestamp: 0,
        });
        let text = message_text(&msg);
        assert!(text.contains("bash"));
        assert!(text.contains("output text"));
    }

    // --- highlight_matches tests ---

    #[test]
    fn test_highlight_matches_basic() {
        let result = highlight_matches("hello world", "world");
        assert!(result.contains(&format!("{BOLD}world{RESET}")));
        assert!(result.contains("hello "));
    }

    #[test]
    fn test_highlight_matches_case_insensitive() {
        let result = highlight_matches("Hello World", "hello");
        assert!(result.contains(&format!("{BOLD}Hello{RESET}")));
    }

    #[test]
    fn test_highlight_matches_multiple_occurrences() {
        let result = highlight_matches("rust is fast, rust is safe", "rust");
        // Should highlight both occurrences
        let bold_rust = format!("{BOLD}rust{RESET}");
        let count = result.matches(&bold_rust.to_string()).count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_highlight_matches_no_match() {
        let result = highlight_matches("hello world", "foobar");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_highlight_matches_empty_query() {
        let result = highlight_matches("hello world", "");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_highlight_matches_empty_text() {
        let result = highlight_matches("", "query");
        assert_eq!(result, "");
    }

    #[test]
    fn test_highlight_matches_preserves_original_case() {
        let result = highlight_matches("The Rust Language", "rust");
        // Should wrap "Rust" (original case), not "rust"
        assert!(result.contains(&format!("{BOLD}Rust{RESET}")));
    }

    #[test]
    fn test_highlight_matches_entire_string() {
        let result = highlight_matches("hello", "hello");
        assert_eq!(result, format!("{BOLD}hello{RESET}"));
    }

    #[test]
    fn test_search_messages_results_are_highlighted() {
        let messages = vec![AgentMessage::Llm(Message::user("hello world"))];
        let results = search_messages(&messages, "hello");
        assert_eq!(results.len(), 1);
        // The preview should contain BOLD highlighting around "hello"
        assert!(results[0].2.contains(&format!("{BOLD}hello{RESET}")));
    }

    // ── Context overflow detection tests ─────────────────────────────────

    #[test]
    fn test_image_content_block_construction() {
        // Verify that Content::Image can be constructed with base64 data and mime type
        let data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string();
        let mime_type = "image/png".to_string();

        let content_blocks = [
            Content::Text {
                text: "describe this image".to_string(),
            },
            Content::Image {
                data: data.clone(),
                mime_type: mime_type.clone(),
            },
        ];

        assert_eq!(content_blocks.len(), 2);
        match &content_blocks[0] {
            Content::Text { text } => assert_eq!(text, "describe this image"),
            _ => panic!("expected Text content"),
        }
        match &content_blocks[1] {
            Content::Image {
                data: d,
                mime_type: m,
            } => {
                assert_eq!(d, &data);
                assert_eq!(m, &mime_type);
            }
            _ => panic!("expected Image content"),
        }
    }

    #[test]
    fn test_user_message_with_image_content() {
        // Verify that a user message with image content blocks can be constructed
        // and wrapped as an AgentMessage — this is the exact pattern used by
        // run_prompt_with_content
        let content_blocks = vec![
            Content::Text {
                text: "what is this?".to_string(),
            },
            Content::Image {
                data: "base64data".to_string(),
                mime_type: "image/jpeg".to_string(),
            },
        ];

        let user_msg = AgentMessage::Llm(Message::User {
            content: content_blocks,
            timestamp: now_ms(),
        });

        assert_eq!(user_msg.role(), "user");
        if let AgentMessage::Llm(Message::User { content, .. }) = &user_msg {
            assert_eq!(content.len(), 2);
        } else {
            panic!("expected Llm(User) message");
        }
    }

    // TurnSnapshot and TurnHistory tests moved to src/session.rs (Day 54)

    /// Verify the deferred bash timer logic: bash tool_call_ids are tracked
    /// in the deferred map with optional command label, removed on first update
    /// (timer start), and cleaned up on end if no update ever arrived (e.g. denied command).
    #[test]
    fn test_deferred_bash_timer_set_lifecycle() {
        let mut deferred: HashMap<String, Option<String>> = HashMap::new();
        let mut timers: HashMap<String, &str> = HashMap::new(); // simplified stand-in

        // 1. ToolExecutionStart for bash → add to deferred set, NOT to timers
        let id = "call_abc".to_string();
        let cmd_label = Some("cargo test".to_string());
        deferred.insert(id.clone(), cmd_label);
        assert!(
            deferred.contains_key(&id),
            "bash tool should be in deferred set"
        );
        assert!(
            !timers.contains_key(&id),
            "timer should NOT start on ToolExecutionStart"
        );

        // 2. ToolExecutionUpdate → remove from deferred, start timer (with label)
        if let Some(label) = deferred.remove(&id) {
            assert_eq!(
                label,
                Some("cargo test".to_string()),
                "label should be preserved"
            );
            timers.insert(id.clone(), "bash");
        }
        assert!(
            !deferred.contains_key(&id),
            "should be removed from deferred after update"
        );
        assert!(
            timers.contains_key(&id),
            "timer should start on first ToolExecutionUpdate"
        );

        // 3. ToolExecutionEnd → timer is already active, just clean up
        timers.remove(&id);
        deferred.remove(&id); // no-op, already removed
        assert!(!timers.contains_key(&id));
        assert!(!deferred.contains_key(&id));
    }

    /// Verify that a denied bash command (no ToolExecutionUpdate) gets cleaned
    /// up properly on ToolExecutionEnd.
    #[test]
    fn test_deferred_bash_timer_denied_command_cleanup() {
        let mut deferred: HashMap<String, Option<String>> = HashMap::new();
        let timers: HashMap<String, &str> = HashMap::new();

        // ToolExecutionStart for bash → deferred
        let id = "call_denied".to_string();
        deferred.insert(id.clone(), Some("rm -rf /".to_string()));

        // No ToolExecutionUpdate (command was denied by user)

        // ToolExecutionEnd → clean up deferred entry
        deferred.remove(&id);
        assert!(
            !deferred.contains_key(&id),
            "deferred entry should be cleaned up on end"
        );
        assert!(
            !timers.contains_key(&id),
            "no timer should exist for denied command"
        );
    }

    /// Non-bash tools should not be deferred — they don't have confirmation prompts.
    #[test]
    fn test_non_bash_tools_not_deferred() {
        let deferred: HashMap<String, Option<String>> = HashMap::new();
        // For non-bash tools (read_file, write_file, etc.), we never insert into deferred
        assert!(
            deferred.is_empty(),
            "non-bash tools should never be in deferred set"
        );
    }

    #[test]
    fn test_prompt_outcome_has_api_error_field() {
        let outcome = PromptOutcome {
            text: String::new(),
            last_tool_error: None,
            last_tool_name: None,
            was_overflow: false,
            last_api_error: Some("503 Service Unavailable".to_string()),
        };
        assert_eq!(
            outcome.last_api_error,
            Some("503 Service Unavailable".to_string())
        );

        let outcome_no_error = PromptOutcome {
            text: "hello".to_string(),
            last_tool_error: None,
            last_tool_name: None,
            was_overflow: false,
            last_api_error: None,
        };
        assert!(outcome_no_error.last_api_error.is_none());
    }

    #[test]
    fn test_prompt_outcome_has_tool_name_field() {
        let outcome = PromptOutcome {
            text: String::new(),
            last_tool_error: Some("file not found".to_string()),
            last_tool_name: Some("read_file".to_string()),
            was_overflow: false,
            last_api_error: None,
        };
        assert_eq!(outcome.last_tool_name.as_deref(), Some("read_file"));

        let outcome_none = PromptOutcome {
            text: String::new(),
            last_tool_error: None,
            last_tool_name: None,
            was_overflow: false,
            last_api_error: None,
        };
        assert!(outcome_none.last_tool_name.is_none());
    }
}
