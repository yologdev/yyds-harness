//! Tool definitions for the yoyo agent.
//!
//! Contains all agent tool structs and implementations:
//! - `GuardedTool` — directory restriction wrapper
//! - `TruncatingTool` — output truncation wrapper
//! - `ConfirmTool` — user confirmation wrapper for file operations
//! - `StreamingBashTool` — real-time subprocess output
//! - `RenameSymbolTool` — cross-file symbol renaming
//! - `AskUserTool` — interactive question-asking
//! - `TodoTool` — task list management
//! - `build_tools` — assembles the complete tool set
//! - `build_sub_agent_tool` — creates a sub-agent with inherited config

use crate::cli;
use crate::commands_project;
use crate::format::*;
use crate::hooks::{self, maybe_hook, AuditHook, HookRegistry};
use crate::safety::analyze_bash_command;
use crate::AgentConfig;

use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;

use yoagent::provider::{
    AnthropicProvider, BedrockProvider, GoogleProvider, OpenAiCompatProvider, StreamProvider,
};
use yoagent::sub_agent::SubAgentTool;
use yoagent::tools::bash::ConfirmFn;
use yoagent::tools::edit::EditFileTool;
use yoagent::tools::file::{ReadFileTool, WriteFileTool};
use yoagent::tools::list::ListFilesTool;
use yoagent::tools::search::SearchTool;
use yoagent::types::AgentTool;

// --- RTK (Rust Token Killer) integration ---

/// Whether RTK integration is disabled via --no-rtk flag.
static RTK_DISABLED: AtomicBool = AtomicBool::new(false);

/// Cached result of RTK availability detection.
static RTK_AVAILABLE: OnceLock<bool> = OnceLock::new();

/// Whether we've already printed the RTK detection message.
static RTK_ANNOUNCED: AtomicBool = AtomicBool::new(false);

/// Disable RTK integration (called when --no-rtk flag is present).
pub fn disable_rtk() {
    RTK_DISABLED.store(true, Ordering::Relaxed);
}

/// Check if RTK is disabled.
pub fn is_rtk_disabled() -> bool {
    RTK_DISABLED.load(Ordering::Relaxed)
}

/// Detect whether `rtk` is available in PATH. Result is cached.
pub fn detect_rtk() -> bool {
    *RTK_AVAILABLE.get_or_init(|| {
        std::process::Command::new("rtk")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    })
}

/// Commands that RTK supports and can compress output for.
const RTK_SUPPORTED_COMMANDS: &[&str] = &[
    "git",
    "ls",
    "find",
    "grep",
    "cat",
    "head",
    "tail",
    "cargo",
    "npm",
    "pip",
    "docker",
    "kubectl",
    "gh",
    "tree",
    "diff",
    "du",
    "wc",
    "ps",
    "rg",
    "fd",
    "ag",
    "ack",
    "svn",
    "hg",
    "yarn",
    "pnpm",
    "go",
    "rustc",
    "make",
    "cmake",
    "apt",
    "brew",
    "pacman",
    "systemctl",
    "journalctl",
    "df",
    "mount",
    "ip",
    "ss",
    "netstat",
    "curl",
    "wget",
];

/// Check if a command string is a simple command (no pipes, redirects, or control flow).
fn is_simple_command(command: &str) -> bool {
    // Check for shell metacharacters that indicate complex expressions
    // We only match top-level pipes/redirects (not inside quotes)
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut prev_char = '\0';

    for ch in command.chars() {
        match ch {
            '\'' if !in_double_quote && prev_char != '\\' => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && prev_char != '\\' => in_double_quote = !in_double_quote,
            '|' | ';' | '>' | '<' if !in_single_quote && !in_double_quote => return false,
            '&' if !in_single_quote && !in_double_quote => return false,
            _ => {}
        }
        prev_char = ch;
    }
    true
}

/// Prefix a command with `rtk` if appropriate.
/// Returns the command unchanged if:
/// - RTK is not installed
/// - RTK is disabled via --no-rtk
/// - The command already starts with `rtk`
/// - The command is not a simple command (has pipes, redirects, control flow)
/// - The command's base program is not in RTK's supported list
pub fn maybe_prefix_rtk(command: &str) -> String {
    if is_rtk_disabled() || !detect_rtk() {
        return command.to_string();
    }

    let trimmed = command.trim();

    // Don't double-prefix
    if trimmed.starts_with("rtk ") || trimmed == "rtk" {
        return command.to_string();
    }

    // Only prefix simple commands
    if !is_simple_command(trimmed) {
        return command.to_string();
    }

    // Extract the base command (first word, skipping env var assignments)
    let base_cmd = trimmed
        .split_whitespace()
        .find(|word| !word.contains('='))
        .unwrap_or("");

    // Check if this is a supported command
    if RTK_SUPPORTED_COMMANDS.contains(&base_cmd) {
        // Print announcement once
        if !RTK_ANNOUNCED.swap(true, Ordering::Relaxed) {
            eprintln!("📦 RTK detected — using compressed output (disable with --no-rtk)");
        }
        format!("rtk {trimmed}")
    } else {
        command.to_string()
    }
}

