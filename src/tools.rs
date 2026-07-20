//! Tool definitions for the yoyo agent.
//!
//! Contains concrete tool implementations and builder functions:
//! - `StreamingBashTool` — real-time subprocess output
//! - `RenameSymbolTool` — cross-file symbol renaming
//! - `AskUserTool` — interactive question-asking
//! - `TodoTool` — task list management
//! - `build_tools` — assembles the complete tool set
//! - `build_sub_agent_tool` — creates a sub-agent with inherited config
//!
//! Tool decorator types (GuardedTool, TruncatingTool, ConfirmTool, ArcGuardedTool)
//! live in `tool_wrappers`.

use crate::cli;
use crate::commands_project;
use crate::commands_todo;
use crate::commands_web;
use crate::format::*;
use crate::hooks::{self, maybe_hook, AuditHook, HookRegistry};
use crate::safety::analyze_bash_command;
use crate::smart_edit::with_smart_edit;
use crate::tool_wrappers::{
    maybe_confirm, maybe_guard, maybe_guard_arc, with_auto_check, with_lite_description,
    with_recovery_hints, with_truncation, ToolFailureTracker,
};
use crate::AgentConfig;

use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use yoagent::provider::{
    AnthropicProvider, BedrockProvider, GoogleProvider, OpenAiCompatProvider, StreamProvider,
};
use yoagent::sub_agent::SubAgentTool;
use yoagent::tools::bash::ConfirmFn;
use yoagent::tools::edit::EditFileTool;
use yoagent::tools::file::{ReadFileTool, WriteFileTool};
use yoagent::tools::list::ListFilesTool;
use yoagent::types::AgentTool;
use yoagent::SharedState;

use crate::rtk::maybe_prefix_rtk;

// ---------------------------------------------------------------------------
// StreamingBashTool — real-time subprocess output via on_update and on_progress callbacks
// ---------------------------------------------------------------------------

/// Execute shell commands with real-time streaming output.
///
/// Unlike the upstream `BashTool` which waits for the process to finish before
/// returning output, `StreamingBashTool` reads stdout/stderr line-by-line and
/// calls `ctx.on_update()` periodically so the UI can display partial output
/// as the command runs. This is the difference between staring at a blank screen
/// during `cargo build` and watching compilation progress live.
///
/// Additionally, each individual line is emitted in real-time via `ctx.on_progress()`
/// (when available and in interactive mode), producing `AgentEvent::ProgressMessage`
/// events that the renderer displays immediately. Stderr lines are prefixed with
/// `stderr: ` so the user can distinguish them from stdout.
///
/// Streaming updates are sent every `update_interval` or every `lines_per_update`
/// lines, whichever comes first.
pub struct StreamingBashTool {
    /// Working directory for commands
    pub cwd: Option<String>,
    /// Max execution time per command
    pub timeout: Duration,
    /// Max output bytes to capture (prevents OOM on huge outputs)
    pub max_output_bytes: usize,
    /// Commands/patterns that are always blocked (e.g., "rm -rf /")
    pub deny_patterns: Vec<String>,
    /// Optional callback for confirming dangerous commands
    pub confirm_fn: Option<ConfirmFn>,
    /// How often to emit streaming updates
    pub update_interval: Duration,
    /// Emit an update after this many new lines (even if interval hasn't elapsed)
    pub lines_per_update: usize,
    /// When true (default), real-time progress via `on_progress` is only emitted
    /// when stderr is a terminal (interactive mode). Set to false in tests to
    /// allow progress emission regardless of TTY state.
    pub progress_requires_tty: bool,
}

impl Default for StreamingBashTool {
    fn default() -> Self {
        Self {
            cwd: None,
            timeout: Duration::from_secs(crate::cli_config::DEFAULT_BASH_TIMEOUT_SECS),
            max_output_bytes: 256 * 1024, // 256KB
            deny_patterns: vec![
                "rm -rf /".into(),
                "rm -rf /*".into(),
                "mkfs".into(),
                "dd if=".into(),
                ":(){:|:&};:".into(), // fork bomb
            ],
            confirm_fn: None,
            update_interval: Duration::from_millis(500),
            lines_per_update: 20,
            progress_requires_tty: true,
        }
    }
}

impl StreamingBashTool {
    pub fn with_confirm(mut self, f: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        self.confirm_fn = Some(Box::new(f));
        self
    }
}

/// Safer local file search for coding agents.
///
/// The upstream yoagent search tool falls back to broad recursive grep. In CI
/// that can scan `target/` binary artifacts and treats every pattern as regex,
/// producing avoidable tool failures for ordinary code-inspection turns.
pub(crate) struct ProjectSearchTool {
    pub root: Option<String>,
    pub max_results: usize,
    pub timeout: Duration,
}

impl Default for ProjectSearchTool {
    fn default() -> Self {
        Self {
            root: None,
            max_results: 50,
            timeout: Duration::from_secs(30),
        }
    }
}

#[async_trait::async_trait]
impl AgentTool for ProjectSearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn label(&self) -> &str {
        "Search Files"
    }

    fn description(&self) -> &str {
        "Search project text files for a pattern. Literal search is the default so symbols and parentheses do not need escaping. Set regex=true only when regex semantics are required. Build artifacts and dependency caches are skipped."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Search text. Interpreted literally unless regex=true."
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in (optional, defaults to working directory)"
                },
                "include": {
                    "type": "string",
                    "description": "File glob pattern to include, e.g. '*.rs' (optional)"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Case sensitive search (default: false)"
                },
                "regex": {
                    "type": "boolean",
                    "description": "Treat pattern as a regular expression (default: false)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        use yoagent::types::{Content, ToolError, ToolResult as TR};

        let cancel = ctx.cancel;
        let pattern = params["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'pattern' parameter".into()))?;
        let search_path = params["path"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| self.root.clone())
            .unwrap_or_else(|| ".".to_string());
        let include = params["include"].as_str();
        let case_sensitive = params["case_sensitive"].as_bool().unwrap_or(false);
        let regex = params["regex"].as_bool().unwrap_or(false);

        if cancel.is_cancelled() {
            return Err(ToolError::Cancelled);
        }

        let (cmd_name, args) = if command_exists("rg") {
            build_project_rg_args(
                pattern,
                &search_path,
                include,
                case_sensitive,
                regex,
                self.max_results,
            )
        } else {
            build_project_grep_args(
                pattern,
                &search_path,
                include,
                case_sensitive,
                regex,
                self.max_results,
            )
        };

        let mut cmd = tokio::process::Command::new(&cmd_name);
        cmd.args(&args);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let result = tokio::select! {
            _ = cancel.cancelled() => {
                return Err(ToolError::Cancelled);
            }
            _ = tokio::time::sleep(self.timeout) => {
                return Err(ToolError::Failed("Search timed out".into()));
            }
            result = cmd.output() => {
                result.map_err(|e| ToolError::Failed(format!("Search failed: {e}")))?
            }
        };

        let stdout = String::from_utf8_lossy(&result.stdout).to_string();
        let stderr = String::from_utf8_lossy(&result.stderr).to_string();
        let code = result.status.code();

        if code != Some(0) && code != Some(1) {
            let stderr_lower = stderr.to_lowercase();
            let is_regex_error = regex
                && (stderr_lower.contains("unmatched")
                    || stderr_lower.contains("invalid")
                    || stderr_lower.contains("regex parse")
                    || stderr_lower.contains("regex syntax")
                    || stderr_lower.contains("regex engine")
                    || stderr_lower.contains("unclosed")
                    || stderr_lower.contains("empty pattern")
                    || stderr_lower.contains("repetition"));
            let mut error_msg = format!("Search error: {}", stderr.trim());
            if is_regex_error {
                error_msg.push_str(
                    " Hint: try regex=false for literal search, or escape regex metacharacters with \\.",
                );
            }
            return Err(ToolError::Failed(error_msg));
        }

        if stdout.trim().is_empty() {
            return Ok(TR {
                content: vec![Content::Text {
                    text: format!("No matches found for '{pattern}'"),
                }],
                details: serde_json::json!({ "matches": 0 }),
            });
        }

        let lines: Vec<&str> = stdout.lines().collect();
        let shown = lines
            .iter()
            .take(self.max_results)
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        let text = if lines.len() > self.max_results {
            format!("{shown}\n... (showing first {} matches)", self.max_results)
        } else {
            format!("{shown}\n({} matches)", lines.len())
        };

        Ok(TR {
            content: vec![Content::Text { text }],
            details: serde_json::json!({
                "matches": lines.len(),
                "literal": !regex,
            }),
        })
    }
}

