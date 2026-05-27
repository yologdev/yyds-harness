//! SmartEditTool — augments edit_file "not found" errors with line-number context
//!
//! Extracted from `tool_wrappers.rs`. This wrapper intercepts edit_file failures,
//! searches for the nearest fuzzy match, and either auto-fixes whitespace-only
//! mismatches or augments the error with location hints.

use yoagent::types::AgentTool;

/// Maximum file size (bytes) we'll read for nearest-match searching.
const SMART_EDIT_MAX_FILE_SIZE: u64 = 100_000;

/// Number of context lines to show around the nearest match.
const SMART_EDIT_CONTEXT_LINES: usize = 5;

/// A wrapper tool specifically for `edit_file` that intercepts "not found"
/// failures and augments the error message with:
/// - The line number of the nearest match (first-line matching with whitespace normalization)
/// - A snippet of actual content at that location
/// - A hint when the mismatch is purely whitespace/indentation
pub(crate) struct SmartEditTool {
    inner: Box<dyn AgentTool>,
}

/// Search a file's content for the best match of `old_text`, returning
/// `(line_number_1indexed, is_whitespace_only_diff, snippet)`.
fn find_nearest_match(file_content: &str, old_text: &str) -> Option<(usize, bool, String)> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    if old_lines.is_empty() {
        return None;
    }

    // Find the first non-empty line in old_text to use as anchor
    let anchor = old_lines.iter().find(|l| !l.trim().is_empty())?;
    let anchor_trimmed = anchor.trim();

    let file_lines: Vec<&str> = file_content.lines().collect();

    // Search for lines whose trimmed content matches the anchor's trimmed content
    let mut best_match: Option<(usize, usize)> = None; // (line_idx, matching_lines_count)

    for (i, line) in file_lines.iter().enumerate() {
        if line.trim() == anchor_trimmed {
            // Count how many subsequent lines also match (trimmed)
            let mut match_count = 1;
            let anchor_offset = old_lines
                .iter()
                .position(|l| !l.trim().is_empty())
                .unwrap_or(0);

            for j in 1..(old_lines.len() - anchor_offset) {
                let old_idx = anchor_offset + j;
                let file_idx = i + j;
                if file_idx < file_lines.len()
                    && old_idx < old_lines.len()
                    && file_lines[file_idx].trim() == old_lines[old_idx].trim()
                {
                    match_count += 1;
                } else {
                    break;
                }
            }

            if best_match.is_none_or(|(_, prev_count)| match_count > prev_count) {
                // Adjust line number to account for leading empty lines in old_text
                let start_line = if anchor_offset > 0 && i >= anchor_offset {
                    i - anchor_offset
                } else {
                    i
                };
                best_match = Some((start_line, match_count));
            }
        }
    }

    let (match_line_idx, _match_count) = best_match?;

    // Check if the entire old_text matches but only differs in whitespace
    let is_ws_only = {
        let mut all_match_trimmed = true;
        let mut any_exact_mismatch = false;
        for (j, old_line) in old_lines.iter().enumerate() {
            let file_idx = match_line_idx + j;
            if file_idx < file_lines.len() {
                if file_lines[file_idx].trim() == old_line.trim() {
                    if file_lines[file_idx] != *old_line {
                        any_exact_mismatch = true;
                    }
                } else {
                    all_match_trimmed = false;
                    break;
                }
            } else {
                all_match_trimmed = false;
                break;
            }
        }
        all_match_trimmed && any_exact_mismatch
    };

    // Build snippet: up to SMART_EDIT_CONTEXT_LINES lines starting at the match
    let snippet_start = match_line_idx;
    let snippet_end = (match_line_idx + SMART_EDIT_CONTEXT_LINES).min(file_lines.len());
    let snippet: String = file_lines[snippet_start..snippet_end]
        .iter()
        .enumerate()
        .map(|(j, line)| format!("{:>4} │ {}", snippet_start + j + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    Some((match_line_idx + 1, is_ws_only, snippet)) // 1-indexed line number
}

/// Extract exact text from file content starting at `match_line_0indexed` for `line_count` lines.
/// Returns the joined text (with newlines between lines). If the file doesn't have enough lines,
/// returns as many as are available.
fn extract_matched_text(
    file_content: &str,
    match_line_0indexed: usize,
    line_count: usize,
) -> String {
    let file_lines: Vec<&str> = file_content.lines().collect();
    let end = (match_line_0indexed + line_count).min(file_lines.len());
    if match_line_0indexed >= file_lines.len() {
        return String::new();
    }
    file_lines[match_line_0indexed..end].join("\n")
}

/// Wrap an edit_file tool with smart error augmentation.
pub(crate) fn with_smart_edit(tool: Box<dyn AgentTool>) -> Box<dyn AgentTool> {
    Box::new(SmartEditTool { inner: tool })
}

#[async_trait::async_trait]
impl AgentTool for SmartEditTool {
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
        match self.inner.execute(params.clone(), ctx.clone()).await {
            Ok(result) => Ok(result),
            Err(yoagent::types::ToolError::Failed(msg)) if msg.contains("not found") => {
                // Try whitespace auto-fix before falling back to augmented error
                if let Some(retry_result) = self.try_whitespace_autofix(&msg, &params, &ctx).await {
                    return retry_result;
                }
                // No auto-fix possible — augment the error with location info
                let augmented = self.augment_not_found_error(&msg, &params);
                Err(yoagent::types::ToolError::Failed(augmented))
            }
            Err(other) => Err(other),
        }
    }
}