/// A wrapper tool that checks directory restrictions before delegating to an inner tool.
/// Intercepts the `"path"` parameter from tool arguments and validates it against
/// the configured `DirectoryRestrictions`. If the path is blocked, the tool returns
/// an error without executing the inner tool.
struct GuardedTool {
    inner: Box<dyn AgentTool>,
    restrictions: cli::DirectoryRestrictions,
}

#[async_trait::async_trait]
impl AgentTool for GuardedTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        // Check the "path" parameter against directory restrictions
        if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
            if let Err(reason) = self.restrictions.check_path(path) {
                return Err(yoagent::types::ToolError::Failed(reason));
            }
        }
        self.inner.execute(params, ctx).await
    }
}

/// A wrapper tool that truncates large tool output to save context window tokens.
/// When tool output exceeds the configured `max_chars`, preserves the first ~100 and
/// last ~50 lines with a clear truncation marker in between.
struct TruncatingTool {
    inner: Box<dyn AgentTool>,
    max_chars: usize,
}

/// Truncate the text content of a ToolResult if it exceeds the given char limit.
pub(crate) fn truncate_result(
    mut result: yoagent::types::ToolResult,
    max_chars: usize,
) -> yoagent::types::ToolResult {
    use yoagent::Content;
    result.content = result
        .content
        .into_iter()
        .map(|c| match c {
            Content::Text { text } => Content::Text {
                text: truncate_tool_output(&text, max_chars),
            },
            other => other,
        })
        .collect();
    result
}

#[async_trait::async_trait]
impl AgentTool for TruncatingTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        let result = self.inner.execute(params, ctx).await?;
        Ok(truncate_result(result, self.max_chars))
    }
}

/// Wrap a tool with output truncation for large results.
fn with_truncation(tool: Box<dyn AgentTool>, max_chars: usize) -> Box<dyn AgentTool> {
    Box::new(TruncatingTool {
        inner: tool,
        max_chars,
    })
}

/// Wrap a tool with directory restrictions if any are configured.
fn maybe_guard(
    tool: Box<dyn AgentTool>,
    restrictions: &cli::DirectoryRestrictions,
) -> Box<dyn AgentTool> {
    if restrictions.is_empty() {
        tool
    } else {
        Box::new(GuardedTool {
            inner: tool,
            restrictions: restrictions.clone(),
        })
    }
}

/// A wrapper tool that checks directory restrictions before delegating to an Arc-wrapped inner tool.
/// Used by sub-agents to inherit the parent's directory restrictions without needing Box ownership.
struct ArcGuardedTool {
    inner: Arc<dyn AgentTool>,
    restrictions: cli::DirectoryRestrictions,
}

#[async_trait::async_trait]
impl AgentTool for ArcGuardedTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        // Check the "path" parameter against directory restrictions
        if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
            if let Err(reason) = self.restrictions.check_path(path) {
                return Err(yoagent::types::ToolError::Failed(reason));
            }
        }
        self.inner.execute(params, ctx).await
    }
}

/// Wrap an Arc-based tool with directory restrictions if any are configured.
/// Used for sub-agent tools which require `Arc<dyn AgentTool>`.
fn maybe_guard_arc(
    tool: Arc<dyn AgentTool>,
    restrictions: &cli::DirectoryRestrictions,
) -> Arc<dyn AgentTool> {
    if restrictions.is_empty() {
        tool
    } else {
        Arc::new(ArcGuardedTool {
            inner: tool,
            restrictions: restrictions.clone(),
        })
    }
}

/// A wrapper tool that prompts for user confirmation before executing write_file or edit_file.
/// Shares the same `always_approved` flag with bash confirmation so "always" applies everywhere.
/// Checks `--allow`/`--deny` patterns against file paths before prompting.
struct ConfirmTool {
    inner: Box<dyn AgentTool>,
    always_approved: Arc<AtomicBool>,
    permissions: cli::PermissionConfig,
}