fn command_exists(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn build_project_rg_args(
    pattern: &str,
    path: &str,
    include: Option<&str>,
    case_sensitive: bool,
    regex: bool,
    max_results: usize,
) -> (String, Vec<String>) {
    let mut args = vec![
        "--line-number".to_string(),
        "--no-heading".to_string(),
        "--with-filename".to_string(),
        "--color=never".to_string(),
        format!("--max-count={max_results}"),
    ];
    if !case_sensitive {
        args.push("--ignore-case".to_string());
    }
    if !regex {
        args.push("--fixed-strings".to_string());
    }
    for glob in [
        "!**/target/**",
        "!**/.git/**",
        "!**/.yoyo/**",
        "!**/node_modules/**",
        "!**/.venv/**",
        "!**/__pycache__/**",
    ] {
        args.push("--glob".to_string());
        args.push(glob.to_string());
    }
    if let Some(glob) = include {
        args.push("--glob".to_string());
        args.push(glob.to_string());
    }
    args.push("--".to_string());
    args.push(pattern.to_string());
    args.push(path.to_string());
    ("rg".to_string(), args)
}

fn build_project_grep_args(
    pattern: &str,
    path: &str,
    include: Option<&str>,
    case_sensitive: bool,
    regex: bool,
    max_results: usize,
) -> (String, Vec<String>) {
    let mut args = vec![
        "-r".to_string(),
        "-n".to_string(),
        "-H".to_string(),
        "-I".to_string(),
        "--color=never".to_string(),
        format!("-m{max_results}"),
    ];
    if !case_sensitive {
        args.push("-i".to_string());
    }
    if !regex {
        args.push("-F".to_string());
    }
    if let Some(glob) = include {
        args.push(format!("--include={glob}"));
    }
    for dir in [
        "target",
        ".git",
        ".yoyo",
        "node_modules",
        "__pycache__",
        ".venv",
    ] {
        args.push(format!("--exclude-dir={dir}"));
    }
    args.push("--".to_string());
    args.push(pattern.to_string());
    args.push(path.to_string());
    ("grep".to_string(), args)
}

/// Emit a streaming update with the accumulated output so far.
fn emit_update(ctx: &yoagent::types::ToolContext, output: &str) {
    if let Some(ref on_update) = ctx.on_update {
        on_update(yoagent::types::ToolResult {
            content: vec![yoagent::types::Content::Text {
                text: output.to_string(),
            }],
            details: serde_json::json!({"streaming": true}),
        });
    }
}

#[async_trait::async_trait]
impl AgentTool for StreamingBashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn label(&self) -> &str {
        "Execute Command"
    }

    fn description(&self) -> &str {
        "Execute a bash command and return stdout/stderr. Use for running scripts, installing packages, checking system state, etc. Supports an optional timeout parameter (in seconds, default: 300, max: 600) for long-running commands."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The bash command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Maximum seconds to wait for command (default: 300, max: 600)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        use tokio::io::AsyncBufReadExt;
        use yoagent::types::{Content, ToolError, ToolResult as TR};

        let cancel = ctx.cancel.clone();
        let command = params["command"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'command' parameter".into()))?;

        // Check deny patterns (hard block — always denied, no override)
        for pattern in &self.deny_patterns {
            if command.contains(pattern.as_str()) {
                return Err(ToolError::Failed(format!(
                    "Command blocked by safety policy: contains '{}'. This pattern is denied for safety.",
                    pattern
                )));
            }
        }

        // Safety analysis — soft warning that routes through confirmation
        if let Some(warning) = analyze_bash_command(command) {
            if let Some(ref confirm) = self.confirm_fn {
                if !confirm(&format!("⚠️  {warning}\nCommand: {command}")) {
                    return Err(ToolError::Failed(
                        "Command was not confirmed by the user.".into(),
                    ));
                }
                // User confirmed the dangerous command — skip the normal confirm below
                // by proceeding directly to execution
            } else {
                return Err(ToolError::Failed(format!(
                    "Command requires explicit approval: {warning}"
                )));
            }
        } else {
            // No safety warning — check normal confirmation callback
            if let Some(ref confirm) = self.confirm_fn {
                if !confirm(command) {
                    return Err(ToolError::Failed(
                        "Command was not confirmed by the user.".into(),
                    ));
                }
            }
        }

        // Apply RTK prefix for supported commands
        let effective_command = maybe_prefix_rtk(command);

        // Prepend pipefail so that non-zero exits in piped commands
        // propagate to the tool result instead of being silently masked
        // by the last command's exit code.
        let guarded_command = format!("set -o pipefail; {}", effective_command);

        let base_timeout = if let Some(t) = params.get("timeout").and_then(|v| v.as_u64()) {
            Duration::from_secs(t.clamp(1, 600))
        } else {
            self.timeout
        };
        let max_bytes = self.max_output_bytes;
        let update_interval = self.update_interval;
        let lines_per_update = self.lines_per_update;

        let mut current_timeout = base_timeout;
        let mut retry_remaining = true;
        let mut accumulated = Arc::new(tokio::sync::Mutex::new(String::new()));
        let mut truncated = Arc::new(AtomicBool::new(false));

        let exit_status = loop {
            let mut cmd = tokio::process::Command::new("bash");
            cmd.arg("-c").arg(&guarded_command);

            if let Some(ref cwd) = self.cwd {
                cmd.current_dir(cwd);
            }

            // Pipe stdout/stderr for line-by-line reading
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());

            let mut child = cmd.spawn().map_err(|e| {
                crate::state::stash_diagnostic_error(&format!("bash spawn failed: {e}"));
                ToolError::Failed(format!("Failed to spawn: {e}"))
            })?;

            // Take stdout/stderr handles
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            accumulated = Arc::new(tokio::sync::Mutex::new(String::new()));
            truncated = Arc::new(AtomicBool::new(false));

            // Spawn a task to read stdout + stderr lines and accumulate them
            let acc_clone = Arc::clone(&accumulated);
            let trunc_clone = Arc::clone(&truncated);
            let cancel_clone = cancel.clone();
            let ctx_clone = ctx.clone();
            let emit_progress = !self.progress_requires_tty || crate::format::stderr_is_terminal();

            let reader_handle = tokio::spawn(async move {
                let stdout_reader = stdout.map(tokio::io::BufReader::new);
                let stderr_reader = stderr.map(tokio::io::BufReader::new);

                let mut stdout_lines = stdout_reader.map(|r| r.lines());
                let mut stderr_lines = stderr_reader.map(|r| r.lines());

                let mut lines_since_update: usize = 0;
                let mut last_update = tokio::time::Instant::now();
                let mut stdout_done = stdout_lines.is_none();
                let mut stderr_done = stderr_lines.is_none();

                loop {
                    if cancel_clone.is_cancelled() {
                        break;
                    }
                    if stdout_done && stderr_done {
                        break;
                    }

                    // Read one line from whichever stream has data, tracking its source
                    let line_info: Option<(String, bool)> = tokio::select! {
                        biased;
                        result = async {
                            match stdout_lines.as_mut() {
                                Some(lines) => lines.next_line().await,
                                None => std::future::pending().await,
                            }
                        }, if !stdout_done => {
                            match result {
                                Ok(Some(line)) => Some((line, false)),
                                Ok(None) => { stdout_done = true; None }
                                Err(_) => { stdout_done = true; None }
                            }
                        }
                        result = async {
                            match stderr_lines.as_mut() {
                                Some(lines) => lines.next_line().await,
                                None => std::future::pending().await,
                            }
                        }, if !stderr_done => {
                            match result {
                                Ok(Some(line)) => Some((line, true)),
                                Ok(None) => { stderr_done = true; None }
                                Err(_) => { stderr_done = true; None }
                            }
                        }
                    };

                    if let Some((line, is_stderr)) = line_info {
                        let mut acc = acc_clone.lock().await;
                        if acc.len() < max_bytes {
                            if !acc.is_empty() {
                                acc.push('\n');
                            }
                            acc.push_str(&line);
                            if acc.len() > max_bytes {
                                let safe_len = crate::format::safe_truncate(&acc, max_bytes).len();
                                acc.truncate(safe_len);
                                acc.push_str("\n... (output truncated)");
                                trunc_clone.store(true, Ordering::Relaxed);
                            }
                        }
                        lines_since_update += 1;
                        drop(acc);

                        // Emit real-time progress for each line (interactive mode only)
                        if emit_progress {
                            if let Some(ref on_progress) = ctx_clone.on_progress {
                                let progress_text = if is_stderr {
                                    format!("stderr: {line}")
                                } else {
                                    line.clone()
                                };
                                on_progress(progress_text);
                            }
                        }

                        // Emit update if interval elapsed or enough lines accumulated
                        let elapsed = last_update.elapsed();
                        if elapsed >= update_interval || lines_since_update >= lines_per_update {
                            let snapshot = acc_clone.lock().await.clone();
                            emit_update(&ctx_clone, &snapshot);
                            lines_since_update = 0;
                            last_update = tokio::time::Instant::now();
                        }
                    }
                }
            });

            // Wait for the process with timeout and cancellation
            let mut timed_out = false;
            let result = tokio::select! {
                _ = cancel.cancelled() => {
                    // Kill the child process on cancellation
                    let _ = child.kill().await;
                    reader_handle.abort();
                    return Err(yoagent::types::ToolError::Cancelled);
                }
                result = tokio::time::timeout(current_timeout, child.wait()) => {
                    match result {
                        Ok(Ok(status)) => {
                            // Wait for the reader to finish consuming remaining buffered output
                            let _ = tokio::time::timeout(Duration::from_secs(2), reader_handle).await;
                            Ok(Some(status))
                        }
                        Ok(Err(e)) => {
                            reader_handle.abort();
                            crate::state::stash_diagnostic_error(&format!("bash wait failed: {e}"));
                            Err(ToolError::Failed(format!("Failed to wait: {e}")))
                        }
                        Err(_elapsed) => {
                            let _ = child.kill().await;
                            reader_handle.abort();
                            if retry_remaining && current_timeout < Duration::from_secs(600) {
                                retry_remaining = false;
                                current_timeout = (current_timeout * 2).min(Duration::from_secs(600));
                                crate::state::stash_diagnostic_error(&format!(
                                    "bash timeout (retrying): {} after {}s, retry with {}s",
                                    command,
                                    base_timeout.as_secs(),
                                    current_timeout.as_secs()
                                ));
                                timed_out = true;
                                Ok(None)
                            } else {
                                crate::state::stash_diagnostic_error(&format!(
                                    "bash timeout: {} after {}s",
                                    command,
                                    current_timeout.as_secs()
                                ));
                                Err(ToolError::Failed(format!(
                                    "Command timed out after {}s",
                                    current_timeout.as_secs()
                                )))
                            }
                        }
                    }
                }
            };

            if timed_out {
                continue;
            }

            match result {
                Ok(Some(status)) => break status,
                Ok(None) => unreachable!("timed_out handled above"),
                Err(e) => return Err(e),
            }
        };

        let exit_code = exit_status.code().unwrap_or(-1);
        let output = accumulated.lock().await.clone();

        // One final update with the complete output
        emit_update(&ctx, &output);

        let formatted = if exit_code != 0 {
            format!(
                "Exit code: {exit_code}. Tip: use explicit paths (./script.sh, not script.sh) and -- to separate flags from positional args.\n{output}"
            )
        } else {
            output
        };

        Ok(TR {
            content: vec![Content::Text { text: formatted }],
            details: serde_json::json!({ "exit_code": exit_code, "success": exit_code == 0 }),
        })
    }
}

// ── rename_symbol agent tool ─────────────────────────────────────────────

/// An agent-invocable tool for renaming symbols across a project.
/// Wraps `commands_project::rename_in_project` so the LLM can do cross-file
/// renames in a single tool call instead of multiple edit_file invocations.
pub(crate) struct RenameSymbolTool;

#[async_trait::async_trait]
impl AgentTool for RenameSymbolTool {
    fn name(&self) -> &str {
        "rename_symbol"
    }

    fn label(&self) -> &str {
        "Rename"
    }

    fn description(&self) -> &str {
        "Rename a symbol across the project. Performs word-boundary-aware find-and-replace \
         in all git-tracked files. More reliable than multiple edit_file calls for renames. \
         Returns a preview of changes and the number of files modified."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "old_name": {
                    "type": "string",
                    "description": "The current name of the symbol to rename"
                },
                "new_name": {
                    "type": "string",
                    "description": "The new name for the symbol"
                },
                "path": {
                    "type": "string",
                    "description": "Optional: limit rename to a specific file or directory (default: entire project)"
                }
            },
            "required": ["old_name", "new_name"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        use yoagent::types::{Content, ToolError, ToolResult as TR};

        let old_name = params["old_name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'old_name' parameter".into()))?;

        let new_name = params["new_name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'new_name' parameter".into()))?;

        let scope = params["path"].as_str();

        match commands_project::rename_in_project(old_name, new_name, scope) {
            Ok(result) => {
                let summary = format!(
                    "Renamed '{}' → '{}': {} replacement{} across {} file{}.\n\nFiles changed:\n{}\n\n{}",
                    old_name,
                    new_name,
                    result.total_replacements,
                    if result.total_replacements == 1 { "" } else { "s" },
                    result.files_changed.len(),
                    if result.files_changed.len() == 1 { "" } else { "s" },
                    result.files_changed.iter().map(|f| format!("  - {f}")).collect::<Vec<_>>().join("\n"),
                    result.preview,
                );
                Ok(TR {
                    content: vec![Content::Text { text: summary }],
                    details: serde_json::json!({}),
                })
            }
            Err(msg) => Err(ToolError::Failed(msg)),
        }
    }
}

// ── ask_user agent tool ──────────────────────────────────────────────────

/// Tool that lets the model ask the user directed questions.
/// The user types their answer, which is returned as the tool result.
/// Only registered in interactive mode (when stdin is a terminal).
pub struct AskUserTool;