impl SmartEditTool {
    /// Attempt to auto-fix a whitespace-only mismatch by extracting the actual text
    /// from the file and retrying with corrected `old_text`.
    ///
    /// Returns `Some(Ok(..))` if the retry succeeded, `Some(Err(..))` if the retry
    /// also failed (falls through to normal augmented error), or `None` if the mismatch
    /// is not whitespace-only (so caller should use the normal augmentation path).
    async fn try_whitespace_autofix(
        &self,
        _original_msg: &str,
        params: &serde_json::Value,
        ctx: &yoagent::types::ToolContext,
    ) -> Option<Result<yoagent::types::ToolResult, yoagent::types::ToolError>> {
        let path = params.get("path").and_then(|v| v.as_str())?;
        let old_text = params.get("old_text").and_then(|v| v.as_str())?;

        // Check file size
        let metadata = std::fs::metadata(path).ok()?;
        if metadata.len() > SMART_EDIT_MAX_FILE_SIZE {
            return None;
        }

        let content = std::fs::read_to_string(path).ok()?;

        // find_nearest_match returns 1-indexed line number
        let (line_num_1indexed, is_ws_only, _snippet) = find_nearest_match(&content, old_text)?;

        if !is_ws_only {
            return None; // Not a whitespace-only diff — let caller handle it
        }

        // Extract the actual text from the file at the match position
        let old_line_count = old_text.lines().count().max(1);
        let match_line_0indexed = line_num_1indexed - 1;
        let actual_text = extract_matched_text(&content, match_line_0indexed, old_line_count);

        // Build corrected params with the file's actual whitespace
        let mut corrected_params = params.clone();
        corrected_params["old_text"] = serde_json::Value::String(actual_text);

        // Retry with corrected old_text
        match self.inner.execute(corrected_params, ctx.clone()).await {
            Ok(mut result) => {
                // Append auto-fix note to the result
                let note = format!(
                    "\n⚡ Auto-fixed whitespace mismatch at line {}",
                    line_num_1indexed
                );
                result.content.push(yoagent::Content::Text { text: note });
                Some(Ok(result))
            }
            Err(_) => {
                // Retry also failed — return None to fall through to augmented error
                None
            }
        }
    }

    fn augment_not_found_error(&self, original_msg: &str, params: &serde_json::Value) -> String {
        // Extract path and old_text from params
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return original_msg.to_string(),
        };
        let old_text = match params.get("old_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return original_msg.to_string(),
        };