/// Build a user-facing description for a write_file or edit_file operation.
/// Used by `ConfirmTool` to show what's about to happen before asking y/n/always.
pub fn describe_file_operation(tool_name: &str, params: &serde_json::Value) -> String {
    match tool_name {
        "write_file" => {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>");
            let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let line_count = if content.is_empty() {
                0
            } else {
                content.lines().count()
            };
            if content.is_empty() {
                format!("write: {path} (⚠ EMPTY content — creates/overwrites with empty file)")
            } else {
                let word = crate::format::pluralize(line_count, "line", "lines");
                format!("write: {path} ({line_count} {word})")
            }
        }
        "edit_file" => {
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>");
            let old_text = params
                .get("old_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let new_text = params
                .get("new_text")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let old_lines = old_text.lines().count();
            let new_lines = new_text.lines().count();
            format!("edit: {path} ({old_lines} → {new_lines} lines)")
        }
        "rename_symbol" => {
            let old_name = params
                .get("old_name")
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>");
            let new_name = params
                .get("new_name")
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>");
            let scope = params
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("project");
            format!("rename: {old_name} → {new_name} (in {scope})")
        }
        _ => format!("{tool_name}: file operation"),
    }
}

/// Prompt the user to confirm a file operation (write_file or edit_file).
/// Returns true if the operation should proceed, false if denied.
/// Shared with bash confirm via the same `always_approved` flag.
pub fn confirm_file_operation(
    description: &str,
    path: &str,
    always_approved: &Arc<AtomicBool>,
    permissions: &cli::PermissionConfig,
) -> bool {
    // If user previously chose "always", skip the prompt
    if always_approved.load(Ordering::Relaxed) {
        eprintln!(
            "{GREEN}  ✓ Auto-approved: {RESET}{}",
            truncate_with_ellipsis(description, 120)
        );
        return true;
    }
    // Check permission patterns against the file path
    if let Some(allowed) = permissions.check(path) {
        if allowed {
            eprintln!(
                "{GREEN}  ✓ Permitted: {RESET}{}",
                truncate_with_ellipsis(description, 120)
            );
            return true;
        } else {
            eprintln!(
                "{RED}  ✗ Denied by permission rule: {RESET}{}",
                truncate_with_ellipsis(description, 120)
            );
            return false;
        }
    }
    use std::io::BufRead;
    // Show the operation and ask for approval
    eprint!(
        "{YELLOW}  ⚠ Allow {RESET}{}{YELLOW} ? {RESET}({GREEN}y{RESET}/{RED}n{RESET}/{GREEN}a{RESET}lways) ",
        truncate_with_ellipsis(description, 120)
    );
    io::stderr().flush().ok();
    let mut response = String::new();
    let stdin = io::stdin();
    if stdin.lock().read_line(&mut response).is_err() {
        return false;
    }
    let response = response.trim().to_lowercase();
    let approved = matches!(response.as_str(), "y" | "yes" | "a" | "always");
    if matches!(response.as_str(), "a" | "always") {
        always_approved.store(true, Ordering::Relaxed);
        eprintln!(
            "{GREEN}  ✓ All subsequent operations will be auto-approved this session.{RESET}"
        );
    }
    approved
}

#[async_trait::async_trait]
impl AgentTool for ConfirmTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: yoagent::types::ToolContext,
    ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
        let tool_name = self.inner.name();
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("<unknown>");
        let description = describe_file_operation(tool_name, &params);

        if !confirm_file_operation(&description, path, &self.always_approved, &self.permissions) {
            return Err(yoagent::types::ToolError::Failed(format!(
                "User denied {tool_name} on '{path}'"
            )));
        }
        self.inner.execute(params, ctx).await
    }
}

/// Wrap a tool with a confirmation prompt for write/edit operations.
fn maybe_confirm(
    tool: Box<dyn AgentTool>,
    always_approved: &Arc<AtomicBool>,
    permissions: &cli::PermissionConfig,
) -> Box<dyn AgentTool> {
    Box::new(ConfirmTool {
        inner: tool,
        always_approved: Arc::clone(always_approved),
        permissions: permissions.clone(),
    })
}

// ---------------------------------------------------------------------------
// StreamingBashTool — real-time subprocess output via on_update callbacks
// ---------------------------------------------------------------------------

/// Execute shell commands with real-time streaming output.
///
/// Unlike the upstream `BashTool` which waits for the process to finish before
/// returning output, `StreamingBashTool` reads stdout/stderr line-by-line and
/// calls `ctx.on_update()` periodically so the UI can display partial output
/// as the command runs. This is the difference between staring at a blank screen
/// during `cargo build` and watching compilation progress live.
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
}

impl Default for StreamingBashTool {
    fn default() -> Self {
        Self {
            cwd: None,
            timeout: Duration::from_secs(120),
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
        }
    }
}