#[async_trait::async_trait]
impl AgentTool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn label(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a question to get clarification or input. Use this when you need \
         specific information to proceed, like a preference, a decision, or context that \
         isn't available in the codebase. The user sees your question and types a response."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask the user. Be specific and concise."
                }
            },
            "required": ["question"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        use yoagent::types::{Content, ToolError, ToolResult as TR};

        let question = params
            .get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("Missing 'question' parameter".into()))?;

        // Display the question with visual distinction
        eprintln!("\n{YELLOW}  ❓ {question}{RESET}");
        eprint!("{GREEN}  → {RESET}");
        io::stderr().flush().ok();

        // Read the user's response
        use std::io::BufRead;
        let mut response = String::new();
        let stdin = io::stdin();
        match stdin.lock().read_line(&mut response) {
            Ok(0) | Err(_) => {
                return Ok(TR {
                    content: vec![Content::Text {
                        text: "(user provided no response)".to_string(),
                    }],
                    details: serde_json::Value::Null,
                });
            }
            _ => {}
        }

        let response = response.trim().to_string();
        if response.is_empty() {
            return Ok(TR {
                content: vec![Content::Text {
                    text: "(user provided empty response)".to_string(),
                }],
                details: serde_json::Value::Null,
            });
        }

        Ok(TR {
            content: vec![Content::Text { text: response }],
            details: serde_json::Value::Null,
        })
    }
}

// ── todo agent tool ──────────────────────────────────────────────────────

/// Agent tool for managing a task list during complex multi-step operations.
pub struct TodoTool;

#[async_trait::async_trait]
impl AgentTool for TodoTool {
    fn name(&self) -> &str {
        "todo"
    }

    fn label(&self) -> &str {
        "todo"
    }

    fn description(&self) -> &str {
        "Manage a task list to track progress on complex multi-step operations. \
         Use this to plan work, check off completed steps, and see what's remaining. \
         Available actions: list, add, done, wip, remove, clear."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "add", "done", "wip", "remove", "clear"],
                    "description": "Action: list (show all), add (create task), done (mark complete), wip (mark in-progress), remove (delete task), clear (delete all)"
                },
                "description": {
                    "type": "string",
                    "description": "Task description (required for 'add')"
                },
                "id": {
                    "type": "integer",
                    "description": "Task ID number (required for 'done', 'wip', 'remove')"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        use yoagent::types::{Content, ToolError, ToolResult as TR};

        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArgs("Missing required 'action' parameter".into()))?;

        let text =
            match action {
                "list" => {
                    let items = commands_todo::todo_list();
                    if items.is_empty() {
                        "No tasks. Use action 'add' to create one.".to_string()
                    } else {
                        commands_todo::format_todo_list(&items)
                    }
                }
                "add" => {
                    let desc = params
                        .get("description")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidArgs("Missing 'description' for add action".into())
                        })?;
                    let id = commands_todo::todo_add(desc);
                    format!("Added task #{id}: {desc}")
                }
                "done" => {
                    let id = params.get("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                        ToolError::InvalidArgs("Missing 'id' for done action".into())
                    })? as usize;
                    commands_todo::todo_update(id, commands_todo::TodoStatus::Done)
                        .map_err(ToolError::Failed)?;
                    format!("Task #{id} marked as done ✓")
                }
                "wip" => {
                    let id = params.get("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                        ToolError::InvalidArgs("Missing 'id' for wip action".into())
                    })? as usize;
                    commands_todo::todo_update(id, commands_todo::TodoStatus::InProgress)
                        .map_err(ToolError::Failed)?;
                    format!("Task #{id} marked as in-progress")
                }
                "remove" => {
                    let id = params.get("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                        ToolError::InvalidArgs("Missing 'id' for remove action".into())
                    })? as usize;
                    let item = commands_todo::todo_remove(id).map_err(ToolError::Failed)?;
                    format!("Removed task #{id}: {}", item.description)
                }
                "clear" => {
                    commands_todo::todo_clear();
                    "All tasks cleared.".to_string()
                }
                other => {
                    return Err(ToolError::InvalidArgs(format!(
                        "Unknown action '{other}'. Use: list, add, done, wip, remove, clear"
                    )));
                }
            };

        Ok(TR {
            content: vec![Content::Text { text }],
            details: serde_json::Value::Null,
        })
    }
}

// ---------------------------------------------------------------------------
// WebSearchTool — agent-callable web search via DuckDuckGo
// ---------------------------------------------------------------------------

/// Search the web and return results the agent can use during problem-solving.
pub(crate) struct WebSearchTool;

#[async_trait::async_trait]
impl AgentTool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn label(&self) -> &str {
        "WebSearch"
    }

    fn description(&self) -> &str {
        "Search the web using DuckDuckGo. Returns a list of search results with titles, \
         URLs, and snippets. Use this when you need to look up documentation, find solutions \
         to errors, or research unfamiliar topics."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5, max: 20)"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        _ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        use yoagent::types::{Content, ToolError, ToolResult as TR};

        let query = params["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("missing 'query' parameter".into()))?;

        if query.trim().is_empty() {
            return Err(ToolError::InvalidArgs(
                "'query' parameter must not be empty".into(),
            ));
        }

        let max_results = params["max_results"]
            .as_u64()
            .map(|n| n.min(20) as usize)
            .unwrap_or(5);

        let result = commands_web::web_search_and_read(query, max_results);
        Ok(TR {
            content: vec![Content::Text { text: result }],
            details: serde_json::json!({}),
        })
    }
}

// ---------------------------------------------------------------------------
// Permission persistence — offer to save "always" approvals to .yoyo.toml
// ---------------------------------------------------------------------------

use std::collections::HashSet;
use std::sync::Mutex;

/// Simplify a bash command into a glob pattern suitable for the allow list.
///
/// Heuristic: keep the first 2 tokens (base command + subcommand), append `*`.
/// This produces patterns like `cargo test*`, `npm run*`, `git commit*`.
pub fn simplify_command_pattern(cmd: &str) -> String {
    let tokens: Vec<&str> = cmd.split_whitespace().collect();
    let base = match tokens.len() {
        0 => return "*".to_string(),
        1 => tokens[0].to_string(),
        _ => format!("{} {}", tokens[0], tokens[1]),
    };
    format!("{base}*")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BashApprovalRequest {
    command: String,
    prompt_text: String,
    risk_label: &'static str,
    warning: Option<String>,
}

impl BashApprovalRequest {
    fn is_critical(&self) -> bool {
        self.risk_label == "critical"
    }
}

fn bash_approval_request(input: &str) -> BashApprovalRequest {
    if let Some((warning, command)) = input.split_once("\nCommand: ") {
        let command = command.trim();
        let warning = warning
            .trim()
            .trim_start_matches(|ch: char| !ch.is_ascii_alphanumeric())
            .trim()
            .to_string();
        if !command.is_empty() && !warning.is_empty() {
            return BashApprovalRequest {
                command: command.to_string(),
                prompt_text: command.to_string(),
                risk_label: "critical",
                warning: Some(warning),
            };
        }
    }

    BashApprovalRequest {
        command: input.to_string(),
        prompt_text: input.to_string(),
        risk_label: "high",
        warning: None,
    }
}

fn bash_allows_noninteractive_policy(request: &BashApprovalRequest) -> bool {
    !request.is_critical()
}

fn bash_approval_response(request: &BashApprovalRequest, response: &str) -> (bool, &'static str) {
    let response = response.trim().to_lowercase();
    let approved = if request.is_critical() {
        matches!(response.as_str(), "y" | "yes")
    } else {
        matches!(response.as_str(), "y" | "yes" | "a" | "always")
    };
    let approval_mode = if request.is_critical() && approved {
        "single_critical"
    } else if matches!(response.as_str(), "a" | "always") && approved {
        "always"
    } else if approved {
        "single"
    } else {
        "denied"
    };
    (approved, approval_mode)
}

/// Track which patterns we've already offered to save this session,
/// so we don't repeatedly ask for the same base pattern.
fn already_offered_persistence(pattern: &str) -> bool {
    static OFFERED: std::sync::LazyLock<Mutex<HashSet<String>>> =
        std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));
    let mut set = OFFERED.lock().unwrap_or_else(|e| e.into_inner());
    !set.insert(pattern.to_string())
}

/// After the user says "always", offer to persist the pattern to .yoyo.toml.
/// Returns without action if the pattern was already offered this session.
fn offer_persist_pattern(cmd: &str) {
    let pattern = simplify_command_pattern(cmd);

    // Don't re-ask if we already offered this pattern this session
    if already_offered_persistence(&pattern) {
        return;
    }

    eprint!(
        "{DIM}  Save '{pattern}' to .yoyo.toml allow list? ({GREEN}y{RESET}{DIM}/{RED}n{RESET}{DIM}) {RESET}"
    );
    io::stderr().flush().ok();

    let mut response = String::new();
    let stdin = io::stdin();
    use std::io::BufRead;
    if stdin.lock().read_line(&mut response).is_err() {
        return;
    }
    let response = response.trim().to_lowercase();
    if matches!(response.as_str(), "y" | "yes") {
        match crate::config::append_allow_pattern(&pattern) {
            Ok(path) => {
                eprintln!("{GREEN}  ✓ Saved to {}{RESET}", path.display());
            }
            Err(e) => {
                eprintln!("{RED}  ✗ Could not save: {e}{RESET}");
            }
        }
    }
}