        // Check file size — skip for huge files
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return original_msg.to_string(),
        };
        if metadata.len() > SMART_EDIT_MAX_FILE_SIZE {
            return original_msg.to_string();
        }

        // Read the file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return original_msg.to_string(),
        };

        // Search for nearest match
        match find_nearest_match(&content, old_text) {
            Some((line_num, is_ws_only, snippet)) => {
                let mut augmented = original_msg.to_string();
                augmented.push_str(&format!(
                    "\n\n📍 Nearest match at line {}:\n```\n{}\n```",
                    line_num, snippet
                ));
                if is_ws_only {
                    augmented.push_str(
                        "\n\n⚠️ Hint: the text exists but indentation/whitespace differs. \
                         Use read_file to see the exact whitespace.",
                    );
                }
                augmented
            }
            None => original_msg.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tool_context() -> yoagent::types::ToolContext {
        yoagent::types::ToolContext {
            tool_call_id: "test".to_string(),
            tool_name: "test".to_string(),
            cancel: tokio_util::sync::CancellationToken::new(),
            on_update: None,
            on_progress: None,
        }
    }

    #[test]
    fn test_find_nearest_match_exact_line() {
        let file_content = "line one\nline two\nfn hello() {\n    world()\n}\nline six\n";
        let old_text = "fn hello() {\n    world()\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some(), "Should find a match");
        let (line, is_ws, snippet) = result.unwrap();
        assert_eq!(line, 3, "Match should be at line 3");
        assert!(!is_ws, "Should not be whitespace-only diff");
        assert!(
            snippet.contains("fn hello()"),
            "Snippet should contain the match"
        );
    }

    #[test]
    fn test_find_nearest_match_whitespace_only_diff() {
        // File has 4-space indent, old_text has 2-space indent
        let file_content = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";
        let old_text = "fn main() {\n  let x = 1;\n  let y = 2;\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some(), "Should find a match");
        let (line, is_ws, _snippet) = result.unwrap();
        assert_eq!(line, 1, "Match should be at line 1");
        assert!(is_ws, "Should detect whitespace-only diff");
    }

    #[test]
    fn test_find_nearest_match_no_match() {
        let file_content = "fn main() {\n    println!(\"hello\");\n}\n";
        let old_text = "fn totally_different() {\n    nothing();\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_none(), "Should not find a match");
    }

    #[test]
    fn test_find_nearest_match_empty_old_text() {
        let file_content = "fn main() {}\n";
        let result = find_nearest_match(file_content, "");
        assert!(result.is_none(), "Empty old_text should return None");
    }

    #[test]
    fn test_find_nearest_match_only_whitespace_lines() {
        let file_content = "fn main() {}\n";
        let result = find_nearest_match(file_content, "   \n   \n");
        assert!(
            result.is_none(),
            "All-whitespace old_text should return None"
        );
    }

    #[test]
    fn test_find_nearest_match_snippet_limited_to_5_lines() {
        let file_content = (1..=20)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let old_text = "line 5";
        let result = find_nearest_match(&file_content, old_text);
        assert!(result.is_some());
        let (line, _, snippet) = result.unwrap();
        assert_eq!(line, 5);
        // Should show exactly 5 lines of context
        let snippet_lines: Vec<&str> = snippet.lines().collect();
        assert_eq!(
            snippet_lines.len(),
            5,
            "Snippet should be 5 lines: {:?}",
            snippet_lines
        );
    }

    /// A mock tool for SmartEditTool tests — returns a configurable error or success.
    struct SmartEditMockTool {
        fail_msg: Option<String>,
        result_text: Option<String>,
    }

    #[async_trait::async_trait]
    impl AgentTool for SmartEditMockTool {
        fn name(&self) -> &str {
            "edit_file"
        }
        fn label(&self) -> &str {
            "edit_file"
        }
        fn description(&self) -> &str {
            "mock edit_file"
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
                        text: self.result_text.clone().unwrap_or_else(|| "ok".into()),
                    }],
                    details: serde_json::Value::Null,
                })
            }
        }
    }

    #[tokio::test]
    async fn test_smart_edit_passes_through_success() {
        let tool = with_smart_edit(Box::new(SmartEditMockTool {
            fail_msg: None,
            result_text: Some("edited successfully".into()),
        }));

        let params = serde_json::json!({
            "path": "src/main.rs",
            "old_text": "fn main()",
            "new_text": "fn main2()"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(result.is_ok(), "Success should pass through");
    }

    #[tokio::test]
    async fn test_smart_edit_passes_through_non_not_found_error() {
        let tool = with_smart_edit(Box::new(SmartEditMockTool {
            fail_msg: Some("permission denied".into()),
            result_text: None,
        }));

        let params = serde_json::json!({
            "path": "src/main.rs",
            "old_text": "fn main()",
            "new_text": "fn main2()"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert_eq!(
            err, "permission denied",
            "Non-'not found' errors pass through unchanged"
        );
    }

    #[tokio::test]
    async fn test_smart_edit_augments_not_found_with_line_number() {
        // Create a temp file with known content
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        std::fs::write(
            &file_path,
            "line one\nline two\nfn hello() {\n    world()\n}\nline six\n",
        )
        .unwrap();

        let tool = with_smart_edit(Box::new(SmartEditMockTool {
            fail_msg: Some("old_text not found in file".into()),
            result_text: None,
        }));

        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "fn hello() {\n  world()\n}",
            "new_text": "fn goodbye()"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("line 3"), "Should mention line number: {err}");
        assert!(
            err.contains("fn hello()"),
            "Should show snippet with actual content: {err}"
        );
        assert!(
            err.contains("📍 Nearest match"),
            "Should have nearest match marker: {err}"
        );
    }

    #[tokio::test]
    async fn test_smart_edit_detects_whitespace_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("ws.rs");
        std::fs::write(
            &file_path,
            "fn main() {\n    let x = 1;\n    let y = 2;\n}\n",
        )
        .unwrap();

        let tool = with_smart_edit(Box::new(SmartEditMockTool {
            fail_msg: Some("old_text not found in file".into()),
            result_text: None,
        }));

        // old_text with 2-space indent instead of 4-space
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "fn main() {\n  let x = 1;\n  let y = 2;\n}",
            "new_text": "fn main() {\n  let x = 42;\n}"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("indentation") || err.contains("whitespace"),
            "Should hint about whitespace difference: {err}"
        );
        assert!(err.contains("line 1"), "Should report line number: {err}");
    }

    #[tokio::test]
    async fn test_smart_edit_handles_missing_file_gracefully() {
        let tool = with_smart_edit(Box::new(SmartEditMockTool {
            fail_msg: Some("old_text not found in file".into()),
            result_text: None,
        }));

        let params = serde_json::json!({
            "path": "/nonexistent/file.rs",
            "old_text": "fn hello()",
            "new_text": "fn goodbye()"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // Should gracefully fall back to original message without panic
        assert!(
            err.contains("old_text not found"),
            "Should contain original error: {err}"
        );
    }

    // === extract_matched_text tests ===

    #[test]
    fn test_extract_matched_text_basic() {
        let content = "line 0\nline 1\nline 2\nline 3\nline 4\n";
        let result = extract_matched_text(content, 1, 2);
        assert_eq!(result, "line 1\nline 2");
    }

    #[test]
    fn test_extract_matched_text_from_start() {
        let content = "fn main() {\n    hello();\n}\n";
        let result = extract_matched_text(content, 0, 3);
        assert_eq!(result, "fn main() {\n    hello();\n}");
    }

    #[test]
    fn test_extract_matched_text_beyond_end() {
        let content = "line 0\nline 1\n";
        // Request more lines than available
        let result = extract_matched_text(content, 1, 5);
        assert_eq!(result, "line 1");
    }

    #[test]
    fn test_extract_matched_text_out_of_bounds() {
        let content = "line 0\n";
        let result = extract_matched_text(content, 10, 2);
        assert_eq!(result, "");
    }

    // === SmartEditTool whitespace auto-fix tests ===

    /// A stateful mock tool that fails on first call and succeeds on second.
    /// Used to simulate auto-fix retry behavior.
    struct SmartEditRetryMockTool {
        call_count: std::sync::atomic::AtomicUsize,
        /// If set, first call fails with this message.
        first_fail_msg: String,
    }

    #[async_trait::async_trait]
    impl AgentTool for SmartEditRetryMockTool {
        fn name(&self) -> &str {
            "edit_file"
        }
        fn label(&self) -> &str {
            "edit_file"
        }
        fn description(&self) -> &str {
            "mock edit_file (retry-aware)"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: yoagent::types::ToolContext,
        ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
            let call = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if call == 0 {
                Err(yoagent::types::ToolError::Failed(
                    self.first_fail_msg.clone(),
                ))
            } else {
                Ok(yoagent::types::ToolResult {
                    content: vec![yoagent::Content::Text {
                        text: "edit applied".into(),
                    }],
                    details: serde_json::Value::Null,
                })
            }
        }
    }

    /// A stateful mock that always fails (used to test retry-failure fallback).
    struct SmartEditAlwaysFailMockTool {
        fail_msg: String,
    }

    #[async_trait::async_trait]
    impl AgentTool for SmartEditAlwaysFailMockTool {
        fn name(&self) -> &str {
            "edit_file"
        }
        fn label(&self) -> &str {
            "edit_file"
        }
        fn description(&self) -> &str {
            "mock edit_file (always fails)"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({})
        }
        async fn execute(
            &self,
            _params: serde_json::Value,
            _ctx: yoagent::types::ToolContext,
        ) -> Result<yoagent::types::ToolResult, yoagent::types::ToolError> {
            Err(yoagent::types::ToolError::Failed(self.fail_msg.clone()))
        }
    }

    #[tokio::test]
    async fn test_smart_edit_autofix_whitespace_mismatch() {
        // Create a temp file with 4-space indentation
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("ws_fix.rs");
        std::fs::write(
            &file_path,
            "fn main() {\n    let x = 1;\n    let y = 2;\n}\n",
        )
        .unwrap();

        // The mock fails on first call (wrong whitespace), succeeds on retry
        let tool = with_smart_edit(Box::new(SmartEditRetryMockTool {
            call_count: std::sync::atomic::AtomicUsize::new(0),
            first_fail_msg: "old_text not found in file".into(),
        }));

        // old_text with 2-space indent (wrong), new_text is the intended replacement
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "fn main() {\n  let x = 1;\n  let y = 2;\n}",
            "new_text": "fn main() {\n    let x = 42;\n}"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(result.is_ok(), "Auto-fix should succeed: {:?}", result);
        let result = result.unwrap();
        // Check that the auto-fix note is appended
        let texts: Vec<String> = result
            .content
            .iter()
            .filter_map(|c| match c {
                yoagent::Content::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect();
        let combined = texts.join(" ");
        assert!(
            combined.contains("Auto-fixed whitespace mismatch"),
            "Should contain auto-fix note: {combined}"
        );
        assert!(
            combined.contains("line 1"),
            "Should mention the line number: {combined}"
        );
    }

    #[tokio::test]
    async fn test_smart_edit_no_autofix_for_non_whitespace_mismatch() {
        // Create a temp file
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("no_fix.rs");
        std::fs::write(&file_path, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let tool = with_smart_edit(Box::new(SmartEditMockTool {
            fail_msg: Some("old_text not found in file".into()),
            result_text: None,
        }));

        // old_text differs in content, not just whitespace
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "fn main() {\n    println!(\"world\");\n}",
            "new_text": "fn main() {\n    println!(\"goodbye\");\n}"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(result.is_err(), "Non-whitespace mismatch should still fail");
        let err = result.unwrap_err().to_string();
        // Should have the augmented error with nearest match, NOT auto-fix
        assert!(
            err.contains("not found"),
            "Should contain original error: {err}"
        );
        assert!(
            !err.contains("Auto-fixed"),
            "Should NOT contain auto-fix note: {err}"
        );
    }

    #[tokio::test]
    async fn test_smart_edit_autofix_retry_failure_falls_through() {
        // Create a temp file with 4-space indentation
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("fail_retry.rs");
        std::fs::write(
            &file_path,
            "fn main() {\n    let x = 1;\n    let y = 2;\n}\n",
        )
        .unwrap();

        // The mock always fails — even the retry
        let tool = with_smart_edit(Box::new(SmartEditAlwaysFailMockTool {
            fail_msg: "old_text not found in file".into(),
        }));

        // old_text with whitespace mismatch (will trigger auto-fix attempt, but retry also fails)
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "fn main() {\n  let x = 1;\n  let y = 2;\n}",
            "new_text": "fn main() {\n    let x = 42;\n}"
        });

        let result = tool.execute(params, test_tool_context()).await;
        assert!(
            result.is_err(),
            "Should fall through to augmented error when retry fails"
        );
        let err = result.unwrap_err().to_string();
        // Should have the augmented error (from augment_not_found_error), including the hint
        assert!(
            err.contains("Nearest match"),
            "Should contain nearest match info: {err}"
        );
        assert!(
            err.contains("whitespace"),
            "Should contain whitespace hint: {err}"
        );
    }

    // === find_nearest_match edge case tests ===

    #[test]
    fn test_find_nearest_match_extra_blank_lines_in_old_text() {
        // File has no blank lines between statements; old_text has an extra blank line
        let file_content = "fn foo() {\n    let a = 1;\n    let b = 2;\n}\n";
        let old_text = "fn foo() {\n    let a = 1;\n\n    let b = 2;\n}";
        let result = find_nearest_match(file_content, old_text);
        // Should still find a match anchored on "fn foo()" even with the blank line mismatch
        assert!(
            result.is_some(),
            "Should find a match despite extra blank line"
        );
        let (line, _is_ws, _snippet) = result.unwrap();
        assert_eq!(line, 1, "Match should be at line 1");
    }

    #[test]
    fn test_find_nearest_match_fewer_blank_lines_in_old_text() {
        // File has a blank line; old_text omits it
        let file_content = "fn bar() {\n    let a = 1;\n\n    let b = 2;\n}\n";
        let old_text = "fn bar() {\n    let a = 1;\n    let b = 2;\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(
            result.is_some(),
            "Should find a match despite fewer blank lines"
        );
        let (line, _is_ws, _snippet) = result.unwrap();
        assert_eq!(line, 1, "Match should be at line 1");
    }

    #[test]
    fn test_find_nearest_match_at_start_of_file() {
        // Match is at the very first line
        let file_content = "fn first() {\n    body();\n}\nfn second() {}\n";
        let old_text = "fn first() {\n    body();\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some(), "Should find match at start");
        let (line, is_ws, _snippet) = result.unwrap();
        assert_eq!(line, 1, "Match should be at line 1 (very start)");
        assert!(!is_ws, "Should be an exact match, not whitespace-only");
    }

    #[test]
    fn test_find_nearest_match_at_end_of_file_no_trailing_newline() {
        // Match at the very end, file has no trailing newline
        let file_content = "fn first() {}\nfn last() {\n    done();\n}";
        let old_text = "fn last() {\n    done();\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some(), "Should find match at end of file");
        let (line, is_ws, _snippet) = result.unwrap();
        assert_eq!(line, 2, "Match should be at line 2");
        assert!(!is_ws, "Should be exact match");
    }

    #[test]
    fn test_find_nearest_match_at_end_snippet_truncated() {
        // When match is near the end, snippet should not go past file end
        let file_content = "a\nb\nc\nlast_line";
        let old_text = "last_line";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some());
        let (line, _, snippet) = result.unwrap();
        assert_eq!(line, 4, "Match at line 4 (last line)");
        // Snippet should only have 1 line since there's nothing after
        let snippet_lines: Vec<&str> = snippet.lines().collect();
        assert_eq!(snippet_lines.len(), 1, "Snippet limited to remaining lines");
        assert!(snippet.contains("last_line"));
    }

    #[test]
    fn test_find_nearest_match_very_short_old_text_single_char() {
        // A single character should still match if it exists as a whole line
        let file_content = "a\nb\nc\n";
        let old_text = "b";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some(), "Single char should match a whole line");
        let (line, is_ws, _) = result.unwrap();
        assert_eq!(line, 2);
        assert!(!is_ws);
    }

    #[test]
    fn test_find_nearest_match_very_short_old_text_no_line_match() {
        // Short old_text that doesn't match any whole line (only substring)
        let file_content = "hello world\nfoo bar\nbaz qux\n";
        let old_text = "oo"; // substring of "foo" but not a whole line
        let result = find_nearest_match(file_content, old_text);
        assert!(
            result.is_none(),
            "Partial substring should not match (trimmed comparison is exact)"
        );
    }

    #[test]
    fn test_find_nearest_match_tabs_vs_spaces() {
        // File uses tabs, old_text uses spaces
        let file_content = "fn main() {\n\tlet x = 1;\n\tlet y = 2;\n}\n";
        let old_text = "fn main() {\n    let x = 1;\n    let y = 2;\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(
            result.is_some(),
            "Should find match with tab/space mismatch"
        );
        let (line, is_ws, _snippet) = result.unwrap();
        assert_eq!(line, 1);
        assert!(is_ws, "Tab vs space difference should be whitespace-only");
    }

    #[test]
    fn test_find_nearest_match_spaces_vs_tabs() {
        // Reverse: file uses spaces, old_text uses tabs
        let file_content = "fn main() {\n    let x = 1;\n}\n";
        let old_text = "fn main() {\n\tlet x = 1;\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(
            result.is_some(),
            "Should find match with space/tab mismatch"
        );
        let (_, is_ws, _) = result.unwrap();
        assert!(is_ws, "Space vs tab difference should be whitespace-only");
    }

    #[test]
    fn test_find_nearest_match_multiple_partial_matches_picks_best() {
        // Two functions with the same opening but different bodies.
        // The old_text matches the second one better (more lines match).
        let file_content = "fn do_thing() {\n    alpha();\n}\n\nfn do_thing() {\n    alpha();\n    beta();\n    gamma();\n}\n";
        let old_text = "fn do_thing() {\n    alpha();\n    beta();\n    gamma();\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some(), "Should find the best match");
        let (line, is_ws, _) = result.unwrap();
        // The second fn do_thing() starts at line 5 and matches 5 lines
        assert_eq!(line, 5, "Should pick the better (longer) match at line 5");
        assert!(!is_ws);
    }

    #[test]
    fn test_find_nearest_match_multiple_matches_first_if_equal() {
        // Two identical matches — should pick the one with the higher match count
        // (in practice, if equal, the later one wins because of > comparison)
        let file_content = "let x = 1;\nlet y = 2;\nlet x = 1;\nlet y = 2;\n";
        let old_text = "let x = 1;\nlet y = 2;";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some());
        let (line, _, _) = result.unwrap();
        // Both matches have count=2, so the second one wins (> not >=)
        // Actually let's check: is_none_or with match_count > prev_count means
        // equal count does NOT replace, so first match wins
        assert_eq!(line, 1, "Equal matches: first one wins");
    }

    #[test]
    fn test_find_nearest_match_unicode_content() {
        let file_content =
            "fn greet() {\n    println!(\"こんにちは\");\n    println!(\"世界\");\n}\n";
        let old_text = "fn greet() {\n    println!(\"こんにちは\");\n    println!(\"世界\");\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some(), "Should match Unicode content");
        let (line, is_ws, snippet) = result.unwrap();
        assert_eq!(line, 1);
        assert!(!is_ws);
        assert!(
            snippet.contains("こんにちは"),
            "Snippet should contain Unicode"
        );
    }

    #[test]
    fn test_find_nearest_match_unicode_with_whitespace_diff() {
        // Unicode content with indentation mismatch
        let file_content = "fn emoji() {\n    let msg = \"🎉✓\";\n}\n";
        let old_text = "fn emoji() {\n  let msg = \"🎉✓\";\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some());
        let (_, is_ws, _) = result.unwrap();
        assert!(
            is_ws,
            "Unicode content with only whitespace diff should be detected"
        );
    }

    #[test]
    fn test_find_nearest_match_empty_file() {
        let result = find_nearest_match("", "fn hello()");
        assert!(result.is_none(), "Empty file should return None");
    }

    #[test]
    fn test_find_nearest_match_empty_file_empty_old_text() {
        let result = find_nearest_match("", "");
        assert!(result.is_none(), "Both empty should return None");
    }

    #[test]
    fn test_find_nearest_match_old_text_with_leading_empty_lines() {
        // old_text starts with empty lines, the anchor is on a later line
        let file_content = "fn alpha() {}\n\nfn beta() {\n    body();\n}\n";
        let old_text = "\n\nfn beta() {\n    body();\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(
            result.is_some(),
            "Should find match even with leading empty lines in old_text"
        );
        let (line, _is_ws, _snippet) = result.unwrap();
        // The anchor "fn beta() {" is at file line 3 (1-indexed),
        // and anchor_offset is 2 (two leading empty lines), so start_line adjusts back
        assert!(line <= 3, "Line should account for leading empty lines");
    }

    #[test]
    fn test_find_nearest_match_single_line_file() {
        let file_content = "only_line";
        let old_text = "only_line";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some());
        let (line, is_ws, _) = result.unwrap();
        assert_eq!(line, 1);
        assert!(!is_ws);
    }

    #[test]
    fn test_find_nearest_match_trailing_whitespace_diff() {
        // File lines have trailing spaces, old_text doesn't
        let file_content = "fn main() {  \n    let x = 1;  \n}\n";
        let old_text = "fn main() {\n    let x = 1;\n}";
        let result = find_nearest_match(file_content, old_text);
        assert!(result.is_some());
        let (line, is_ws, _) = result.unwrap();
        assert_eq!(line, 1);
        assert!(
            is_ws,
            "Trailing whitespace difference should be whitespace-only"
        );
    }

    // === augment_not_found_error edge case tests ===

    #[test]
    fn test_augment_not_found_error_missing_path() {
        let tool = SmartEditTool {
            inner: Box::new(SmartEditMockTool {
                fail_msg: None,
                result_text: None,
            }),
        };
        let params = serde_json::json!({
            "old_text": "fn hello()",
            "new_text": "fn goodbye()"
        });
        let result = tool.augment_not_found_error("old_text not found in file", &params);
        assert_eq!(
            result, "old_text not found in file",
            "Missing path should return original msg"
        );
    }

    #[test]
    fn test_augment_not_found_error_missing_old_text() {
        let tool = SmartEditTool {
            inner: Box::new(SmartEditMockTool {
                fail_msg: None,
                result_text: None,
            }),
        };
        let params = serde_json::json!({
            "path": "/some/file.rs",
            "new_text": "fn goodbye()"
        });
        let result = tool.augment_not_found_error("old_text not found in file", &params);
        assert_eq!(
            result, "old_text not found in file",
            "Missing old_text should return original msg"
        );
    }

    #[test]
    fn test_augment_not_found_error_nonexistent_file() {
        let tool = SmartEditTool {
            inner: Box::new(SmartEditMockTool {
                fail_msg: None,
                result_text: None,
            }),
        };
        let params = serde_json::json!({
            "path": "/definitely/does/not/exist/file.rs",
            "old_text": "fn hello()",
            "new_text": "fn goodbye()"
        });
        let result = tool.augment_not_found_error("old_text not found in file", &params);
        assert_eq!(
            result, "old_text not found in file",
            "Nonexistent file should return original msg"
        );
    }

    #[test]
    fn test_augment_not_found_error_line_number_accuracy() {
        // Create a temp file with known content, verify exact line number in output
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("lines.rs");
        std::fs::write(
            &file_path,
            "line 1\nline 2\nline 3\nfn target() {\n    body();\n}\nline 7\n",
        )
        .unwrap();

        let tool = SmartEditTool {
            inner: Box::new(SmartEditMockTool {
                fail_msg: None,
                result_text: None,
            }),
        };
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "fn target() {\n  body();\n}",
            "new_text": "fn replacement()"
        });
        let result = tool.augment_not_found_error("old_text not found", &params);
        assert!(
            result.contains("line 4"),
            "Should report line 4 for fn target(): {result}"
        );
        assert!(
            result.contains("fn target()"),
            "Should show the actual content"
        );
        assert!(
            result.contains("whitespace"),
            "Should hint about whitespace diff"
        );
    }

    #[test]
    fn test_augment_not_found_error_no_match_in_file() {
        // File exists but old_text has no match at all
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("nomatch.rs");
        std::fs::write(&file_path, "fn alpha() {}\nfn beta() {}\n").unwrap();

        let tool = SmartEditTool {
            inner: Box::new(SmartEditMockTool {
                fail_msg: None,
                result_text: None,
            }),
        };
        let params = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "fn completely_unrelated()",
            "new_text": "fn replacement()"
        });
        let result = tool.augment_not_found_error("old_text not found", &params);
        // No match found, should return original message unaugmented
        assert_eq!(
            result, "old_text not found",
            "No match should return original: {result}"
        );
    }

    // === extract_matched_text additional tests ===

    #[test]
    fn test_extract_matched_text_empty_content() {
        let result = extract_matched_text("", 0, 3);
        assert_eq!(result, "", "Empty content should return empty string");
    }

    #[test]
    fn test_extract_matched_text_single_line() {
        let result = extract_matched_text("only_line", 0, 1);
        assert_eq!(result, "only_line");
    }

    #[test]
    fn test_extract_matched_text_exact_range() {
        let content = "a\nb\nc\nd\n";
        let result = extract_matched_text(content, 0, 4);
        assert_eq!(result, "a\nb\nc\nd");
    }
}
