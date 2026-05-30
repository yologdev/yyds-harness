//! Message search, highlighting, summarization, and output utilities.
//!
//! Extracted from `prompt.rs` (Day 64) — these are utility functions
//! used by `/search`, `/history`, and output file writing, but unrelated
//! to the core prompt execution loop.

use crate::format::*;
use yoagent::*;

/// Extract a preview of tool result content for display.
/// Returns an empty string if there's nothing meaningful to show.
pub(crate) fn tool_result_preview(result: &ToolResult, max_chars: usize) -> String {
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

/// File-tool names whose `path` argument references a file.
const FILE_TOOLS: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "list_files",
    "search",
];

/// Max number of file paths to include in the context summary.
const MAX_FILES: usize = 10;
/// Max number of topic strings to include in the context summary.
const MAX_TOPICS: usize = 5;

/// Summarize what topics and files are present in a set of messages.
///
/// Scans tool calls for file paths and user messages for topic keywords.
/// Returns a vec of short strings like `["src/main.rs", "auth refactor"]`.
pub fn summarize_context_topics(messages: &[AgentMessage]) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    let mut topics: Vec<String> = Vec::new();
    let mut user_count = 0;

    for msg in messages {
        match msg {
            // Extract file paths from tool calls in assistant messages
            AgentMessage::Llm(Message::Assistant { content, .. }) => {
                for c in content {
                    if let Content::ToolCall {
                        name, arguments, ..
                    } = c
                    {
                        if FILE_TOOLS.contains(&name.as_str()) {
                            if let Some(path) = arguments.get("path").and_then(|v| v.as_str()) {
                                if !path.is_empty()
                                    && files.len() < MAX_FILES
                                    && !files.iter().any(|f| f == path)
                                {
                                    files.push(path.to_string());
                                }
                            }
                        }
                    }
                }
            }
            // Extract topics from user messages (first few meaningful messages)
            AgentMessage::Llm(Message::User { content, .. }) if user_count < MAX_TOPICS => {
                let text: String = content
                    .iter()
                    .filter_map(|c| match c {
                        Content::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let text = text.trim();
                // Skip empty, slash-commands, and very short messages
                if !text.is_empty() && !text.starts_with('/') && text.len() > 3 {
                    let topic = extract_topic_phrase(text);
                    if !topic.is_empty() && !topics.contains(&topic) && !files.contains(&topic) {
                        topics.push(topic);
                    }
                    user_count += 1;
                }
            }
            _ => {}
        }
    }

    let mut result: Vec<String> = files.into_iter().collect();
    result.extend(topics);
    result
}

/// Extract a short topic phrase from user text.
/// Takes the first meaningful segment (up to ~50 chars), stopping at sentence/clause boundaries.
fn extract_topic_phrase(text: &str) -> String {
    // Take first line only
    let first_line = text.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return String::new();
    }
    // Truncate at a reasonable length, preferring to break at word boundaries
    let max_len = 50;
    if first_line.len() <= max_len {
        return first_line.to_string();
    }
    // Find a safe char boundary and then a word boundary
    let mut end = max_len;
    while end > 0 && !first_line.is_char_boundary(end) {
        end -= 1;
    }
    // Try to break at the last space before the limit
    if let Some(space_pos) = first_line[..end].rfind(' ') {
        if space_pos > 10 {
            end = space_pos;
        }
    }
    format!("{}…", &first_line[..end])
}

/// Format the context summary for display after compaction.
/// Returns `None` if there's nothing to summarize.
pub fn format_context_summary(topics: &[String]) -> Option<String> {
    if topics.is_empty() {
        return None;
    }

    let file_count = topics.iter().filter(|t| looks_like_path(t)).count();
    let topic_count = topics.len() - file_count;

    let items = topics.join(", ");

    let mut suffix_parts = Vec::new();
    if file_count > 0 {
        suffix_parts.push(format!(
            "{file_count} file{}",
            if file_count == 1 { "" } else { "s" }
        ));
    }
    if topic_count > 0 {
        suffix_parts.push(format!(
            "{topic_count} topic{}",
            if topic_count == 1 { "" } else { "s" }
        ));
    }

    let suffix = if suffix_parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", suffix_parts.join(", "))
    };

    Some(format!("📋 Still in context: {items}{suffix}"))
}