/// Build the tool set, optionally with a bash confirmation prompt.
/// When `auto_approve` is false (default), bash commands and file writes require user approval.
/// The "always" option sets a session-wide flag so subsequent operations are auto-approved.
/// The same `always_approved` flag is shared across bash, write_file, and edit_file.
/// When `permissions` has patterns, matching commands/paths are auto-approved or auto-denied.
/// When `dir_restrictions` has rules, file tools check paths before executing.
/// When `audit` is true, all tools are wrapped with the AuditHook via the hook system.
pub fn build_tools(
    auto_approve: bool,
    permissions: &cli::PermissionConfig,
    dir_restrictions: &cli::DirectoryRestrictions,
    max_tool_output: usize,
    audit: bool,
    shell_hooks: Vec<hooks::ShellHook>,
) -> Vec<Box<dyn AgentTool>> {
    // Shared flag: when any tool gets "always", all tools skip prompts
    let always_approved = Arc::new(AtomicBool::new(false));
    if auto_approve {
        always_approved.store(true, Ordering::Relaxed);
    }

    let bash = if auto_approve {
        StreamingBashTool::default()
    } else {
        let flag = Arc::clone(&always_approved);
        let perms = permissions.clone();
        StreamingBashTool::default().with_confirm(move |cmd: &str| {
            let request = bash_approval_request(cmd);
            let command = request.command.as_str();
            let description = format!("bash: {command}");

            // If user previously chose "always", skip the prompt for high-risk
            // commands only. Critical commands still require explicit approval.
            if bash_allows_noninteractive_policy(&request) && flag.load(Ordering::Relaxed) {
                eprintln!(
                    "{GREEN}  ✓ Auto-approved: {RESET}{}",
                    truncate_with_ellipsis(command, 120)
                );
                crate::tool_wrappers::record_tool_policy_decision(
                    "bash_command",
                    &description,
                    command,
                    true,
                    "session_always",
                    request.risk_label,
                );
                return true;
            }

            // Permission policy can handle high-risk commands. Critical commands
            // always require a fresh human decision for this specific command.
            if bash_allows_noninteractive_policy(&request) {
                if let Some(allowed) = perms.check(command) {
                    if allowed {
                        eprintln!(
                            "{GREEN}  ✓ Permitted: {RESET}{}",
                            truncate_with_ellipsis(command, 120)
                        );
                        crate::tool_wrappers::record_tool_policy_decision(
                            "bash_command",
                            &description,
                            command,
                            true,
                            "permission_allow",
                            request.risk_label,
                        );
                        return true;
                    } else {
                        eprintln!(
                            "{RED}  ✗ Denied by permission rule: {RESET}{}",
                            truncate_with_ellipsis(command, 120)
                        );
                        crate::tool_wrappers::record_tool_policy_decision(
                            "bash_command",
                            &description,
                            command,
                            false,
                            "permission_deny",
                            request.risk_label,
                        );
                        return false;
                    }
                }
            }
            use std::io::BufRead;
            crate::tool_wrappers::record_tool_approval_requested(
                "bash_command",
                &description,
                command,
                request.risk_label,
            );
            if let Some(warning) = &request.warning {
                eprintln!(
                    "{YELLOW}  ⚠ Critical command requires explicit approval: {RESET}{}",
                    truncate_with_ellipsis(warning, 120)
                );
            }
            if request.is_critical() {
                eprint!(
                    "{YELLOW}  ⚠ Allow critical command: {RESET}{}{YELLOW} ? {RESET}({GREEN}y{RESET}/{RED}n{RESET}) ",
                    truncate_with_ellipsis(&request.prompt_text, 120)
                );
            } else {
                eprint!(
                    "{YELLOW}  ⚠ Allow: {RESET}{}{YELLOW} ? {RESET}({GREEN}y{RESET}/{RED}n{RESET}/{GREEN}a{RESET}lways) ",
                    truncate_with_ellipsis(&request.prompt_text, 120)
                );
            }
            io::stderr().flush().ok();
            let mut response = String::new();
            let stdin = io::stdin();
            if stdin.lock().read_line(&mut response).is_err() {
                return false;
            }
            let (approved, approval_mode) = bash_approval_response(&request, &response);
            crate::tool_wrappers::record_tool_approval_received(
                "bash_command",
                &description,
                command,
                approved,
                approval_mode,
            );
            if bash_allows_noninteractive_policy(&request) && approval_mode == "always" {
                flag.store(true, Ordering::Relaxed);
                eprintln!(
                    "{GREEN}  ✓ All subsequent operations will be auto-approved this session.{RESET}"
                );
                // Offer to persist this pattern to .yoyo.toml
                offer_persist_pattern(command);
            }
            approved
        })
    };

    // Build write_file and edit_file with optional confirmation prompts
    let write_tool: Box<dyn AgentTool> = maybe_guard(
        maybe_confirm(
            Box::new(WriteFileTool::new()),
            &always_approved,
            permissions,
        ),
        dir_restrictions,
    );
    let edit_tool: Box<dyn AgentTool> = maybe_guard(
        maybe_confirm(Box::new(EditFileTool::new()), &always_approved, permissions),
        dir_restrictions,
    );

    // Build rename_symbol tool with optional confirmation (it writes files)
    let rename_tool: Box<dyn AgentTool> =
        maybe_confirm(Box::new(RenameSymbolTool), &always_approved, permissions);

    // Shared failure tracker for recovery hints — counts per-tool failures
    // so hints escalate from diagnostic to alternative suggestions.
    let failure_tracker = ToolFailureTracker::new();

    // Build hook registry — AuditHook when audit mode is on, plus user-configured shell hooks.
    let hooks = {
        let mut registry = HookRegistry::new();
        if audit {
            registry.register(Box::new(AuditHook));
        }
        for hook in shell_hooks {
            registry.register(Box::new(hook));
        }
        Arc::new(registry)
    };

    let mut tools = vec![
        maybe_hook(
            with_recovery_hints(
                with_truncation(Box::new(bash), max_tool_output),
                &failure_tracker,
            ),
            &hooks,
        ),
        maybe_hook(
            with_recovery_hints(
                with_truncation(
                    maybe_guard(Box::new(ReadFileTool::default()), dir_restrictions),
                    max_tool_output,
                ),
                &failure_tracker,
            ),
            &hooks,
        ),
        maybe_hook(
            with_recovery_hints(
                with_truncation(with_auto_check(write_tool), max_tool_output),
                &failure_tracker,
            ),
            &hooks,
        ),
        maybe_hook(
            with_recovery_hints(
                with_truncation(with_smart_edit(with_auto_check(edit_tool)), max_tool_output),
                &failure_tracker,
            ),
            &hooks,
        ),
        maybe_hook(
            with_recovery_hints(
                with_truncation(
                    maybe_guard(Box::new(ListFilesTool::default()), dir_restrictions),
                    max_tool_output,
                ),
                &failure_tracker,
            ),
            &hooks,
        ),
        maybe_hook(
            with_recovery_hints(
                with_truncation(
                    maybe_guard(Box::new(ProjectSearchTool::default()), dir_restrictions),
                    max_tool_output,
                ),
                &failure_tracker,
            ),
            &hooks,
        ),
        maybe_hook(
            with_recovery_hints(
                with_truncation(rename_tool, max_tool_output),
                &failure_tracker,
            ),
            &hooks,
        ),
    ];

    // Only add ask_user in interactive mode (stdin is a terminal).
    // In piped mode or test environments, this tool isn't available.
    if std::io::stdin().is_terminal() {
        tools.push(maybe_hook(Box::new(AskUserTool), &hooks));
    }

    // TodoTool is always available — it only modifies in-memory state, not filesystem
    tools.push(maybe_hook(Box::new(TodoTool), &hooks));

    // WebSearchTool — agent-callable web search (always available)
    tools.push(maybe_hook(
        with_recovery_hints(
            with_truncation(Box::new(WebSearchTool), max_tool_output),
            &failure_tracker,
        ),
        &hooks,
    ));

    // In lite mode (small context window), augment tool descriptions with
    // JSON format examples so small/local LLMs can produce valid tool calls.
    if crate::cli_config::effective_context_tokens() <= 16_000 {
        tools = tools.into_iter().map(with_lite_description).collect();
    }

    tools
}