impl StreamingBashTool {
    pub fn with_confirm(mut self, f: impl Fn(&str) -> bool + Send + Sync + 'static) -> Self {
        self.confirm_fn = Some(Box::new(f));
        self
    }
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
        "Execute a bash command and return stdout/stderr. Use for running scripts, installing packages, checking system state, etc. Supports an optional timeout parameter (in seconds) for long-running commands."
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
                    "description": "Maximum seconds to wait for command (default: 120, max: 600)"
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
            }
            // If no confirm_fn (piped mode), log warning but allow
            // (the deny_patterns still block the truly catastrophic ones)
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

        let mut cmd = tokio::process::Command::new("bash");
        cmd.arg("-c").arg(&effective_command);

        if let Some(ref cwd) = self.cwd {
            cmd.current_dir(cwd);
        }

        // Pipe stdout/stderr for line-by-line reading
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let timeout = if let Some(t) = params.get("timeout").and_then(|v| v.as_u64()) {
            Duration::from_secs(t.clamp(1, 600))
        } else {
            self.timeout
        };
        let max_bytes = self.max_output_bytes;
        let update_interval = self.update_interval;
        let lines_per_update = self.lines_per_update;

        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError::Failed(format!("Failed to spawn: {e}")))?;

        // Take stdout/stderr handles
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let accumulated = Arc::new(tokio::sync::Mutex::new(String::new()));
        let truncated = Arc::new(AtomicBool::new(false));

        // Spawn a task to read stdout + stderr lines and accumulate them
        let acc_clone = Arc::clone(&accumulated);
        let trunc_clone = Arc::clone(&truncated);
        let cancel_clone = cancel.clone();
        let ctx_clone = ctx.clone();

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

                // Read one line from whichever stream has data
                let line = tokio::select! {
                    biased;
                    result = async {
                        match stdout_lines.as_mut() {
                            Some(lines) => lines.next_line().await,
                            None => std::future::pending().await,
                        }
                    }, if !stdout_done => {
                        match result {
                            Ok(Some(line)) => Some(line),
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
                            Ok(Some(line)) => Some(line),
                            Ok(None) => { stderr_done = true; None }
                            Err(_) => { stderr_done = true; None }
                        }
                    }
                };

                if let Some(line) = line {
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
        let exit_status = tokio::select! {
            _ = cancel.cancelled() => {
                // Kill the child process on cancellation
                let _ = child.kill().await;
                reader_handle.abort();
                return Err(yoagent::types::ToolError::Cancelled);
            }
            _ = tokio::time::sleep(timeout) => {
                let _ = child.kill().await;
                reader_handle.abort();
                return Err(ToolError::Failed(format!(
                    "Command timed out after {}s",
                    timeout.as_secs()
                )));
            }
            status = child.wait() => {
                status.map_err(|e| ToolError::Failed(format!("Failed to wait: {e}")))?
            }
        };

        // Wait for the reader to finish consuming remaining buffered output
        let _ = tokio::time::timeout(Duration::from_secs(2), reader_handle).await;

        let exit_code = exit_status.code().unwrap_or(-1);
        let output = accumulated.lock().await.clone();

        // One final update with the complete output
        emit_update(&ctx, &output);

        let formatted = format!("Exit code: {exit_code}\n{output}");

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
                    let items = commands_project::todo_list();
                    if items.is_empty() {
                        "No tasks. Use action 'add' to create one.".to_string()
                    } else {
                        commands_project::format_todo_list(&items)
                    }
                }
                "add" => {
                    let desc = params
                        .get("description")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            ToolError::InvalidArgs("Missing 'description' for add action".into())
                        })?;
                    let id = commands_project::todo_add(desc);
                    format!("Added task #{id}: {desc}")
                }
                "done" => {
                    let id = params.get("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                        ToolError::InvalidArgs("Missing 'id' for done action".into())
                    })? as usize;
                    commands_project::todo_update(id, commands_project::TodoStatus::Done)
                        .map_err(ToolError::Failed)?;
                    format!("Task #{id} marked as done ✓")
                }
                "wip" => {
                    let id = params.get("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                        ToolError::InvalidArgs("Missing 'id' for wip action".into())
                    })? as usize;
                    commands_project::todo_update(id, commands_project::TodoStatus::InProgress)
                        .map_err(ToolError::Failed)?;
                    format!("Task #{id} marked as in-progress")
                }
                "remove" => {
                    let id = params.get("id").and_then(|v| v.as_u64()).ok_or_else(|| {
                        ToolError::InvalidArgs("Missing 'id' for remove action".into())
                    })? as usize;
                    let item = commands_project::todo_remove(id).map_err(ToolError::Failed)?;
                    format!("Removed task #{id}: {}", item.description)
                }
                "clear" => {
                    commands_project::todo_clear();
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

    let bash = if auto_approve {
        StreamingBashTool::default()
    } else {
        let flag = Arc::clone(&always_approved);
        let perms = permissions.clone();
        StreamingBashTool::default().with_confirm(move |cmd: &str| {
            // If user previously chose "always", skip the prompt
            if flag.load(Ordering::Relaxed) {
                eprintln!(
                    "{GREEN}  ✓ Auto-approved: {RESET}{}",
                    truncate_with_ellipsis(cmd, 120)
                );
                return true;
            }
            // Check permission patterns before prompting
            if let Some(allowed) = perms.check(cmd) {
                if allowed {
                    eprintln!(
                        "{GREEN}  ✓ Permitted: {RESET}{}",
                        truncate_with_ellipsis(cmd, 120)
                    );
                    return true;
                } else {
                    eprintln!(
                        "{RED}  ✗ Denied by permission rule: {RESET}{}",
                        truncate_with_ellipsis(cmd, 120)
                    );
                    return false;
                }
            }
            use std::io::BufRead;
            // Show the command and ask for approval
            eprint!(
                "{YELLOW}  ⚠ Allow: {RESET}{}{YELLOW} ? {RESET}({GREEN}y{RESET}/{RED}n{RESET}/{GREEN}a{RESET}lways) ",
                truncate_with_ellipsis(cmd, 120)
            );
            io::stderr().flush().ok();
            let mut response = String::new();
            let stdin = io::stdin();
            if stdin.lock().read_line(&mut response).is_err() {
                return false;
            }
            let response = response.trim().to_lowercase();
            let approved = matches!(response.as_str(), "y" | "yes" | "a" | "always");
            if matches!(response.as_str(), "a" | "always") {
                flag.store(true, Ordering::Relaxed);
                eprintln!(
                    "{GREEN}  ✓ All subsequent operations will be auto-approved this session.{RESET}"
                );
            }
            approved
        })
    };

    // Build write_file and edit_file with optional confirmation prompts
    let write_tool: Box<dyn AgentTool> = if auto_approve {
        maybe_guard(Box::new(WriteFileTool::new()), dir_restrictions)
    } else {
        maybe_guard(
            maybe_confirm(
                Box::new(WriteFileTool::new()),
                &always_approved,
                permissions,
            ),
            dir_restrictions,
        )
    };
    let edit_tool: Box<dyn AgentTool> = if auto_approve {
        maybe_guard(Box::new(EditFileTool::new()), dir_restrictions)
    } else {
        maybe_guard(
            maybe_confirm(Box::new(EditFileTool::new()), &always_approved, permissions),
            dir_restrictions,
        )
    };

    // Build rename_symbol tool with optional confirmation (it writes files)
    let rename_tool: Box<dyn AgentTool> = if auto_approve {
        Box::new(RenameSymbolTool)
    } else {
        maybe_confirm(Box::new(RenameSymbolTool), &always_approved, permissions)
    };

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
        maybe_hook(with_truncation(Box::new(bash), max_tool_output), &hooks),
        maybe_hook(
            with_truncation(
                maybe_guard(Box::new(ReadFileTool::default()), dir_restrictions),
                max_tool_output,
            ),
            &hooks,
        ),
        maybe_hook(with_truncation(write_tool, max_tool_output), &hooks),
        maybe_hook(with_truncation(edit_tool, max_tool_output), &hooks),
        maybe_hook(
            with_truncation(
                maybe_guard(Box::new(ListFilesTool::default()), dir_restrictions),
                max_tool_output,
            ),
            &hooks,
        ),
        maybe_hook(
            with_truncation(
                maybe_guard(Box::new(SearchTool::default()), dir_restrictions),
                max_tool_output,
            ),
            &hooks,
        ),
        maybe_hook(with_truncation(rename_tool, max_tool_output), &hooks),
    ];

    // Only add ask_user in interactive mode (stdin is a terminal).
    // In piped mode or test environments, this tool isn't available.
    if std::io::stdin().is_terminal() {
        tools.push(maybe_hook(Box::new(AskUserTool), &hooks));
    }

    // TodoTool is always available — it only modifies in-memory state, not filesystem
    tools.push(maybe_hook(Box::new(TodoTool), &hooks));

    tools
}