/// Heuristic: does this string look like a file path?
fn looks_like_path(s: &str) -> bool {
    s.contains('/') || (s.contains('.') && !s.contains(' '))
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let tmp_dir = tempfile::Builder::new()
            .prefix("yoyo_test_output")
            .tempdir()
            .unwrap();
        let path = tmp_dir.path().join("test_output.txt");
        let path_str = path.to_string_lossy().to_string();
        write_output_file(&Some(path_str), "hello from yoyo");
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "hello from yoyo");
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

    // --- summarize_context_topics tests ---

    /// Helper: build an assistant message with a single tool call.
    fn assistant_with_tool(name: &str, path: &str) -> AgentMessage {
        AgentMessage::Llm(Message::Assistant {
            content: vec![Content::ToolCall {
                id: "tc_1".into(),
                name: name.into(),
                arguments: serde_json::json!({ "path": path }),
                provider_metadata: None,
            }],
            stop_reason: StopReason::ToolUse,
            model: "test".into(),
            provider: "test".into(),
            usage: Usage::default(),
            timestamp: 0,
            error_message: None,
        })
    }

    #[test]
    fn test_summarize_context_topics_extracts_file_paths() {
        let messages = vec![
            assistant_with_tool("read_file", "src/main.rs"),
            assistant_with_tool("edit_file", "src/tools.rs"),
            assistant_with_tool("write_file", "src/new.rs"),
        ];
        let topics = summarize_context_topics(&messages);
        assert!(topics.contains(&"src/main.rs".to_string()));
        assert!(topics.contains(&"src/tools.rs".to_string()));
        assert!(topics.contains(&"src/new.rs".to_string()));
        assert_eq!(topics.len(), 3);
    }

    #[test]
    fn test_summarize_context_topics_deduplicates_paths() {
        let messages = vec![
            assistant_with_tool("read_file", "src/main.rs"),
            assistant_with_tool("edit_file", "src/main.rs"),
            assistant_with_tool("read_file", "src/main.rs"),
        ];
        let topics = summarize_context_topics(&messages);
        assert_eq!(
            topics.iter().filter(|t| *t == "src/main.rs").count(),
            1,
            "should deduplicate file paths"
        );
    }

    #[test]
    fn test_summarize_context_topics_empty_messages() {
        let topics = summarize_context_topics(&[]);
        assert!(topics.is_empty());
    }

    #[test]
    fn test_summarize_context_topics_ignores_non_file_tools() {
        let messages = vec![assistant_with_tool("bash", "/usr/bin/ls")];
        let topics = summarize_context_topics(&messages);
        // bash is not a file tool, so no paths extracted
        assert!(
            !topics.contains(&"/usr/bin/ls".to_string()),
            "should not extract paths from non-file tools"
        );
    }

    #[test]
    fn test_summarize_context_topics_extracts_user_topics() {
        let messages = vec![
            AgentMessage::Llm(Message::user(
                "refactor the authentication module to use JWT tokens",
            )),
            assistant_with_tool("read_file", "src/auth.rs"),
        ];
        let topics = summarize_context_topics(&messages);
        assert!(topics.contains(&"src/auth.rs".to_string()));
        // Should have a topic from the user message
        assert!(topics.len() >= 2, "should include user topic");
        assert!(
            topics
                .iter()
                .any(|t| t.contains("refactor") || t.contains("authentication")),
            "should extract topic from user message"
        );
    }

    #[test]
    fn test_summarize_context_topics_caps_files() {
        // Create 15 unique file paths — should cap at MAX_FILES (10)
        let messages: Vec<AgentMessage> = (0..15)
            .map(|i| assistant_with_tool("read_file", &format!("src/file_{i}.rs")))
            .collect();
        let topics = summarize_context_topics(&messages);
        let file_count = topics.iter().filter(|t| looks_like_path(t)).count();
        assert!(file_count <= 10, "should cap at MAX_FILES");
    }

    #[test]
    fn test_summarize_context_topics_skips_slash_commands() {
        let messages = vec![AgentMessage::Llm(Message::user("/compact"))];
        let topics = summarize_context_topics(&messages);
        assert!(
            topics.is_empty(),
            "should skip slash commands as user topics"
        );
    }

    #[test]
    fn test_format_context_summary_empty() {
        assert!(format_context_summary(&[]).is_none());
    }

    #[test]
    fn test_format_context_summary_files_only() {
        let topics = vec!["src/main.rs".to_string(), "src/tools.rs".to_string()];
        let summary = format_context_summary(&topics).unwrap();
        assert!(summary.contains("src/main.rs"));
        assert!(summary.contains("src/tools.rs"));
        assert!(summary.contains("2 files"));
    }

    #[test]
    fn test_format_context_summary_mixed() {
        let topics = vec![
            "src/main.rs".to_string(),
            "refactor auth module".to_string(),
        ];
        let summary = format_context_summary(&topics).unwrap();
        assert!(summary.contains("1 file"));
        assert!(summary.contains("1 topic"));
        assert!(summary.contains("📋"));
    }

    #[test]
    fn test_extract_topic_phrase_short() {
        assert_eq!(extract_topic_phrase("fix the bug"), "fix the bug");
    }

    #[test]
    fn test_extract_topic_phrase_long() {
        let long_text = "this is a very long message that should be truncated at a reasonable point so it doesn't overflow the display area in the terminal";
        let result = extract_topic_phrase(long_text);
        assert!(result.len() <= 55); // 50 + a few for the ellipsis
        assert!(result.ends_with('…'));
    }

    #[test]
    fn test_extract_topic_phrase_empty() {
        assert_eq!(extract_topic_phrase(""), "");
    }
}
