//! Tool decorator types that add behavior around any tool.
//!
//! These are generic wrappers — they don't know about specific tool implementations,
//! only about the `AgentTool` trait. Each adds one concern:
//! - `GuardedTool` — directory restriction enforcement
//! - `TruncatingTool` — output truncation for context window savings
//! - `ConfirmTool` — user confirmation before write/edit operations
//! - `ArcGuardedTool` — directory restrictions for `Arc<dyn AgentTool>` (sub-agents)
//!
//! Helper functions (`maybe_guard`, `maybe_confirm`, `with_truncation`, `maybe_guard_arc`)
//! conditionally wrap tools based on configuration.

use crate::cli;
use crate::format::*;

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use yoagent::types::AgentTool;

// ---------------------------------------------------------------------------
// GuardedTool — directory restriction wrapper (Box-based)
// ---------------------------------------------------------------------------

/// A wrapper tool that checks directory restrictions before delegating to an inner tool.
/// Intercepts the `"path"` parameter from tool arguments and validates it against
/// the configured `DirectoryRestrictions`. If the path is blocked, the tool returns
/// an error without executing the inner tool.
pub(crate) struct GuardedTool {
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

/// Wrap a tool with directory restrictions if any are configured.
pub(crate) fn maybe_guard(
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

// ---------------------------------------------------------------------------
// ArcGuardedTool — directory restriction wrapper (Arc-based, for sub-agents)
// ---------------------------------------------------------------------------

/// A wrapper tool that checks directory restrictions before delegating to an Arc-wrapped inner tool.
/// Used by sub-agents to inherit the parent's directory restrictions without needing Box ownership.
pub(crate) struct ArcGuardedTool {
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
pub(crate) fn maybe_guard_arc(
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

// ---------------------------------------------------------------------------
// TruncatingTool — output truncation wrapper
// ---------------------------------------------------------------------------

/// A wrapper tool that truncates large tool output to save context window tokens.
/// When tool output exceeds the configured `max_chars`, preserves the first ~100 and
/// last ~50 lines with a clear truncation marker in between.
pub(crate) struct TruncatingTool {
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
pub(crate) fn with_truncation(tool: Box<dyn AgentTool>, max_chars: usize) -> Box<dyn AgentTool> {
    Box::new(TruncatingTool {
        inner: tool,
        max_chars,
    })
}

// ---------------------------------------------------------------------------
// ConfirmTool — user confirmation wrapper for file operations
// ---------------------------------------------------------------------------

/// A wrapper tool that prompts for user confirmation before executing write_file or edit_file.
/// Shares the same `always_approved` flag with bash confirmation so "always" applies everywhere.
/// Checks `--allow`/`--deny` patterns against file paths before prompting.
pub(crate) struct ConfirmTool {
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

/// Maximum combined lines (old_text + new_text) before the diff preview is truncated.
const EDIT_DIFF_MAX_LINES: usize = 40;

/// Generate a colored diff preview for an `edit_file` operation.
///
/// Extracts `old_text` and `new_text` from the tool params and returns a
/// formatted diff string using the LCS-based diff renderer. Returns an empty
/// string when both texts are identical or when the params are missing.
///
/// If the combined old+new text exceeds `EDIT_DIFF_MAX_LINES`, the diff is
/// truncated with a `... (N more lines)` ellipsis.
pub fn format_edit_diff_preview(params: &serde_json::Value) -> String {
    let old_text = params
        .get("old_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let new_text = params
        .get("new_text")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if old_text.is_empty() && new_text.is_empty() {
        return String::new();
    }

    let diff = crate::format::format_edit_diff(old_text, new_text);
    if diff.is_empty() {
        return diff;
    }

    // Apply additional truncation for very large diffs
    let total_input_lines = old_text.lines().count() + new_text.lines().count();
    if total_input_lines > EDIT_DIFF_MAX_LINES {
        crate::format::truncate_diff_preview(&diff, 20)
    } else {
        diff
    }
}

/// Prompt the user to confirm a file operation, optionally showing a diff preview.
///
/// When `diff_preview` is `Some(text)`, the colored diff is printed to stderr
/// before the interactive confirmation prompt. The diff is NOT shown for
/// auto-approved or permission-approved operations.
pub fn confirm_file_operation(
    description: &str,
    path: &str,
    always_approved: &Arc<AtomicBool>,
    permissions: &cli::PermissionConfig,
    diff_preview: Option<&str>,
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
    // Show the diff preview before the confirmation prompt (if available)
    if let Some(diff) = diff_preview {
        if !diff.is_empty() {
            eprintln!("{}", diff);
        }
    }
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

        // Generate a diff preview for edit_file operations
        let diff_preview = if tool_name == "edit_file" {
            let preview = format_edit_diff_preview(&params);
            if preview.is_empty() {
                None
            } else {
                Some(preview)
            }
        } else {
            None
        };

        if !confirm_file_operation(
            &description,
            path,
            &self.always_approved,
            &self.permissions,
            diff_preview.as_deref(),
        ) {
            return Err(yoagent::types::ToolError::Failed(format!(
                "User denied {tool_name} on '{path}'"
            )));
        }
        self.inner.execute(params, ctx).await
    }
}

/// Wrap a tool with a confirmation prompt for write/edit operations.
pub(crate) fn maybe_confirm(
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
// AutoCheckTool — runs check command after successful file edits
// ---------------------------------------------------------------------------

/// Maximum characters of auto-check output to append to tool results.
const AUTO_CHECK_MAX_CHARS: usize = 2000;

/// A tool wrapper that automatically runs a check command after file edits.
/// When a watch command is configured (via `/watch set`), it runs the first
/// watch phase (typically lint) after successful write_file or edit_file
/// operations and appends any errors to the tool result.
///
/// This gives the agent immediate compilation feedback inline with each edit,
/// catching errors before moving on to the next file — similar to how Aider
/// runs lint+test after each individual file write.
pub(crate) struct AutoCheckTool {
    inner: Box<dyn AgentTool>,
}

#[async_trait::async_trait]
impl AgentTool for AutoCheckTool {
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

        // Only run check when a watch command is active
        let commands = crate::watch::get_watch_commands();
        if commands.is_empty() {
            return Ok(result);
        }

        // Use only the first phase (typically lint/check, not the full test suite)
        let check_cmd = &commands[0];
        let (passed, output) = crate::watch::run_watch_command(check_cmd);

        if passed {
            return Ok(result);
        }

        // Append check failure output to the tool result
        let truncated_output = if output.len() > AUTO_CHECK_MAX_CHARS {
            // Find safe char boundary for truncation
            let mut b = AUTO_CHECK_MAX_CHARS;
            while b > 0 && !output.is_char_boundary(b) {
                b -= 1;
            }
            format!(
                "{}...\n[auto-check output truncated at {AUTO_CHECK_MAX_CHARS} chars]",
                &output[..b]
            )
        } else {
            output
        };

        let check_notice = format!("\n\n⚠ Auto-check failed ({check_cmd}):\n{truncated_output}");

        // Append the check notice to each text content block
        let new_content = result
            .content
            .into_iter()
            .map(|c| match c {
                yoagent::Content::Text { text } => yoagent::Content::Text {
                    text: format!("{text}{check_notice}"),
                },
                other => other,
            })
            .collect();

        Ok(yoagent::types::ToolResult {
            content: new_content,
            details: result.details,
        })
    }
}

/// Wrap a tool with auto-check: runs the watch command after successful edits
/// and appends any errors to the tool result for immediate feedback.
pub(crate) fn with_auto_check(tool: Box<dyn AgentTool>) -> Box<dyn AgentTool> {
    Box::new(AutoCheckTool { inner: tool })
}

// ---------------------------------------------------------------------------
// RecoveryHintTool — appends recovery hints to tool error messages
// ---------------------------------------------------------------------------

/// Tracks consecutive failures per tool name so recovery hints can escalate.
///
/// Shared across all tools in a session — when tool A fails 3 times,
/// the hint it gets is more aggressive than on its first failure.
/// When a tool succeeds, its counter resets.
#[derive(Clone, Default)]
pub(crate) struct ToolFailureTracker {
    counts: Arc<Mutex<HashMap<String, u32>>>,
}

#[allow(dead_code)] // Public API — wired in a follow-up task
impl ToolFailureTracker {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Increment the failure count for a tool and return the new count.
    fn record_failure(&self, tool_name: &str) -> u32 {
        let mut map = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        let count = map.entry(tool_name.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Reset the failure count for a tool (called on success).
    fn record_success(&self, tool_name: &str) {
        let mut map = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(tool_name);
    }

    /// Get the current failure count for a tool (for testing).
    #[cfg(test)]
    fn get(&self, tool_name: &str) -> u32 {
        let map = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        map.get(tool_name).copied().unwrap_or(0)
    }
}

/// A wrapper tool that enriches error messages with recovery hints.
///
/// On success the failure counter resets. On failure the counter increments
/// and a tool-specific recovery hint (from `prompt_retry::tool_recovery_hint`)
/// is appended to the error message.
#[allow(dead_code)] // Public API — wired in a follow-up task
pub(crate) struct RecoveryHintTool {
    inner: Box<dyn AgentTool>,
    tracker: ToolFailureTracker,
}

#[async_trait::async_trait]
impl AgentTool for RecoveryHintTool {
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
        let tool_name = self.inner.name().to_string();
        match self.inner.execute(params, ctx).await {
            Ok(result) => {
                self.tracker.record_success(&tool_name);
                Ok(result)
            }
            Err(yoagent::types::ToolError::Failed(msg)) => {
                let attempt = self.tracker.record_failure(&tool_name);
                let hint = crate::prompt_retry::tool_recovery_hint(&tool_name, attempt);
                Err(yoagent::types::ToolError::Failed(format!(
                    "{msg}\n\n💡 Recovery hint: {hint}"
                )))
            }
            Err(other) => {
                // Non-Failed errors (NotFound, InvalidArgs, Cancelled) pass through
                // but still count as failures for escalation purposes
                self.tracker.record_failure(&tool_name);
                Err(other)
            }
        }
    }
}

/// Wrap a tool with recovery hints on failure. The `tracker` is shared across
/// all tools so consecutive failures of the same tool escalate the advice.
#[allow(dead_code)] // Public API — wired in a follow-up task
pub(crate) fn with_recovery_hints(
    tool: Box<dyn AgentTool>,
    tracker: &ToolFailureTracker,
) -> Box<dyn AgentTool> {
    Box::new(RecoveryHintTool {
        inner: tool,
        tracker: tracker.clone(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    // === describe_file_operation tests ===

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

    // === format_edit_diff_preview tests ===

    #[test]
    fn test_edit_diff_preview_basic_change() {
        let params = serde_json::json!({
            "path": "src/main.rs",
            "old_text": "let x = 1;",
            "new_text": "let x = 2;"
        });
        let preview = format_edit_diff_preview(&params);
        assert!(!preview.is_empty(), "Should produce a diff preview");
        assert!(
            preview.contains("- let x = 1;"),
            "Should show removed line: {preview}"
        );
        assert!(
            preview.contains("+ let x = 2;"),
            "Should show added line: {preview}"
        );
    }

    #[test]
    fn test_edit_diff_preview_multiline() {
        let params = serde_json::json!({
            "path": "src/lib.rs",
            "old_text": "fn foo() {\n    println!(\"old\");\n}",
            "new_text": "fn foo() {\n    println!(\"new\");\n    println!(\"extra\");\n}"
        });
        let preview = format_edit_diff_preview(&params);
        assert!(preview.contains("- "), "Should have removed lines");
        assert!(preview.contains("+ "), "Should have added lines");
        assert!(preview.contains("new"), "Should show new content");
        assert!(preview.contains("extra"), "Should show extra line");
    }

    #[test]
    fn test_edit_diff_preview_identical_texts() {
        let params = serde_json::json!({
            "path": "src/main.rs",
            "old_text": "same text",
            "new_text": "same text"
        });
        let preview = format_edit_diff_preview(&params);
        assert!(
            preview.is_empty(),
            "Identical texts should produce empty preview"
        );
    }

    #[test]
    fn test_edit_diff_preview_missing_params() {
        let params = serde_json::json!({
            "path": "src/main.rs"
        });
        let preview = format_edit_diff_preview(&params);
        assert!(
            preview.is_empty(),
            "Missing old_text/new_text should produce empty preview"
        );
    }

    #[test]
    fn test_edit_diff_preview_empty_old_text() {
        let params = serde_json::json!({
            "path": "src/main.rs",
            "old_text": "",
            "new_text": "new line 1\nnew line 2"
        });
        let preview = format_edit_diff_preview(&params);
        assert!(
            !preview.is_empty(),
            "Adding new content should produce preview"
        );
        assert!(
            preview.contains("+ new line 1"),
            "Should show additions: {preview}"
        );
        assert!(
            !preview.contains("- "),
            "Should have no removals for pure addition"
        );
    }

    #[test]
    fn test_edit_diff_preview_empty_new_text() {
        let params = serde_json::json!({
            "path": "src/main.rs",
            "old_text": "old line 1\nold line 2",
            "new_text": ""
        });
        let preview = format_edit_diff_preview(&params);
        assert!(
            !preview.is_empty(),
            "Deleting content should produce preview"
        );
        assert!(
            preview.contains("- old line 1"),
            "Should show deletions: {preview}"
        );
        assert!(
            !preview.contains("+ "),
            "Should have no additions for pure deletion"
        );
    }

    #[test]
    fn test_edit_diff_preview_truncates_large_diff() {
        // Generate old_text and new_text that together exceed EDIT_DIFF_MAX_LINES (40)
        let old_lines: Vec<String> = (0..25).map(|i| format!("old line {i}")).collect();
        let new_lines: Vec<String> = (0..25).map(|i| format!("new line {i}")).collect();
        let params = serde_json::json!({
            "path": "src/big.rs",
            "old_text": old_lines.join("\n"),
            "new_text": new_lines.join("\n")
        });
        let preview = format_edit_diff_preview(&params);
        assert!(
            !preview.is_empty(),
            "Large diff should still produce preview"
        );
        // The preview should be truncated (the combined 50 lines exceeds the 40-line threshold)
        assert!(
            preview.contains("more lines"),
            "Large diff should be truncated with ellipsis: {preview}"
        );
    }

    #[test]
    fn test_edit_diff_preview_small_diff_not_truncated() {
        let params = serde_json::json!({
            "path": "src/main.rs",
            "old_text": "line 1\nline 2\nline 3",
            "new_text": "line 1\nmodified\nline 3"
        });
        let preview = format_edit_diff_preview(&params);
        // 6 total input lines — well under the 40-line threshold
        assert!(!preview.is_empty());
        assert!(
            !preview.contains("more lines"),
            "Small diff should not be truncated: {preview}"
        );
    }

    // === confirm_file_operation tests ===

    #[test]
    fn test_confirm_file_operation_auto_approved_flag() {
        // When always_approved is true, confirm should return true immediately
        let flag = Arc::new(AtomicBool::new(true));
        let perms = cli::PermissionConfig::default();
        let result =
            confirm_file_operation("write: test.rs (5 lines)", "test.rs", &flag, &perms, None);
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
        let result = confirm_file_operation(
            "write: README.md (10 lines)",
            "README.md",
            &flag,
            &perms,
            None,
        );
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
        let result = confirm_file_operation(
            "write: secrets.key (1 line)",
            "secrets.key",
            &flag,
            &perms,
            None,
        );
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
        let result = confirm_file_operation(
            "write: secrets.key (1 line)",
            "secrets.key",
            &flag,
            &perms,
            None,
        );
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
            None,
        );
        assert!(
            result,
            "Should auto-approve src/ files with 'src/*' pattern"
        );
    }

    // === Shared approval flag test ===

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

    // === describe_file_operation: rename_symbol ===

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

    // === truncate_result tests ===

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
        use crate::format::TOOL_OUTPUT_MAX_CHARS;
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

    // === AutoCheckTool tests ===

    /// A simple mock tool that always succeeds with the given text.
    struct MockTool {
        tool_name: &'static str,
        result_text: String,
    }

    #[async_trait::async_trait]
    impl AgentTool for MockTool {
        fn name(&self) -> &str {
            self.tool_name
        }
        fn label(&self) -> &str {
            self.tool_name
        }
        fn description(&self) -> &str {
            "mock tool"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: yoagent::types::ToolContext,
        ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
            Ok(yoagent::types::ToolResult {
                content: vec![yoagent::Content::Text {
                    text: self.result_text.clone(),
                }],
                details: serde_json::Value::Null,
            })
        }
    }

    fn test_tool_context() -> yoagent::types::ToolContext {
        yoagent::types::ToolContext {
            tool_call_id: "test".to_string(),
            tool_name: "test".to_string(),
            cancel: tokio_util::sync::CancellationToken::new(),
            on_update: None,
            on_progress: None,
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_auto_check_passthrough_no_watch_command() {
        // Clear any watch commands to ensure passthrough
        crate::watch::clear_watch_command();

        let tool = with_auto_check(Box::new(MockTool {
            tool_name: "write_file",
            result_text: "File written successfully.".to_string(),
        }));

        let result = tool
            .execute(serde_json::json!({}), test_tool_context())
            .await
            .unwrap();

        let text = match &result.content[0] {
            yoagent::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };
        assert_eq!(text, "File written successfully.");
        assert!(
            !text.contains("Auto-check"),
            "Should not contain check output when no watch command"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_auto_check_appends_failure_output() {
        // Set a watch command that always fails
        crate::watch::set_watch_command("echo 'error[E0433]: module not found' && exit 1");

        let tool = with_auto_check(Box::new(MockTool {
            tool_name: "edit_file",
            result_text: "Edit applied.".to_string(),
        }));

        let result = tool
            .execute(serde_json::json!({}), test_tool_context())
            .await
            .unwrap();

        let text = match &result.content[0] {
            yoagent::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };

        // Clean up
        crate::watch::clear_watch_command();

        assert!(
            text.starts_with("Edit applied."),
            "Should start with original result"
        );
        assert!(
            text.contains("⚠ Auto-check failed"),
            "Should contain check failure notice"
        );
        assert!(
            text.contains("error[E0433]"),
            "Should contain the actual error output"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_auto_check_silent_on_success() {
        // Set a watch command that succeeds
        crate::watch::set_watch_command("true");

        let tool = with_auto_check(Box::new(MockTool {
            tool_name: "write_file",
            result_text: "File written successfully.".to_string(),
        }));

        let result = tool
            .execute(serde_json::json!({}), test_tool_context())
            .await
            .unwrap();

        let text = match &result.content[0] {
            yoagent::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };

        // Clean up
        crate::watch::clear_watch_command();

        assert_eq!(
            text, "File written successfully.",
            "Should pass through unchanged when check passes"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_auto_check_truncates_long_output() {
        // Set a watch command that produces output longer than AUTO_CHECK_MAX_CHARS
        // Generate ~3000 chars of output
        let long_cmd = "python3 -c \"print('x' * 3000)\" && exit 1";
        crate::watch::set_watch_command(long_cmd);

        let tool = with_auto_check(Box::new(MockTool {
            tool_name: "write_file",
            result_text: "OK".to_string(),
        }));

        let result = tool
            .execute(serde_json::json!({}), test_tool_context())
            .await
            .unwrap();

        let text = match &result.content[0] {
            yoagent::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };

        // Clean up
        crate::watch::clear_watch_command();

        assert!(
            text.contains("auto-check output truncated"),
            "Long output should be truncated: {text}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_auto_check_uses_first_phase_only() {
        // Set multi-phase watch commands — only first phase should run
        crate::watch::set_watch_commands(&[
            "echo 'lint phase' && exit 1",
            "echo 'test phase' && exit 1",
        ]);

        let tool = with_auto_check(Box::new(MockTool {
            tool_name: "write_file",
            result_text: "OK".to_string(),
        }));

        let result = tool
            .execute(serde_json::json!({}), test_tool_context())
            .await
            .unwrap();

        let text = match &result.content[0] {
            yoagent::Content::Text { text } => text.clone(),
            _ => panic!("Expected text content"),
        };

        // Clean up
        crate::watch::clear_watch_command();

        assert!(
            text.contains("lint phase"),
            "Should run first phase: {text}"
        );
        assert!(
            !text.contains("test phase"),
            "Should NOT run second phase: {text}"
        );
    }

    // === RecoveryHintTool tests ===

    /// A mock tool that can be configured to succeed or fail.
    struct ConfigurableMockTool {
        tool_name: &'static str,
        /// When `Some(msg)`, execute returns `ToolError::Failed(msg)`.
        /// When `None`, execute succeeds with "ok".
        fail_msg: Option<String>,
    }

    #[async_trait::async_trait]
    impl AgentTool for ConfigurableMockTool {
        fn name(&self) -> &str {
            self.tool_name
        }
        fn label(&self) -> &str {
            self.tool_name
        }
        fn description(&self) -> &str {
            "configurable mock"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: yoagent::types::ToolContext,
        ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
            if let Some(ref msg) = self.fail_msg {
                Err(yoagent::types::ToolError::Failed(msg.clone()))
            } else {
                Ok(yoagent::types::ToolResult {
                    content: vec![yoagent::Content::Text {
                        text: "ok".to_string(),
                    }],
                    details: serde_json::Value::Null,
                })
            }
        }
    }

    #[tokio::test]
    async fn test_recovery_hint_tool_success_resets_counter() {
        let tracker = ToolFailureTracker::new();

        // Manually seed a failure count
        assert_eq!(tracker.record_failure("bash"), 1);
        assert_eq!(tracker.record_failure("bash"), 2);
        assert_eq!(tracker.get("bash"), 2);

        // Wrap a succeeding tool
        let tool = with_recovery_hints(
            Box::new(ConfigurableMockTool {
                tool_name: "bash",
                fail_msg: None,
            }),
            &tracker,
        );

        let result = tool
            .execute(serde_json::json!({}), test_tool_context())
            .await;
        assert!(result.is_ok(), "Should succeed");

        // Counter should be reset after success
        assert_eq!(tracker.get("bash"), 0);
    }

    #[tokio::test]
    async fn test_recovery_hint_tool_appends_hint_on_failure() {
        let tracker = ToolFailureTracker::new();

        let tool = with_recovery_hints(
            Box::new(ConfigurableMockTool {
                tool_name: "edit_file",
                fail_msg: Some("old_text not found".to_string()),
            }),
            &tracker,
        );

        let result = tool
            .execute(serde_json::json!({}), test_tool_context())
            .await;
        assert!(result.is_err(), "Should fail");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("old_text not found"),
            "Should contain original error: {err_msg}"
        );
        assert!(
            err_msg.contains("💡 Recovery hint:"),
            "Should contain recovery hint marker: {err_msg}"
        );
        // Attempt 1 for edit_file should suggest using read_file first
        assert!(
            err_msg.contains("read_file"),
            "Attempt 1 hint for edit_file should mention read_file: {err_msg}"
        );
    }

    #[tokio::test]
    async fn test_recovery_hint_tool_escalates_on_repeated_failure() {
        let tracker = ToolFailureTracker::new();

        // First failure
        let tool1 = with_recovery_hints(
            Box::new(ConfigurableMockTool {
                tool_name: "edit_file",
                fail_msg: Some("mismatch".to_string()),
            }),
            &tracker,
        );

        let err1 = tool1
            .execute(serde_json::json!({}), test_tool_context())
            .await
            .unwrap_err()
            .to_string();

        // Second failure — should escalate (attempt >= 2 suggests write_file)
        let tool2 = with_recovery_hints(
            Box::new(ConfigurableMockTool {
                tool_name: "edit_file",
                fail_msg: Some("mismatch again".to_string()),
            }),
            &tracker,
        );

        let err2 = tool2
            .execute(serde_json::json!({}), test_tool_context())
            .await
            .unwrap_err()
            .to_string();

        // Attempt 1 mentions read_file (diagnostic hint)
        assert!(
            err1.contains("read_file"),
            "Attempt 1 should suggest read_file: {err1}"
        );
        // Attempt 2 should mention write_file (escalated alternative)
        assert!(
            err2.contains("write_file"),
            "Attempt 2 should escalate to suggesting write_file: {err2}"
        );
        // The two hints should be different
        assert_ne!(err1, err2, "Hints should escalate between attempts");
    }

    #[tokio::test]
    async fn test_tool_failure_tracker_independent_per_tool() {
        let tracker = ToolFailureTracker::new();

        // Fail bash twice
        let bash_tool = with_recovery_hints(
            Box::new(ConfigurableMockTool {
                tool_name: "bash",
                fail_msg: Some("command not found".to_string()),
            }),
            &tracker,
        );
        let _ = bash_tool
            .execute(serde_json::json!({}), test_tool_context())
            .await;
        let _ = bash_tool
            .execute(serde_json::json!({}), test_tool_context())
            .await;

        assert_eq!(tracker.get("bash"), 2, "bash should have 2 failures");

        // Fail edit_file once
        let edit_tool = with_recovery_hints(
            Box::new(ConfigurableMockTool {
                tool_name: "edit_file",
                fail_msg: Some("not found".to_string()),
            }),
            &tracker,
        );
        let _ = edit_tool
            .execute(serde_json::json!({}), test_tool_context())
            .await;

        assert_eq!(
            tracker.get("edit_file"),
            1,
            "edit_file should have 1 failure"
        );
        assert_eq!(tracker.get("bash"), 2, "bash should still have 2 failures");

        // Succeed on bash — resets only bash
        let bash_ok = with_recovery_hints(
            Box::new(ConfigurableMockTool {
                tool_name: "bash",
                fail_msg: None,
            }),
            &tracker,
        );
        let _ = bash_ok
            .execute(serde_json::json!({}), test_tool_context())
            .await;

        assert_eq!(tracker.get("bash"), 0, "bash should be reset after success");
        assert_eq!(
            tracker.get("edit_file"),
            1,
            "edit_file should be unaffected"
        );
    }
}