/// Build a SubAgentTool that inherits the parent's provider/model/key.
/// The sub-agent gets basic tools with inherited directory restrictions
/// (no permission prompts, no sub-agent recursion).
pub(crate) fn build_sub_agent_tool(config: &AgentConfig) -> SubAgentTool {
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
        maybe_guard_arc(Arc::new(SearchTool::default()), restrictions),
    ];

    // Select the right provider
    let provider: Arc<dyn StreamProvider> = match config.provider.as_str() {
        "anthropic" => Arc::new(AnthropicProvider),
        "google" => Arc::new(GoogleProvider),
        "bedrock" => Arc::new(BedrockProvider),
        _ => Arc::new(OpenAiCompatProvider),
    };

    SubAgentTool::new("sub_agent", provider)
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands_refactor;
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
        }
    }

    #[test]
    fn test_build_tools_returns_eight_tools() {
        // build_tools should return 8 tools regardless of auto_approve (in non-terminal: no ask_user)
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools_approved = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        let tools_confirm = build_tools(false, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(tools_approved.len(), 8);
        assert_eq!(tools_confirm.len(), 8);
    }

    #[test]
    fn test_build_sub_agent_tool_returns_correct_name() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let tool = build_sub_agent_tool(&config);
        assert_eq!(tool.name(), "sub_agent");
    }

    #[test]
    fn test_build_sub_agent_tool_has_task_parameter() {
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        let tool = build_sub_agent_tool(&config);
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
        let _tool_anthropic =
            build_sub_agent_tool(&test_agent_config("anthropic", "claude-sonnet-4-20250514"));
        let _tool_google = build_sub_agent_tool(&test_agent_config("google", "gemini-2.0-flash"));
        let _tool_openai = build_sub_agent_tool(&test_agent_config("openai", "gpt-4o"));
        let _tool_bedrock = build_sub_agent_tool(&test_agent_config(
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
        let tool = build_sub_agent_tool(&config);
        assert_eq!(tool.name(), "sub_agent");
    }

    #[test]
    fn test_build_sub_agent_tool_no_restrictions_still_works() {
        // Empty restrictions shouldn't break sub-agent building
        let config = test_agent_config("anthropic", "claude-sonnet-4-20250514");
        assert!(config.dir_restrictions.is_empty());
        let tool = build_sub_agent_tool(&config);
        assert_eq!(tool.name(), "sub_agent");
    }

    #[test]
    fn test_build_tools_count_unchanged_with_sub_agent() {
        // Verify build_tools still returns exactly 8 — SubAgentTool is added via with_sub_agent
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(
            tools.len(),
            8,
            "build_tools must stay at 8 — SubAgentTool is added via with_sub_agent"
        );
    }

    // === File operation confirmation tests ===

    #[test]
    fn test_describe_write_file_operation() {
        let params = serde_json::json!({
            "path": "src/main.rs",
            "content": "line1\nline2\nline3\n"
        });
        let desc = describe_file_operation("write_file", &params);
        assert!(desc.contains("write:"));
        assert!(desc.contains("src/main.rs"));
        assert!(desc.contains("3 lines")); // Rust's .lines() strips trailing newline
    }

    #[test]
    fn test_describe_write_file_empty_content() {
        let params = serde_json::json!({
            "path": "empty.txt",
            "content": ""
        });
        let desc = describe_file_operation("write_file", &params);
        assert!(desc.contains("write:"));
        assert!(desc.contains("empty.txt"));
        assert!(
            desc.contains("EMPTY content"),
            "Empty content should show warning, got: {desc}"
        );
    }

    #[test]
    fn test_describe_write_file_missing_content() {
        // When the content key is entirely absent (model bug), treat as empty
        let params = serde_json::json!({
            "path": "missing.txt"
        });
        let desc = describe_file_operation("write_file", &params);
        assert!(desc.contains("write:"));
        assert!(desc.contains("missing.txt"));
        assert!(
            desc.contains("EMPTY content"),
            "Missing content should show warning, got: {desc}"
        );
    }

    #[test]
    fn test_describe_write_file_normal_content() {
        // Normal write_file should NOT show the empty warning
        let params = serde_json::json!({
            "path": "hello.txt",
            "content": "hello world\n"
        });
        let desc = describe_file_operation("write_file", &params);
        assert!(desc.contains("write:"));
        assert!(desc.contains("hello.txt"));
        assert!(desc.contains("1 line"));
        assert!(
            !desc.contains("EMPTY"),
            "Non-empty content should not show warning, got: {desc}"
        );
    }

    #[test]
    fn test_describe_edit_file_operation() {
        let params = serde_json::json!({
            "path": "src/cli.rs",
            "old_text": "old line 1\nold line 2",
            "new_text": "new line 1\nnew line 2\nnew line 3"
        });
        let desc = describe_file_operation("edit_file", &params);
        assert!(desc.contains("edit:"));
        assert!(desc.contains("src/cli.rs"));
        assert!(desc.contains("2 → 3 lines"));
    }

    #[test]
    fn test_describe_edit_file_missing_params() {
        let params = serde_json::json!({
            "path": "test.rs"
        });
        let desc = describe_file_operation("edit_file", &params);
        assert!(desc.contains("edit:"));
        assert!(desc.contains("test.rs"));
        assert!(desc.contains("0 → 0 lines"));
    }

    #[test]
    fn test_describe_unknown_tool() {
        let params = serde_json::json!({});
        let desc = describe_file_operation("unknown_tool", &params);
        assert!(desc.contains("unknown_tool"));
    }

    #[test]
    fn test_confirm_file_operation_auto_approved_flag() {
        // When always_approved is true, confirm should return true immediately
        let flag = Arc::new(AtomicBool::new(true));
        let perms = cli::PermissionConfig::default();
        let result = confirm_file_operation("write: test.rs (5 lines)", "test.rs", &flag, &perms);
        assert!(
            result,
            "Should auto-approve when always_approved flag is set"
        );
    }

    #[test]
    fn test_confirm_file_operation_with_allow_pattern() {
        // Permission patterns should match file paths
        let flag = Arc::new(AtomicBool::new(false));
        let perms = cli::PermissionConfig {
            allow: vec!["*.md".to_string()],
            deny: vec![],
        };
        let result =
            confirm_file_operation("write: README.md (10 lines)", "README.md", &flag, &perms);
        assert!(result, "Should auto-approve paths matching allow pattern");
    }

    #[test]
    fn test_confirm_file_operation_with_deny_pattern() {
        // Denied patterns should block the operation
        let flag = Arc::new(AtomicBool::new(false));
        let perms = cli::PermissionConfig {
            allow: vec![],
            deny: vec!["*.key".to_string()],
        };
        let result =
            confirm_file_operation("write: secrets.key (1 line)", "secrets.key", &flag, &perms);
        assert!(!result, "Should deny paths matching deny pattern");
    }

    #[test]
    fn test_confirm_file_operation_deny_overrides_allow() {
        // Deny takes priority over allow
        let flag = Arc::new(AtomicBool::new(false));
        let perms = cli::PermissionConfig {
            allow: vec!["*".to_string()],
            deny: vec!["*.key".to_string()],
        };
        let result =
            confirm_file_operation("write: secrets.key (1 line)", "secrets.key", &flag, &perms);
        assert!(!result, "Deny should override allow");
    }

    #[test]
    fn test_confirm_file_operation_allow_src_pattern() {
        // Realistic pattern: allow all files under src/
        let flag = Arc::new(AtomicBool::new(false));
        let perms = cli::PermissionConfig {
            allow: vec!["src/*".to_string()],
            deny: vec![],
        };
        let result = confirm_file_operation(
            "edit: src/main.rs (2 → 3 lines)",
            "src/main.rs",
            &flag,
            &perms,
        );
        assert!(
            result,
            "Should auto-approve src/ files with 'src/*' pattern"
        );
    }

    #[test]
    fn test_build_tools_auto_approve_skips_confirmation() {
        // When auto_approve is true, tools should not have ConfirmTool wrappers
        let perms = cli::PermissionConfig::default();
        let dirs = cli::DirectoryRestrictions::default();
        let tools = build_tools(true, &perms, &dirs, TOOL_OUTPUT_MAX_CHARS, false, vec![]);
        assert_eq!(tools.len(), 8);
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
        assert_eq!(tools.len(), 8);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"edit_file"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"search"));
        assert!(names.contains(&"todo"));
    }

    #[test]
    fn test_always_approved_shared_between_bash_and_file_tools() {
        // Simulates: user says "always" on a bash prompt,
        // subsequent file operations should auto-approve too.
        // This test verifies the shared flag concept.
        let always_approved = Arc::new(AtomicBool::new(false));
        let bash_flag = Arc::clone(&always_approved);
        let file_flag = Arc::clone(&always_approved);

        // Initially, nothing is auto-approved
        assert!(!bash_flag.load(Ordering::Relaxed));
        assert!(!file_flag.load(Ordering::Relaxed));

        // User says "always" on a bash command
        bash_flag.store(true, Ordering::Relaxed);

        // File tool should now see the flag as true
        assert!(
            file_flag.load(Ordering::Relaxed),
            "File tool should see always_approved after bash 'always'"
        );
    }

    // -----------------------------------------------------------------------
    // StreamingBashTool tests
    // -----------------------------------------------------------------------

    /// Create a ToolContext for testing, with an optional on_update callback
    /// that collects partial results.
    fn test_tool_context(
        updates: Option<Arc<tokio::sync::Mutex<Vec<yoagent::types::ToolResult>>>>,
    ) -> yoagent::types::ToolContext {
        let on_update: Option<yoagent::types::ToolUpdateFn> = updates.map(|u| {
            Arc::new(move |result: yoagent::types::ToolResult| {
                // Use try_lock to avoid blocking in sync callback
                if let Ok(mut guard) = u.try_lock() {
                    guard.push(result);
                }
            }) as yoagent::types::ToolUpdateFn
        });
        yoagent::types::ToolContext {
            tool_call_id: "test-id".to_string(),
            tool_name: "bash".to_string(),
            cancel: tokio_util::sync::CancellationToken::new(),
            on_update,
            on_progress: None,
        }
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
                assert!(text.contains("Exit code: 0"));
            }
            _ => panic!("Expected text content"),
        }
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
                assert!(text.contains("Exit code: 0"));
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
                // Total text = "Exit code: 0\n" + accumulated (which was truncated to ~100 bytes)
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
        let result = commands_refactor::RenameResult {
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
    fn test_describe_rename_symbol_operation() {
        let params = serde_json::json!({
            "old_name": "FooBar",
            "new_name": "BazQux",
            "path": "src/"
        });
        let desc = describe_file_operation("rename_symbol", &params);
        assert!(desc.contains("FooBar"), "Should contain old_name: {desc}");
        assert!(desc.contains("BazQux"), "Should contain new_name: {desc}");
        assert!(desc.contains("src/"), "Should contain scope: {desc}");
    }

    #[test]
    fn test_describe_rename_symbol_no_path() {
        let params = serde_json::json!({
            "old_name": "Foo",
            "new_name": "Bar"
        });
        let desc = describe_file_operation("rename_symbol", &params);
        assert!(
            desc.contains("project"),
            "Should default to 'project': {desc}"
        );
    }

    #[test]
    fn test_truncate_result_with_custom_limit() {
        use yoagent::types::{Content, ToolResult};
        // Create a ToolResult with text longer than 100 chars and enough lines.
        // Each line starts with a unique first word to avoid compression collapsing.
        let long_text = (0..200)
            .map(|i| format!("T{i} data"))
            .collect::<Vec<_>>()
            .join("\n");
        let result = ToolResult {
            content: vec![Content::Text {
                text: long_text.clone(),
            }],
            details: serde_json::Value::Null,
        };
        let truncated = truncate_result(result, 100);
        let text = match &truncated.content[0] {
            Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(
            text.contains("[... truncated"),
            "Result should be truncated with 100-char limit"
        );
    }

    #[test]
    fn test_truncate_result_preserves_under_limit() {
        use yoagent::types::{Content, ToolResult};
        let short_text = "hello world".to_string();
        let result = ToolResult {
            content: vec![Content::Text {
                text: short_text.clone(),
            }],
            details: serde_json::Value::Null,
        };
        let truncated = truncate_result(result, TOOL_OUTPUT_MAX_CHARS);
        let text = match &truncated.content[0] {
            Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert_eq!(text, short_text, "Short text should be unchanged");
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
        assert_eq!(tools.len(), 8, "Should still have 8 tools with piped limit");
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
        commands_project::todo_clear();
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
        commands_project::todo_clear();
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
        commands_project::todo_clear();
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
        // Without a timeout param, the schema should use the default (120s)
        let schema = tool.parameters_schema();
        let props = schema["properties"].as_object().unwrap();
        assert!(
            props.contains_key("timeout"),
            "Schema should include timeout parameter"
        );
        // Verify the default timeout is 120s by checking the struct field
        assert_eq!(tool.timeout, Duration::from_secs(120));
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

    // --- RTK integration tests ---

    #[test]
    fn test_detect_rtk_returns_bool() {
        // In CI, RTK is likely not installed, so this should return false
        // The important thing is it doesn't panic
        let _result: bool = detect_rtk();
    }

    #[test]
    fn test_maybe_prefix_rtk_when_disabled() {
        // Disable RTK for this test
        RTK_DISABLED.store(true, Ordering::Relaxed);
        assert_eq!(maybe_prefix_rtk("git status"), "git status");
        assert_eq!(maybe_prefix_rtk("cargo test"), "cargo test");
        // Re-enable for other tests
        RTK_DISABLED.store(false, Ordering::Relaxed);
    }

    #[test]
    fn test_maybe_prefix_rtk_no_double_prefix() {
        // Even if RTK were available, shouldn't double-prefix
        // (This works regardless of RTK availability)
        let cmd = "rtk git status";
        let result = maybe_prefix_rtk(cmd);
        assert_eq!(result, "rtk git status");
    }

    #[test]
    fn test_maybe_prefix_rtk_complex_commands_not_prefixed() {
        // These should never be prefixed regardless of RTK availability
        RTK_DISABLED.store(false, Ordering::Relaxed);
        // Force RTK "available" for this test by checking the logic path
        // Since we can't fake RTK being available, test the is_simple_command helper
        assert!(!is_simple_command("git status | grep main"));
        assert!(!is_simple_command("echo hello && cargo test"));
        assert!(!is_simple_command("ls; rm -rf /"));
        assert!(!is_simple_command("cat file > output.txt"));
        assert!(!is_simple_command("sort < input.txt"));
        assert!(!is_simple_command("cmd1 & cmd2"));
    }

    #[test]
    fn test_is_simple_command_positive() {
        assert!(is_simple_command("git status"));
        assert!(is_simple_command("cargo test --release"));
        assert!(is_simple_command("ls -la"));
        assert!(is_simple_command("echo hello"));
        assert!(is_simple_command("grep -r pattern ."));
    }

    #[test]
    fn test_is_simple_command_quoted_metacharacters() {
        // Pipes/redirects inside quotes should NOT break simplicity
        assert!(is_simple_command("echo 'hello | world'"));
        assert!(is_simple_command("grep \"pattern > here\""));
        assert!(is_simple_command("echo 'a && b'"));
    }

    #[test]
    fn test_maybe_prefix_rtk_unsupported_commands() {
        // These commands are not in the RTK supported list
        // Even if RTK is installed, they shouldn't be prefixed
        // We test the logic by checking the supported list directly
        assert!(!RTK_SUPPORTED_COMMANDS.contains(&"echo"));
        assert!(!RTK_SUPPORTED_COMMANDS.contains(&"cd"));
        assert!(!RTK_SUPPORTED_COMMANDS.contains(&"python"));
        assert!(!RTK_SUPPORTED_COMMANDS.contains(&"python3"));
        assert!(!RTK_SUPPORTED_COMMANDS.contains(&"ruby"));
        assert!(!RTK_SUPPORTED_COMMANDS.contains(&"node"));
    }

    #[test]
    fn test_rtk_supported_commands_includes_expected() {
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"git"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"ls"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"cargo"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"npm"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"docker"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"kubectl"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"grep"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"find"));
        assert!(RTK_SUPPORTED_COMMANDS.contains(&"gh"));
    }

    #[test]
    fn test_maybe_prefix_rtk_with_env_var_prefix() {
        // Commands with env var assignments before the actual command
        // The function should skip env vars and find the actual command
        // Testing indirectly: "FOO=bar git status" - base_cmd should be "git"
        // Since RTK may not be installed in CI, we just verify the logic doesn't panic
        let _ = maybe_prefix_rtk("FOO=bar git status");
        let _ = maybe_prefix_rtk("HOME=/tmp ls");
    }
}