/// Build a SubAgentTool that inherits the parent's provider/model/key.
/// The sub-agent gets basic tools with inherited directory restrictions
/// (no permission prompts, no sub-agent recursion).
///
/// Returns `(SubAgentTool, SharedState)` — the `SharedState` handle lets the
/// parent agent pre-populate or read shared variables. The sub-agent
/// automatically receives a `shared_state` tool (via yoagent's
/// `SharedStateTool`) so it can read/write the same store.
pub(crate) fn build_sub_agent_tool(config: &AgentConfig) -> (SubAgentTool, SharedState) {
    let shared_state = SharedState::new();

    // Sub-agent gets standard yoagent tools — no permission guards needed
    // since the parent already authorized the delegation.
    // Directory restrictions ARE inherited to prevent sub-agents from bypassing
    // path-based security boundaries.
    let restrictions = &config.dir_restrictions;
    let child_tools: Vec<Arc<dyn AgentTool>> = vec![
        Arc::new(yoagent::tools::bash::BashTool::default()),
        maybe_guard_arc(Arc::new(ReadFileTool::default()), restrictions),
        maybe_guard_arc(Arc::new(WriteFileTool::new()), restrictions),
        maybe_guard_arc(Arc::new(EditFileTool::new()), restrictions),
        maybe_guard_arc(Arc::new(ListFilesTool::default()), restrictions),
        maybe_guard_arc(Arc::new(ProjectSearchTool::default()), restrictions),
        Arc::new(WebSearchTool),
    ];

    // Select the right provider
    let provider: Arc<dyn StreamProvider> = match config.provider.as_str() {
        "anthropic" => Arc::new(AnthropicProvider),
        "google" => Arc::new(GoogleProvider),
        "bedrock" => Arc::new(BedrockProvider),
        _ => Arc::new(OpenAiCompatProvider),
    };

    let tool = SubAgentTool::new("sub_agent", provider)
        .with_description(
            "Delegate a subtask to a fresh sub-agent with its own context window. \
             Use for complex, self-contained subtasks like: researching a codebase, \
             running a series of tests, or implementing a well-scoped change. \
             The sub-agent has bash, file read/write/edit, list, and search tools. \
             It starts with a clean context and returns a summary of what it did.",
        )
        .with_system_prompt(
            "You are a focused sub-agent. Complete the given task efficiently \
             using the tools available. Be thorough but concise in your final \
             response — summarize what you did, what you found, and any issues.",
        )
        .with_model(&config.model)
        .with_api_key(&config.api_key)
        .with_tools(child_tools)
        .with_thinking(config.thinking)
        .with_max_turns(25)
        .with_shared_state(shared_state.clone());

    (tool, shared_state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::time::Duration;
    use yoagent::ThinkingLevel;

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
            auto_watch: true,
            allowed_tools: vec![],
            disallowed_tools: vec![],
            no_tools: false,
            lite: false,
        }
    }

    #[test]
    fn test_build_tools_returns_eight_tools() {
        // build_tools should return 8 tools regardless of auto_approve (in non-terminal: no ask_user)
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools_approved = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let tools_confirm = build_tools(false, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(tools_approved.len(), 9);
        assert_eq!(tools_confirm.len(), 9);
    }

    #[test]
    fn test_build_sub_agent_tool_returns_correct_name() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (tool, _state) = build_sub_agent_tool(&config);
        assert_eq!(tool.name(), "sub_agent");
    }

    #[test]
    fn test_build_sub_agent_tool_has_task_parameter() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (tool, _state) = build_sub_agent_tool(&config);
        let schema = tool.parameters_schema();
        assert!(
            schema["properties"]["task"].is_object(),
            "Should have 'task' parameter"
        );
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("task")));
    }

    #[test]
    fn test_build_sub_agent_tool_all_providers() {
        // All provider paths should build without panic
        let (_tool_anthropic, _) =
            build_sub_agent_tool(&test_agent_config("anthropic", "claude-sonnet-4-20250514"));
        let (_tool_google, _) =
            build_sub_agent_tool(&test_agent_config("google", "gemini-2.0-flash"));
        let (_tool_openai, _) = build_sub_agent_tool(&test_agent_config("openai", "gpt-4o"));
        let (_tool_bedrock, _) = build_sub_agent_tool(&test_agent_config(
            "bedrock",
            "anthropic.claude-sonnet-4-20250514-v1:0",
        ));
    }

    #[test]
    fn test_build_sub_agent_tool_inherits_dir_restrictions() {
        // Sub-agent should inherit directory restrictions from parent config
        let mut config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        config.dir_restrictions = cli::DirectoryRestrictions {
            allow: vec!["./src".to_string()],
            deny: vec!["/etc".to_string()],
        };
        // Should build without panic — restrictions are applied to file tools
        let (tool, _state) = build_sub_agent_tool(&config);
        assert_eq!(tool.name(), "sub_agent");
    }

    #[test]
    fn test_build_sub_agent_tool_no_restrictions_still_works() {
        // Empty restrictions shouldn't break sub-agent building
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        assert!(config.dir_restrictions.is_empty());
        let (tool, _state) = build_sub_agent_tool(&config);
        assert_eq!(tool.name(), "sub_agent");
    }

    #[test]
    fn test_build_tools_count_unchanged_with_sub_agent() {
        // Verify build_tools still returns exactly 9 — SubAgentTool is added via with_sub_agent
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(
            tools.len(),
            9,
            "build_tools must stay at 9 — SubAgentTool is added via with_sub_agent"
        );
    }

    // === SharedState integration tests ===

    #[test]
    fn test_build_sub_agent_tool_returns_shared_state() {
        // The returned SharedState should be a valid, usable handle
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (_tool, shared_state) = build_sub_agent_tool(&config);
        // SharedState starts empty — verify via the async API
        let rt = tokio::runtime::Runtime::new().unwrap();
        let keys = rt.block_on(shared_state.keys());
        assert!(keys.is_empty(), "Fresh SharedState should have no keys");
    }

    #[test]
    fn test_shared_state_parent_can_prepopulate() {
        // Parent agent should be able to write into SharedState before dispatching
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (_tool, shared_state) = build_sub_agent_tool(&config);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            shared_state
                .set("context", "some analysis artifact".into())
                .await
                .unwrap();
            let val = shared_state.get("context").await;
            assert_eq!(val, Some("some analysis artifact".to_string()));
        });
    }

    #[test]
    fn test_shared_state_independent_per_build() {
        // Each call to build_sub_agent_tool should produce an independent SharedState
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (_tool1, state1) = build_sub_agent_tool(&config);
        let (_tool2, state2) = build_sub_agent_tool(&config);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            state1.set("key", "from_agent_1".into()).await.unwrap();
            // state2 should not see state1's data
            assert_eq!(state2.get("key").await, None);
        });
    }

    // === build_tools confirmation integration tests ===

    #[test]
    fn test_build_tools_auto_approve_skips_confirmation() {
        // Auto-approve preserves canonical tool names even though write-capable
        // tools still pass through the critical-file approval guard.
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(tools.len(), 9);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"edit_file"));
        assert!(names.contains(&"bash"));
    }

    #[test]
    fn test_build_tools_no_approve_includes_confirmation() {
        // When auto_approve is false, write_file and edit_file should still have correct names
        // (ConfirmTool delegates name() to inner tool)
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(false, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(tools.len(), 9);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"edit_file"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"search"));
        assert!(names.contains(&"todo"));
    }

    // -----------------------------------------------------------------------
    // StreamingBashTool tests
    // -----------------------------------------------------------------------

    /// Create a ToolContext for testing, with an optional on_update callback
    /// that collects partial results.
    fn test_tool_context(
        updates: Option<Arc<tokio::sync::Mutex<Vec<yoagent::types::ToolResult>>>>,
    ) -> yoagent::types::ToolContext {
        test_tool_context_with_progress(updates, None)
    }

    /// Create a ToolContext for testing, with optional on_update and on_progress callbacks.
    fn test_tool_context_with_progress(
        updates: Option<Arc<tokio::sync::Mutex<Vec<yoagent::types::ToolResult>>>>,
        progress: Option<Arc<tokio::sync::Mutex<Vec<String>>>>,
    ) -> yoagent::types::ToolContext {
        let on_update: Option<yoagent::types::ToolUpdateFn> = updates.map(|u| {
            Arc::new(move |result: yoagent::types::ToolResult| {
                // Use try_lock to avoid blocking in sync callback
                if let Ok(mut guard) = u.try_lock() {
                    guard.push(result);
                }
            }) as yoagent::types::ToolUpdateFn
        });
        let on_progress: Option<yoagent::types::ProgressFn> = progress.map(|p| {
            Arc::new(move |text: String| {
                if let Ok(mut guard) = p.try_lock() {
                    guard.push(text);
                }
            }) as yoagent::types::ProgressFn
        });
        yoagent::types::ToolContext {
            tool_call_id: "test-id".to_string(),
            tool_name: "bash".to_string(),
            cancel: tokio_util::sync::CancellationToken::new(),
            on_update,
            on_progress,
        }
    }

    #[tokio::test]
    async fn test_project_search_defaults_to_literal_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("state.rs"), "fn take_diagnostic_error() {}\n").unwrap();

        let tool = ProjectSearchTool::default();
        let params = serde_json::json!({
            "pattern": "take_diagnostic_error()",
            "path": dir.path().to_string_lossy(),
        });
        let result = tool
            .execute(params, test_tool_context(None))
            .await
            .expect("literal parentheses should not be treated as invalid regex");
        let text = match &result.content[0] {
            yoagent::types::Content::Text { text } => text,
            _ => panic!("expected text result"),
        };
        assert!(
            text.contains("take_diagnostic_error()"),
            "search should find the literal pattern: {text}"
        );
    }

    #[tokio::test]
    async fn test_project_search_skips_target_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let target = dir.path().join("target/debug/deps");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(src.join("lib.rs"), "pub fn real_match() {}\n").unwrap();
        std::fs::write(
            target.join("artifact.rmeta"),
            "real_match from build artifact\n",
        )
        .unwrap();

        let tool = ProjectSearchTool::default();
        let params = serde_json::json!({
            "pattern": "real_match",
            "path": dir.path().to_string_lossy(),
        });
        let result = tool
            .execute(params, test_tool_context(None))
            .await
            .expect("target artifacts should be skipped, not reported as search errors");
        let text = match &result.content[0] {
            yoagent::types::Content::Text { text } => text,
            _ => panic!("expected text result"),
        };
        assert!(
            text.contains("src/lib.rs"),
            "source match should remain: {text}"
        );
        assert!(
            !text.contains("artifact.rmeta") && !text.contains("target/debug"),
            "target artifacts should not appear in search results: {text}"
        );
    }

    #[tokio::test]
    async fn test_project_search_skips_yoyo_state_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let state = dir.path().join(".yoyo/state");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(src.join("state.rs"), "pub fn diagnostic_match() {}\n").unwrap();
        std::fs::write(
            state.join("state.sqlite"),
            "diagnostic_match from generated sqlite projection\n",
        )
        .unwrap();

        let tool = ProjectSearchTool::default();
        let params = serde_json::json!({
            "pattern": "diagnostic_match",
            "path": dir.path().to_string_lossy(),
        });
        let result = tool
            .execute(params, test_tool_context(None))
            .await
            .expect(".yoyo state artifacts should be skipped, not reported as search errors");
        let text = match &result.content[0] {
            yoagent::types::Content::Text { text } => text,
            _ => panic!("expected text result"),
        };
        assert!(
            text.contains("src/state.rs"),
            "source match should remain: {text}"
        );
        assert!(
            !text.contains(".yoyo") && !text.contains("state.sqlite"),
            ".yoyo generated state should not appear in search results: {text}"
        );
    }

    #[tokio::test]
    async fn test_project_search_regex_error_includes_hint() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "fn example() {}\n").unwrap();

        let tool = ProjectSearchTool::default();
        // Use an unclosed character class which reliably fails regex parsing
        let params = serde_json::json!({
            "pattern": "[",
            "regex": true,
            "path": dir.path().to_string_lossy(),
        });
        let result = tool.execute(params, test_tool_context(None)).await;
        assert!(result.is_err(), "Invalid regex should produce an error");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Hint:"),
            "Error should include a recovery hint, got: {err}"
        );
        assert!(
            err.contains("regex=false"),
            "Hint should mention regex=false, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_project_search_non_regex_error_no_hint() {
        // A non-existent path error should NOT include the hint
        let tool = ProjectSearchTool::default();
        let params = serde_json::json!({
            "pattern": "something",
            "path": "/nonexistent/path/xyz",
        });
        let result = tool.execute(params, test_tool_context(None)).await;
        // This may succeed (exit code 1: no matches) or fail (exit code 2: path error)
        if let Err(err) = result {
            let msg = err.to_string();
            assert!(
                !msg.contains("Hint:"),
                "Non-regex errors should not include hint, got: {msg}"
            );
        }
    }

    #[tokio::test]
    async fn test_project_search_skips_binary_files() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "pub fn binary_test_pattern() {}\n").unwrap();
        // Write a binary file (with null bytes) that also contains the search pattern
        let mut binary = b"binary_test_pattern\0\x00\x01\x02\x03\xff".to_vec();
        // Pad to make it clearly non-text
        binary.extend(std::iter::repeat_n(0u8, 128));
        std::fs::write(src.join("artifact.bin"), &binary).unwrap();

        let tool = ProjectSearchTool::default();
        let params = serde_json::json!({
            "pattern": "binary_test_pattern",
            "path": dir.path().to_string_lossy(),
        });
        let result = tool
            .execute(params, test_tool_context(None))
            .await
            .expect("binary files should be skipped, not reported as search errors");
        let text = match &result.content[0] {
            yoagent::types::Content::Text { text } => text,
            _ => panic!("expected text result"),
        };
        assert!(
            text.contains("src/lib.rs"),
            "source match should remain: {text}"
        );
        assert!(
            !text.contains("artifact.bin"),
            "binary file should not appear in search results: {text}"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_deny_patterns() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "rm -rf /"});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("blocked by safety policy"),
            "Expected deny pattern error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_deny_pattern_fork_bomb() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": ":(){:|:&};:"});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("blocked by safety policy"));
    }

    #[tokio::test]
    async fn test_streaming_bash_confirm_rejection() {
        let tool = StreamingBashTool::default().with_confirm(|_cmd: &str| false);
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "echo hello"});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("not confirmed"),
            "Expected confirmation rejection"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_confirm_approval() {
        let tool = StreamingBashTool::default().with_confirm(|_cmd: &str| true);
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "echo approved"});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_ok());
        let text = &result.unwrap().content[0];
        match text {
            yoagent::types::Content::Text { text } => {
                assert!(text.contains("approved"));
                // Exit code 0: no exit-code line in content
                assert!(!text.contains("Exit code:"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_streaming_bash_requires_explicit_approval_for_dangerous_commands() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "echo 'DROP TABLE users'"});
        let result = tool.execute(params, ctx).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires explicit approval"));
    }

    #[tokio::test]
    async fn test_streaming_bash_allows_confirmed_dangerous_commands() {
        let tool = StreamingBashTool::default().with_confirm(|cmd: &str| {
            cmd.contains("Database destruction") && cmd.contains("DROP TABLE users")
        });
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "echo 'DROP TABLE users'"});
        let result = tool.execute(params, ctx).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_streaming_bash_basic_execution() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "echo hello world"});
        let result = tool.execute(params, ctx).await.unwrap();
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(text.contains("hello world"));
                // Exit code 0: no exit-code line or tip in content (success output is clean)
                assert!(
                    !text.contains("Exit code:"),
                    "successful command should not show exit-code line: {text}"
                );
            }
            _ => panic!("Expected text content"),
        }
        assert_eq!(result.details["exit_code"], 0);
        assert_eq!(result.details["success"], true);
    }

    #[tokio::test]
    async fn test_streaming_bash_captures_exit_code() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "exit 42"});
        let result = tool.execute(params, ctx).await.unwrap();
        assert_eq!(result.details["exit_code"], 42);
        assert_eq!(result.details["success"], false);
        // Verify the content text includes the exit code and bounded-command hint
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(
                    text.starts_with("Exit code: 42."),
                    "non-zero exit should include exit-code line in content: {text}"
                );
                assert!(
                    text.contains("Tip:"),
                    "non-zero exit should include a recovery tip: {text}"
                );
                assert!(
                    text.contains("explicit paths"),
                    "tip should mention explicit paths: {text}"
                );
            }
            _ => panic!("Expected text content"),
        }
    }

    /// When a bash command succeeds (exit 0), the content should not include
    /// an exit-code line or tip — the output is clean by default. The exit
    /// code is still available in the `details` field for programmatic use.
    #[tokio::test]
    async fn test_streaming_bash_exit_zero_no_prefix() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "echo success"});
        let result = tool.execute(params, ctx).await.unwrap();
        assert_eq!(result.details["exit_code"], 0);
        assert_eq!(result.details["success"], true);
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(
                    text.starts_with("success"),
                    "exit 0 should not have exit-code prefix: {text}"
                );
                assert!(
                    !text.contains("Exit code:"),
                    "exit 0 should not contain exit-code line: {text}"
                );
                assert!(
                    !text.contains("Tip:"),
                    "exit 0 should not contain recovery tip: {text}"
                );
            }
            _ => panic!("Expected text content"),
        }
    }

    /// pipefail: when a pipe member fails mid-pipeline, the non-zero exit
    /// propagates to the tool result instead of being masked by the last
    /// command's exit code.
    #[tokio::test]
    async fn test_streaming_bash_pipefail_propagates_first_command_exit() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        // The first command exits 3, cat exits 0. With pipefail, exit=3.
        let params = serde_json::json!({"command": "sh -c 'exit 3' | cat"});
        let result = tool.execute(params, ctx).await.unwrap();
        assert_eq!(result.details["exit_code"], 3);
        assert_eq!(result.details["success"], false);
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(
                    text.starts_with("Exit code: 3."),
                    "pipe-failing command should report exit-code line: {text}"
                );
            }
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_streaming_bash_timeout() {
        let tool = StreamingBashTool {
            timeout: Duration::from_millis(200),
            ..Default::default()
        };
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "sleep 30"});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("timed out"),
            "Expected timeout error"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_output_truncation() {
        let tool = StreamingBashTool {
            max_output_bytes: 100,
            ..Default::default()
        };
        let ctx = test_tool_context(None);
        // Generate output longer than 100 bytes
        let params = serde_json::json!({"command": "for i in $(seq 1 100); do echo \"line number $i of the output\"; done"});
        let result = tool.execute(params, ctx).await.unwrap();
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                // The accumulated output should have been truncated
                // When exit code is 0, the text is just the accumulated output (no exit-code prefix)
                assert!(
                    text.contains("truncated") || text.len() < 500,
                    "Output should be truncated or short, got {} bytes",
                    text.len()
                );
            }
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_streaming_bash_emits_updates() {
        let updates = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let tool = StreamingBashTool {
            lines_per_update: 1,
            update_interval: Duration::from_millis(10),
            ..Default::default()
        };
        let ctx = test_tool_context(Some(Arc::clone(&updates)));
        // Generate multi-line output with small delays to allow update emission
        let params = serde_json::json!({
            "command": "for i in 1 2 3 4 5; do echo line$i; sleep 0.02; done"
        });
        let result = tool.execute(params, ctx).await.unwrap();
        assert!(result.details["success"] == true);

        let collected = updates.lock().await;
        // Should have emitted at least one streaming update
        assert!(
            !collected.is_empty(),
            "Expected at least one streaming update, got none"
        );
        // The final update (or a late one) should contain multiple lines
        let last = &collected[collected.len() - 1];
        match &last.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(
                    text.contains("line"),
                    "Update should contain partial output"
                );
            }
            _ => panic!("Expected text content in update"),
        }
    }

    #[tokio::test]
    async fn test_streaming_bash_missing_command_param() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing"));
    }

    #[tokio::test]
    async fn test_streaming_bash_captures_stderr() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        let params = serde_json::json!({"command": "echo err_output >&2"});
        let result = tool.execute(params, ctx).await.unwrap();
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(text.contains("err_output"), "Should capture stderr: {text}");
            }
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_streaming_bash_progress_emits_each_line() {
        let progress = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let tool = StreamingBashTool {
            progress_requires_tty: false,
            ..Default::default()
        };
        let ctx = test_tool_context_with_progress(None, Some(Arc::clone(&progress)));
        let params = serde_json::json!({
            "command": "echo alpha; echo beta; echo gamma"
        });
        let result = tool.execute(params, ctx).await.unwrap();
        assert_eq!(result.details["exit_code"], 0);

        let lines = progress.lock().await;
        // Each stdout line should appear in progress
        assert!(
            lines.iter().any(|l| l.contains("alpha")),
            "Progress should contain 'alpha', got: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("beta")),
            "Progress should contain 'beta', got: {lines:?}"
        );
        assert!(
            lines.iter().any(|l| l.contains("gamma")),
            "Progress should contain 'gamma', got: {lines:?}"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_progress_stderr_prefix() {
        let progress = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let tool = StreamingBashTool {
            progress_requires_tty: false,
            ..Default::default()
        };
        let ctx = test_tool_context_with_progress(None, Some(Arc::clone(&progress)));
        let params = serde_json::json!({
            "command": "echo normal_out; echo err_line >&2"
        });
        let result = tool.execute(params, ctx).await.unwrap();
        assert_eq!(result.details["exit_code"], 0);

        let lines = progress.lock().await;
        // stdout lines emitted as-is (no prefix)
        let stdout_line = lines.iter().find(|l| l.contains("normal_out"));
        assert!(
            stdout_line.is_some(),
            "Should have stdout progress line, got: {lines:?}"
        );
        assert!(
            !stdout_line.unwrap().starts_with("stderr: "),
            "Stdout lines should not have stderr prefix"
        );
        // stderr lines should have "stderr: " prefix
        let stderr_line = lines.iter().find(|l| l.contains("err_line"));
        assert!(
            stderr_line.is_some(),
            "Should have stderr progress line, got: {lines:?}"
        );
        assert!(
            stderr_line.unwrap().starts_with("stderr: "),
            "Stderr line should have 'stderr: ' prefix, got: {:?}",
            stderr_line.unwrap()
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_progress_complete_output_unchanged() {
        // Verify that the final ToolResult still contains the full buffered output
        // (on_progress doesn't affect the return value)
        let progress = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let tool = StreamingBashTool {
            progress_requires_tty: false,
            ..Default::default()
        };
        let ctx = test_tool_context_with_progress(None, Some(Arc::clone(&progress)));
        let params = serde_json::json!({
            "command": "echo line1; echo line2; echo line3"
        });
        let result = tool.execute(params, ctx).await.unwrap();
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                // Exit code 0: no exit-code line in content
                assert!(!text.contains("Exit code:"));
                assert!(text.contains("line1"));
                assert!(text.contains("line2"));
                assert!(text.contains("line3"));
            }
            _ => panic!("Expected text content"),
        }
    }

    #[tokio::test]
    async fn test_streaming_bash_progress_with_timeout() {
        // Verify timeout still works with on_progress set
        let progress = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let tool = StreamingBashTool {
            timeout: Duration::from_millis(200),
            progress_requires_tty: false,
            ..Default::default()
        };
        let ctx = test_tool_context_with_progress(None, Some(Arc::clone(&progress)));
        let params = serde_json::json!({"command": "sleep 30"});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("timed out"),
            "Expected timeout error"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_progress_with_cancellation() {
        // Verify cancellation still works with on_progress set
        let progress = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
        let tool = StreamingBashTool {
            progress_requires_tty: false,
            ..Default::default()
        };
        let ctx = test_tool_context_with_progress(None, Some(Arc::clone(&progress)));
        let cancel = ctx.cancel.clone();

        // Cancel immediately
        cancel.cancel();
        let params = serde_json::json!({"command": "sleep 30"});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
    }

    // ── rename_symbol tool tests ─────────────────────────────────────

    #[test]
    fn test_rename_symbol_tool_name() {
        let tool = RenameSymbolTool;
        assert_eq!(tool.name(), "rename_symbol");
    }

    #[test]
    fn test_rename_symbol_tool_label() {
        let tool = RenameSymbolTool;
        assert_eq!(tool.label(), "Rename");
    }

    #[test]
    fn test_rename_symbol_tool_schema() {
        let tool = RenameSymbolTool;
        let schema = tool.parameters_schema();
        // Must have old_name, new_name, and path properties
        let props = schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("old_name"),
            "schema should have old_name"
        );
        assert!(
            props.contains_key("new_name"),
            "schema should have new_name"
        );
        assert!(props.contains_key("path"), "schema should have path");
        // old_name and new_name are required
        let required = schema["required"].as_array().unwrap();
        let required_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(required_strs.contains(&"old_name"));
        assert!(required_strs.contains(&"new_name"));
        // path is NOT required
        assert!(!required_strs.contains(&"path"));
    }

    #[test]
    fn test_rename_result_struct() {
        let result = crate::commands_rename::RenameResult {
            files_changed: vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            total_replacements: 5,
            preview: "preview text".to_string(),
        };
        assert_eq!(result.files_changed.len(), 2);
        assert_eq!(result.total_replacements, 5);
        assert_eq!(result.preview, "preview text");
    }

    #[test]
    fn test_rename_symbol_tool_in_build_tools() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(
            names.contains(&"rename_symbol"),
            "build_tools should include rename_symbol, got: {names:?}"
        );
    }

    #[test]
    fn test_build_tools_with_piped_limit() {
        // build_tools should work with the piped limit too
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(
            true,
            &perms,
            &dirs,
            TOOL_OUTPUT_MAX_CHARS_PIPED,
            false,
            vec![],
        );
        assert_eq!(tools.len(), 9, "Should still have 9 tools with piped limit");
    }

    #[test]
    fn test_ask_user_tool_schema() {
        let tool = AskUserTool;
        assert_eq!(tool.name(), "ask_user");
        assert_eq!(tool.label(), "ask_user");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["question"].is_object());
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("question")));
    }

    #[test]
    fn test_ask_user_tool_not_in_non_terminal_mode() {
        // In test environment (no terminal), ask_user should NOT be included
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(
            !names.contains(&"ask_user"),
            "ask_user should not be in non-terminal mode"
        );
    }

    // -----------------------------------------------------------------------
    // TodoTool tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_todo_tool_schema() {
        let tool = TodoTool;
        assert_eq!(tool.name(), "todo");
        assert_eq!(tool.label(), "todo");
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["properties"]["description"].is_object());
        assert!(schema["properties"]["id"].is_object());
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_list_empty() {
        commands_todo::todo_clear();
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "list"}), ctx)
            .await;
        assert!(result.is_ok());
        let text = match &result.unwrap().content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("No tasks"));
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_add_and_list() {
        commands_todo::todo_clear();
        let tool = TodoTool;

        let ctx = test_tool_context(None);
        let result = tool
            .execute(
                serde_json::json!({"action": "add", "description": "Write tests"}),
                ctx,
            )
            .await;
        assert!(result.is_ok());

        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "list"}), ctx)
            .await;
        let text = match &result.unwrap().content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("Write tests"));
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_done() {
        commands_todo::todo_clear();
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        tool.execute(
            serde_json::json!({"action": "add", "description": "Task A"}),
            ctx,
        )
        .await
        .unwrap();

        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "done", "id": 1}), ctx)
            .await;
        let text = match &result.unwrap().content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("done ✓"));
    }

    #[tokio::test]
    async fn test_todo_tool_invalid_action() {
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "explode"}), ctx)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_todo_tool_missing_description() {
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "add"}), ctx)
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_todo_tool_in_build_tools() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(
            names.contains(&"todo"),
            "build_tools should include todo, got: {names:?}"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_custom_timeout() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        // Pass timeout: 1 second, command sleeps 5 — should time out
        let params = serde_json::json!({"command": "sleep 5", "timeout": 1});
        let result = tool.execute(params, ctx).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("timed out"),
            "Expected timeout error with custom timeout of 1s"
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_custom_timeout_default() {
        let tool = StreamingBashTool::default();
        // Without a timeout param, the struct field default (300s) is used
        let schema = tool.parameters_schema();
        let props = schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("timeout"),
            "Schema should include timeout parameter"
        );
        // Verify the default timeout matches the cli_config constant
        assert_eq!(
            tool.timeout,
            Duration::from_secs(crate::cli_config::DEFAULT_BASH_TIMEOUT_SECS)
        );
    }

    #[tokio::test]
    async fn test_streaming_bash_custom_timeout_clamped() {
        let tool = StreamingBashTool::default();
        let ctx = test_tool_context(None);
        // Pass timeout: 9999, which should be clamped to 600
        // We verify by running a fast command — it succeeds because the
        // clamped 600s timeout is more than enough for echo
        let params = serde_json::json!({"command": "echo clamped", "timeout": 9999});
        let result = tool.execute(params, ctx).await.unwrap();
        match &result.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(text.contains("clamped"));
            }
            _ => panic!("Expected text content"),
        }

        // Also verify 0 gets clamped to 1 (minimum) — command still succeeds
        let ctx2 = test_tool_context(None);
        let params2 = serde_json::json!({"command": "echo fast", "timeout": 0});
        let result2 = tool.execute(params2, ctx2).await.unwrap();
        match &result2.content[0] {
            yoagent::types::Content::Text { text } => {
                assert!(text.contains("fast"));
            }
            _ => panic!("Expected text content"),
        }
    }

    // -----------------------------------------------------------------------
    // TodoTool — additional parameter validation tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_todo_tool_wip_missing_id() {
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "wip"}), ctx)
            .await;
        assert!(result.is_err(), "wip without id should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("id"),
            "Error should mention missing 'id', got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_todo_tool_remove_missing_id() {
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "remove"}), ctx)
            .await;
        assert!(result.is_err(), "remove without id should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("id"),
            "Error should mention missing 'id', got: {err_msg}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_clear() {
        commands_todo::todo_clear();
        let tool = TodoTool;

        // Add a task first
        let ctx = test_tool_context(None);
        tool.execute(
            serde_json::json!({"action": "add", "description": "Temp task"}),
            ctx,
        )
        .await
        .unwrap();

        // Clear all tasks
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "clear"}), ctx)
            .await;
        assert!(result.is_ok());
        let text = match &result.unwrap().content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(
            text.contains("cleared"),
            "Clear result should mention 'cleared', got: {text}"
        );

        // Verify list is now empty
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "list"}), ctx)
            .await
            .unwrap();
        let text = match &result.content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(
            text.contains("No tasks"),
            "List after clear should show no tasks, got: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_wip_marks_in_progress() {
        commands_todo::todo_clear();
        let tool = TodoTool;

        // Add then mark wip
        let ctx = test_tool_context(None);
        tool.execute(
            serde_json::json!({"action": "add", "description": "WIP task"}),
            ctx,
        )
        .await
        .unwrap();

        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "wip", "id": 1}), ctx)
            .await;
        assert!(result.is_ok());
        let text = match &result.unwrap().content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(
            text.contains("in-progress"),
            "WIP result should mention 'in-progress', got: {text}"
        );
    }

    #[test]
    fn test_todo_tool_schema_action_required() {
        let tool = TodoTool;
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        let required_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(
            required_strs.contains(&"action"),
            "action should be required, got: {required_strs:?}"
        );
    }

    #[test]
    fn test_todo_tool_schema_action_enum_values() {
        let tool = TodoTool;
        let schema = tool.parameters_schema();
        let action_enum = schema["properties"]["action"]["enum"]
            .as_array()
            .expect("action should have enum");
        let values: Vec<&str> = action_enum.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(values.len(), 6, "Should have 6 action values");
        for expected in &["list", "add", "done", "wip", "remove", "clear"] {
            assert!(
                values.contains(expected),
                "Action enum should contain '{expected}', got: {values:?}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // RenameSymbolTool — parameter validation tests
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_rename_symbol_tool_missing_old_name() {
        let tool = RenameSymbolTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"new_name": "foo"}), ctx)
            .await;
        assert!(result.is_err(), "Missing old_name should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("old_name"),
            "Error should mention 'old_name', got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_rename_symbol_tool_missing_new_name() {
        let tool = RenameSymbolTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"old_name": "foo"}), ctx)
            .await;
        assert!(result.is_err(), "Missing new_name should fail");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("new_name"),
            "Error should mention 'new_name', got: {err_msg}"
        );
    }

    #[test]
    fn test_rename_symbol_tool_schema_required_fields() {
        let tool = RenameSymbolTool;
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        let required_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(
            required_strs.len(),
            2,
            "Should have exactly 2 required fields"
        );
        assert!(required_strs.contains(&"old_name"));
        assert!(required_strs.contains(&"new_name"));
    }

    // -----------------------------------------------------------------------
    // Tool metadata consistency tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_tool_names_unique() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(
                seen.insert(name),
                "Duplicate tool name found: '{name}' in {names:?}"
            );
        }
    }

    #[test]
    fn test_all_tools_have_descriptions() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        for tool in &tools {
            let desc = tool.description();
            assert!(
                !desc.is_empty(),
                "Tool '{}' has empty description",
                tool.name()
            );
        }
    }

    // -----------------------------------------------------------------------
    // build_tools with directory restrictions and audit
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_tools_with_dir_restrictions() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions {
            allow: vec!["./src".to_string()],
            deny: vec!["/etc".to_string(), "/tmp/secret".to_string()],
        };
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(
            tools.len(),
            9,
            "Directory restrictions should not change tool count"
        );
    }

    #[test]
    fn test_build_tools_with_audit_wrapping() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, true, vec![]);
        assert_eq!(
            tools.len(),
            9,
            "Audit wrapping should not change tool count"
        );
        // Verify tool names survive wrapping
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(
            names.contains(&"bash"),
            "Should still have bash after audit wrap"
        );
        assert!(
            names.contains(&"todo"),
            "Should still have todo after audit wrap"
        );
    }

    // -----------------------------------------------------------------------
    // StreamingBashTool — default values and construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_streaming_bash_default_cwd_is_none() {
        let tool = StreamingBashTool::default();
        assert!(tool.cwd.is_none(), "Default cwd should be None");
    }

    #[test]
    fn test_streaming_bash_default_timeout_is_300s() {
        let tool = StreamingBashTool::default();
        assert_eq!(
            tool.timeout,
            Duration::from_secs(crate::cli_config::DEFAULT_BASH_TIMEOUT_SECS)
        );
    }

    #[test]
    fn test_streaming_bash_default_max_output_bytes() {
        let tool = StreamingBashTool::default();
        assert_eq!(
            tool.max_output_bytes,
            256 * 1024,
            "Default max output should be 256KB"
        );
    }

    #[test]
    fn test_streaming_bash_default_deny_patterns_count() {
        let tool = StreamingBashTool::default();
        assert!(
            tool.deny_patterns.len() >= 5,
            "Should have at least 5 deny patterns, got: {}",
            tool.deny_patterns.len()
        );
    }

    #[test]
    fn test_streaming_bash_deny_patterns_include_critical() {
        let tool = StreamingBashTool::default();
        assert!(tool.deny_patterns.contains(&"rm -rf /".to_string()));
        assert!(tool.deny_patterns.contains(&"mkfs".to_string()));
        assert!(tool.deny_patterns.contains(&"dd if=".to_string()));
    }

    #[test]
    fn test_streaming_bash_default_confirm_fn_is_none() {
        let tool = StreamingBashTool::default();
        assert!(
            tool.confirm_fn.is_none(),
            "Default confirm_fn should be None"
        );
    }

    #[test]
    fn test_streaming_bash_with_confirm_sets_fn() {
        let tool = StreamingBashTool::default().with_confirm(|_cmd| true);
        assert!(
            tool.confirm_fn.is_some(),
            "with_confirm should set the confirm_fn"
        );
    }

    #[test]
    fn test_streaming_bash_cwd_can_be_set() {
        let tool = StreamingBashTool {
            cwd: Some("/tmp".to_string()),
            ..Default::default()
        };
        assert_eq!(tool.cwd.as_deref(), Some("/tmp"));
    }

    #[tokio::test]
    async fn test_streaming_bash_cwd_is_applied() {
        let tmp = std::env::temp_dir();
        let tool = StreamingBashTool {
            cwd: Some(tmp.to_string_lossy().to_string()),
            ..Default::default()
        };
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"command": "pwd"}), ctx)
            .await
            .unwrap();
        let text = match &result.content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        // pwd output should contain the temp dir path
        let canonical_tmp = std::fs::canonicalize(&tmp)
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            text.contains(&canonical_tmp),
            "Expected pwd output to contain '{}', got: {}",
            canonical_tmp,
            text
        );
    }

    #[test]
    fn test_streaming_bash_default_update_interval() {
        let tool = StreamingBashTool::default();
        assert_eq!(tool.update_interval, Duration::from_millis(500));
    }

    #[test]
    fn test_streaming_bash_default_lines_per_update() {
        let tool = StreamingBashTool::default();
        assert_eq!(tool.lines_per_update, 20);
    }

    #[test]
    fn test_streaming_bash_name_and_description() {
        let tool = StreamingBashTool::default();
        assert_eq!(tool.name(), "bash");
        assert_eq!(tool.label(), "Execute Command");
        let desc = tool.description();
        assert!(desc.contains("Execute a bash command"));
        assert!(desc.contains("timeout"));
    }

    #[test]
    fn test_streaming_bash_schema_properties() {
        let tool = StreamingBashTool::default();
        let schema = tool.parameters_schema();
        let props = schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("command"),
            "Schema should have 'command'"
        );
        assert!(
            props.contains_key("timeout"),
            "Schema should have 'timeout'"
        );
        // command is required, timeout is not
        let required = schema["required"].as_array().unwrap();
        let req_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(req_strs.contains(&"command"));
        assert!(
            !req_strs.contains(&"timeout"),
            "timeout should not be required"
        );
    }

    #[test]
    fn test_streaming_bash_progress_requires_tty_default() {
        let tool = StreamingBashTool::default();
        assert!(
            tool.progress_requires_tty,
            "Default should require TTY for progress"
        );
    }

    // -----------------------------------------------------------------------
    // RenameSymbolTool — description and schema details
    // -----------------------------------------------------------------------

    #[test]
    fn test_rename_symbol_tool_description_content() {
        let tool = RenameSymbolTool;
        let desc = tool.description();
        assert!(
            desc.contains("word-boundary"),
            "Description should mention word-boundary matching"
        );
        assert!(
            desc.contains("git-tracked"),
            "Description should mention git-tracked files"
        );
    }

    #[test]
    fn test_rename_symbol_tool_schema_path_is_optional() {
        let tool = RenameSymbolTool;
        let schema = tool.parameters_schema();
        let required = schema["required"].as_array().unwrap();
        let req_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
        assert_eq!(req_strs.len(), 2, "Only old_name and new_name are required");
        assert!(!req_strs.contains(&"path"), "path must NOT be required");
    }

    #[test]
    fn test_rename_symbol_tool_schema_property_types() {
        let tool = RenameSymbolTool;
        let schema = tool.parameters_schema();
        let props = schema["properties"].as_object().unwrap();
        // All three properties should be string type
        assert_eq!(props["old_name"]["type"], "string");
        assert_eq!(props["new_name"]["type"], "string");
        assert_eq!(props["path"]["type"], "string");
    }

    // -----------------------------------------------------------------------
    // AskUserTool — description and schema details
    // -----------------------------------------------------------------------

    #[test]
    fn test_ask_user_tool_description_content() {
        let tool = AskUserTool;
        let desc = tool.description();
        assert!(desc.contains("user"), "Description should mention user");
        assert!(
            desc.contains("question"),
            "Description should mention question"
        );
        assert!(
            desc.contains("clarification"),
            "Description should mention clarification"
        );
    }

    #[test]
    fn test_ask_user_tool_schema_question_is_string() {
        let tool = AskUserTool;
        let schema = tool.parameters_schema();
        assert_eq!(
            schema["properties"]["question"]["type"], "string",
            "question parameter should be string type"
        );
    }

    // -----------------------------------------------------------------------
    // TodoTool — edge cases
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_todo_tool_missing_action_entirely() {
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        // Pass empty object — no "action" key at all
        let result = tool.execute(serde_json::json!({}), ctx).await;
        assert!(result.is_err(), "Missing action should produce an error");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("action"),
            "Error should mention missing 'action', got: {err_msg}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_done_nonexistent_id() {
        commands_todo::todo_clear();
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        // Try to mark done an ID that doesn't exist
        let result = tool
            .execute(serde_json::json!({"action": "done", "id": 999}), ctx)
            .await;
        // This should either error or return a message about the task not existing
        // The implementation uses todo_done which panics or returns error on bad id
        assert!(result.is_err(), "done with non-existent id should fail");
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_remove_nonexistent_id() {
        commands_todo::todo_clear();
        let tool = TodoTool;
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "remove", "id": 999}), ctx)
            .await;
        assert!(result.is_err(), "remove with non-existent id should fail");
    }

    #[tokio::test]
    #[serial]
    async fn test_todo_tool_add_multiple_tasks() {
        commands_todo::todo_clear();
        let tool = TodoTool;

        // Add three tasks
        for desc in &["First", "Second", "Third"] {
            let ctx = test_tool_context(None);
            tool.execute(
                serde_json::json!({"action": "add", "description": desc}),
                ctx,
            )
            .await
            .unwrap();
        }

        // List should show all three
        let ctx = test_tool_context(None);
        let result = tool
            .execute(serde_json::json!({"action": "list"}), ctx)
            .await
            .unwrap();
        let text = match &result.content[0] {
            yoagent::types::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("First"), "Should contain 'First'");
        assert!(text.contains("Second"), "Should contain 'Second'");
        assert!(text.contains("Third"), "Should contain 'Third'");
    }

    // -----------------------------------------------------------------------
    // build_tools — canonical tool names
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_tools_canonical_names() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        // In non-terminal mode (test env), there should be exactly these 9 tools
        let expected = [
            "bash",
            "read_file",
            "write_file",
            "edit_file",
            "list_files",
            "search",
            "rename_symbol",
            "todo",
            "web_search",
        ];
        for name in &expected {
            assert!(
                names.contains(name),
                "Expected tool '{name}' not found in: {names:?}"
            );
        }
        assert_eq!(
            names.len(),
            expected.len(),
            "Tool count mismatch: got {names:?}"
        );
    }

    #[test]
    fn test_build_tools_no_ask_user_in_tests() {
        // In test (non-terminal) environment, ask_user should be excluded
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(
            !names.contains(&"ask_user"),
            "ask_user should NOT appear in non-terminal test env"
        );
    }

    // -----------------------------------------------------------------------
    // simplify_command_pattern
    // -----------------------------------------------------------------------

    #[test]
    fn test_simplify_command_pattern_two_tokens() {
        assert_eq!(simplify_command_pattern("cargo test"), "cargo test*");
        assert_eq!(simplify_command_pattern("cargo build"), "cargo build*");
        assert_eq!(simplify_command_pattern("npm run"), "npm run*");
    }

    #[test]
    fn test_simplify_command_pattern_more_tokens() {
        assert_eq!(
            simplify_command_pattern("cargo build --release"),
            "cargo build*"
        );
        assert_eq!(
            simplify_command_pattern("git commit -m \"hello world\""),
            "git commit*"
        );
        assert_eq!(
            simplify_command_pattern("npm run test -- --watch"),
            "npm run*"
        );
    }

    #[test]
    fn test_simplify_command_pattern_single_token() {
        assert_eq!(simplify_command_pattern("ls"), "ls*");
        assert_eq!(simplify_command_pattern("make"), "make*");
    }

    #[test]
    fn test_simplify_command_pattern_empty() {
        assert_eq!(simplify_command_pattern(""), "*");
    }

    #[test]
    fn bash_approval_request_marks_safety_warning_as_critical() {
        let request = bash_approval_request(
            "⚠️  Database destruction: DROP TABLE detected\nCommand: echo 'DROP TABLE users'",
        );

        assert!(request.is_critical());
        assert_eq!(request.command, "echo 'DROP TABLE users'");
        assert_eq!(request.prompt_text, "echo 'DROP TABLE users'");
        assert_eq!(request.risk_label, "critical");
        assert!(request
            .warning
            .as_deref()
            .unwrap()
            .contains("Database destruction"));
    }

    #[test]
    fn bash_approval_request_keeps_normal_commands_high_risk() {
        let request = bash_approval_request("cargo test --bin yyds");

        assert!(!request.is_critical());
        assert_eq!(request.command, "cargo test --bin yyds");
        assert_eq!(request.risk_label, "high");
        assert!(request.warning.is_none());
    }

    #[test]
    fn critical_bash_approval_disables_always_and_policy_shortcuts() {
        let critical = bash_approval_request(
            "⚠️  Force push detected: 'git push --force' can overwrite remote history\nCommand: git push --force",
        );
        let normal = bash_approval_request("cargo test");

        assert!(!bash_allows_noninteractive_policy(&critical));
        assert!(bash_allows_noninteractive_policy(&normal));
        assert_eq!(
            bash_approval_response(&critical, "always"),
            (false, "denied")
        );
        assert_eq!(
            bash_approval_response(&critical, "yes"),
            (true, "single_critical")
        );
        assert_eq!(bash_approval_response(&normal, "always"), (true, "always"));
    }

    // -----------------------------------------------------------------------
    // build_sub_agent_tool — deeper property checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_build_sub_agent_tool_description_mentions_subtask() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (tool, _) = build_sub_agent_tool(&config);
        let desc = tool.description();
        assert!(
            desc.contains("subtask") || desc.contains("sub-agent"),
            "Sub-agent description should mention subtask/sub-agent, got: {desc}"
        );
    }

    #[tokio::test]
    async fn test_build_sub_agent_tool_shared_state_is_independent() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (_, state1) = build_sub_agent_tool(&config);
        let (_, state2) = build_sub_agent_tool(&config);

        // Set a value in state1, it should NOT appear in state2
        state1
            .set("test_key", "test_value".to_string())
            .await
            .unwrap();
        assert_eq!(state1.get("test_key").await, Some("test_value".to_string()));
        assert_eq!(
            state2.get("test_key").await,
            None,
            "Each build_sub_agent_tool call should produce independent shared state"
        );
    }

    #[tokio::test]
    async fn test_build_sub_agent_tool_shared_state_set_get() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (_, state) = build_sub_agent_tool(&config);

        // Initially empty
        assert_eq!(state.get("nonexistent").await, None);

        // Set and get
        state.set("key1", "value1".to_string()).await.unwrap();
        assert_eq!(state.get("key1").await, Some("value1".to_string()));

        // Overwrite
        state.set("key1", "value2".to_string()).await.unwrap();
        assert_eq!(state.get("key1").await, Some("value2".to_string()));
    }

    #[test]
    fn test_build_sub_agent_tool_schema_has_task_description() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let (tool, _) = build_sub_agent_tool(&config);
        let schema = tool.parameters_schema();
        // The task parameter should have a description
        let task_prop = &schema["properties"]["task"];
        assert!(task_prop.is_object(), "task should be an object in schema");
        assert!(
            task_prop.get("description").is_some() || task_prop.get("type").is_some(),
            "task property should have type or description"
        );
    }

    #[test]
    fn test_build_sub_agent_tool_openai_compatible_provider() {
        // "openai-compat", "custom", etc. should all use OpenAiCompatProvider path
        let config = test_agent_config("deepseek", "deepseek-chat");
        let (tool, _) = build_sub_agent_tool(&config);
        assert_eq!(tool.name(), "sub_agent");
    }

    // -----------------------------------------------------------------------
    // WebSearchTool — schema, parameter validation, BUILTIN_TOOL_NAMES
    // -----------------------------------------------------------------------

    #[test]
    fn test_web_search_tool_name() {
        let tool = WebSearchTool;
        assert_eq!(tool.name(), "web_search");
    }

    #[test]
    fn test_web_search_tool_in_builtin_names() {
        use crate::agent_builder::BUILTIN_TOOL_NAMES;
        assert!(
            BUILTIN_TOOL_NAMES.contains(&"web_search"),
            "BUILTIN_TOOL_NAMES must include 'web_search' to guard against MCP collisions"
        );
    }

    #[test]
    fn test_web_search_tool_schema_has_query_required() {
        let tool = WebSearchTool;
        let schema = tool.parameters_schema();
        let props = &schema["properties"];
        assert!(props["query"].is_object(), "Should have 'query' property");
        assert_eq!(props["query"]["type"], "string");
        let required = schema["required"].as_array().unwrap();
        assert!(
            required.contains(&serde_json::json!("query")),
            "query should be required"
        );
    }

    #[test]
    fn test_web_search_tool_schema_has_max_results_optional() {
        let tool = WebSearchTool;
        let schema = tool.parameters_schema();
        let props = &schema["properties"];
        assert!(
            props["max_results"].is_object(),
            "Should have 'max_results' property"
        );
        assert_eq!(props["max_results"]["type"], "integer");
        let required = schema["required"].as_array().unwrap();
        assert!(
            !required.contains(&serde_json::json!("max_results")),
            "max_results should NOT be required"
        );
    }

    #[tokio::test]
    async fn test_web_search_tool_missing_query_returns_error() {
        let tool = WebSearchTool;
        let ctx = test_tool_context(None);
        let result = tool.execute(serde_json::json!({}), ctx).await;
        assert!(result.is_err(), "Missing query should return error");
    }

    #[tokio::test]
    async fn test_web_search_tool_empty_query_returns_error() {
        let tool = WebSearchTool;
        let ctx = test_tool_context(None);
        let result = tool.execute(serde_json::json!({"query": "   "}), ctx).await;
        assert!(
            result.is_err(),
            "Empty/whitespace query should return error"
        );
    }

    #[test]
    fn test_web_search_tool_in_build_tools() {
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(
            names.contains(&"web_search"),
            "web_search should be in build_tools output, got: {names:?}"
        );
    }
}
