//! Web content fetching (/web) and clipboard integration (/copy).

use crate::format::*;

/// Maximum characters to display from a fetched web page.
pub(crate) const WEB_MAX_CHARS: usize = 5000;

/// Case-insensitive search for an ASCII-only pattern in a UTF-8 string.
///
/// Returns the byte offset in `haystack` where `needle` starts.
/// `needle` must be ASCII lowercase.
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    let needle_bytes = needle.as_bytes();
    let hay_bytes = haystack.as_bytes();
    if needle_bytes.is_empty() || needle_bytes.len() > hay_bytes.len() {
        return None;
    }
    'outer: for start in 0..=(hay_bytes.len() - needle_bytes.len()) {
        for (k, &nb) in needle_bytes.iter().enumerate() {
            if hay_bytes[start + k].to_ascii_lowercase() != nb {
                continue 'outer;
            }
        }
        return Some(start);
    }
    None
}

/// Check if `haystack` starts with ASCII lowercase `needle` (case-insensitive).
fn starts_with_ascii_ci(haystack: &str, needle: &str) -> bool {
    let hay_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();
    if hay_bytes.len() < needle_bytes.len() {
        return false;
    }
    for (k, &nb) in needle_bytes.iter().enumerate() {
        if hay_bytes[k].to_ascii_lowercase() != nb {
            return false;
        }
    }
    true
}

/// Strip HTML tags and extract readable text content.
///
/// This function:
/// - Removes `<script>`, `<style>`, `<nav>`, `<footer>`, `<header>`, `<svg>` blocks entirely
/// - Converts `<br>`, `<p>`, `<div>`, `<li>`, `<h1>`–`<h6>`, `<tr>` to newlines
/// - Converts `<li>` items to bullet points
/// - Strips all remaining HTML tags
/// - Decodes common HTML entities
/// - Collapses excessive whitespace
/// - Truncates to `max_chars`
pub fn strip_html_tags(html: &str, max_chars: usize) -> String {
    // First pass: remove blocks we want to skip entirely (script, style, etc.)
    // Uses find_ascii_ci for case-insensitive tag matching without pre-lowering
    // the entire string (which would break byte-position correspondence for
    // non-ASCII chars whose lowercase has a different byte length).
    let mut cleaned = String::with_capacity(html.len());
    let skip_tags = ["script", "style", "nav", "footer", "header", "svg"];

    let mut i = 0;
    let bytes = html.as_bytes();

    while i < bytes.len() {
        // '<' is ASCII (0x3C) — never appears as a UTF-8 continuation byte
        if bytes[i] == b'<' {
            let rest = &html[i..];
            let mut found_skip = false;
            for tag in &skip_tags {
                let open = format!("<{}", tag);
                if starts_with_ascii_ci(rest, &open) {
                    // Check delimiter after tag name (open is ASCII, so len is byte-safe)
                    let after = &rest[open.len()..];
                    if after.is_empty()
                        || after.starts_with(' ')
                        || after.starts_with('>')
                        || after.starts_with('\t')
                        || after.starts_with('\n')
                    {
                        // Find the closing tag (case-insensitive)
                        let close = format!("</{}>", tag);
                        if let Some(end_pos) = find_ascii_ci(rest, &close) {
                            i += end_pos + close.len();
                            found_skip = true;
                            break;
                        }
                    }
                }
            }
            if !found_skip {
                cleaned.push('<');
                i += 1; // '<' is 1 byte
            }
        } else {
            // Copy one full UTF-8 character. i is always at a char boundary
            // because we only advance by char len or past single-byte ASCII '<'.
            if let Some(c) = html[i..].chars().next() {
                cleaned.push(c);
                i += c.len_utf8();
            } else {
                break;
            }
        }
    }

    // Second pass: convert meaningful tags to formatting, strip the rest.
    // Tag delimiters '<' and '>' are ASCII, so byte-scanning for them is safe
    // in UTF-8. Non-tag text is copied char-by-char to preserve multi-byte chars.
    let mut result = String::with_capacity(cleaned.len());
    let cbytes = cleaned.as_bytes();
    let mut j = 0;

    while j < cbytes.len() {
        if cbytes[j] == b'<' {
            let tag_start = j;
            let mut tag_end = j + 1;
            // '>' is ASCII — safe to scan byte-by-byte
            while tag_end < cbytes.len() && cbytes[tag_end] != b'>' {
                tag_end += 1;
            }
            if tag_end < cbytes.len() {
                tag_end += 1; // include '>'
            }

            let tag_content = &cleaned[tag_start..tag_end.min(cbytes.len())];

            if starts_with_ascii_ci(tag_content, "<br") {
                result.push('\n');
            } else if starts_with_ascii_ci(tag_content, "<li") {
                result.push_str("\n• ");
            } else if starts_with_ascii_ci(tag_content, "<h1")
                || starts_with_ascii_ci(tag_content, "<h2")
                || starts_with_ascii_ci(tag_content, "<h3")
                || starts_with_ascii_ci(tag_content, "<h4")
                || starts_with_ascii_ci(tag_content, "<h5")
                || starts_with_ascii_ci(tag_content, "<h6")
            {
                result.push_str("\n\n");
            } else if starts_with_ascii_ci(tag_content, "</h")
                || starts_with_ascii_ci(tag_content, "<p")
                || starts_with_ascii_ci(tag_content, "</p")
                || starts_with_ascii_ci(tag_content, "<div")
                || starts_with_ascii_ci(tag_content, "</div")
                || starts_with_ascii_ci(tag_content, "<tr")
                || starts_with_ascii_ci(tag_content, "</tr")
                || starts_with_ascii_ci(tag_content, "<blockquote")
                || starts_with_ascii_ci(tag_content, "</blockquote")
                || starts_with_ascii_ci(tag_content, "<section")
                || starts_with_ascii_ci(tag_content, "</section")
                || starts_with_ascii_ci(tag_content, "<article")
                || starts_with_ascii_ci(tag_content, "</article")
            {
                result.push('\n');
            }
            // All other tags: skip (emit nothing)

            j = tag_end;
        } else {
            // Copy one full UTF-8 character
            if let Some(c) = cleaned[j..].chars().next() {
                result.push(c);
                j += c.len_utf8();
            } else {
                break;
            }
        }
    }

    // Decode HTML entities (shared utility)
    let decoded = crate::format::decode_html_entities(&result);

    // Collapse whitespace: multiple blank lines → two newlines, multiple spaces → one
    let mut final_text = String::with_capacity(decoded.len());
    let mut prev_newlines = 0u32;
    let mut prev_space = false;

    for c in decoded.chars() {
        if c == '\n' {
            prev_newlines += 1;
            prev_space = false;
            if prev_newlines <= 2 {
                final_text.push('\n');
            }
        } else if c == ' ' || c == '\t' {
            if prev_newlines > 0 {
                // Skip spaces right after newlines (trim line starts)
            } else if !prev_space {
                final_text.push(' ');
                prev_space = true;
            }
        } else {
            prev_newlines = 0;
            prev_space = false;
            final_text.push(c);
        }
    }

    // Trim each line and rejoin
    let final_text: String = final_text
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n");

    let final_text = final_text.trim().to_string();

    // Truncate to max_chars
    if final_text.len() > max_chars {
        let truncated = &final_text[..final_text.floor_char_boundary(max_chars)];
        format!("{truncated}\n\n[… truncated at {max_chars} chars]")
    } else {
        final_text
    }
}

/// Validate that a string looks like a URL.
pub fn is_valid_url(url: &str) -> bool {
    (url.starts_with("http://") || url.starts_with("https://"))
        && url.len() > 10
        && url.contains('.')
}

/// Fetch a URL using curl and return the HTML content.
pub(crate) fn fetch_url(url: &str) -> Result<String, String> {
    let output = std::process::Command::new("curl")
        .args([
            "-sL", // silent, follow redirects
            "--max-time",
            "15", // timeout
            "-A",
            "Mozilla/5.0 (compatible; yoyo-agent/0.1)", // user agent
            url,
        ])
        .output()
        .map_err(|e| format!("failed to run curl: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "curl failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    if body.is_empty() {
        return Err("empty response".to_string());
    }

    Ok(body)
}

/// Handle the /web command — fetch a URL and display readable text.
pub fn handle_web(input: &str) {
    let url = input.trim_start_matches("/web").trim();

    if url.is_empty() {
        println!("{DIM}  usage: /web <url>");
        println!("  Fetch a web page and display readable text content.");
        println!(
            "  Example: /web https://doc.rust-lang.org/book/ch01-01-installation.html{RESET}\n"
        );
        return;
    }

    // Auto-prepend https:// if missing
    let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{url}")
    } else {
        url.to_string()
    };

    if !is_valid_url(&url) {
        println!("{RED}  Invalid URL: {url}{RESET}\n");
        return;
    }

    println!("{DIM}  Fetching {url}...{RESET}");

    match fetch_url(&url) {
        Ok(html) => {
            let text = strip_html_tags(&html, WEB_MAX_CHARS);
            if text.is_empty() {
                println!("{DIM}  (no readable text content found){RESET}\n");
            } else {
                let line_count = text.lines().count();
                let char_count = text.len();
                println!();
                println!("{text}");
                println!();
                println!("{DIM}  ── {line_count} lines, {char_count} chars from {url}{RESET}\n");
            }
        }
        Err(e) => {
            println!("{RED}  Failed to fetch: {e}{RESET}\n");
        }
    }
}

pub const COPY_SUBCOMMANDS: &[&str] = &["last", "code"];

/// Extract the text content of the last assistant message from the message
/// list.  Returns `None` if there are no assistant messages.
pub(crate) fn extract_last_assistant_text(messages: &[yoagent::AgentMessage]) -> Option<String> {
    for msg in messages.iter().rev() {
        if let yoagent::AgentMessage::Llm(yoagent::Message::Assistant { content, .. }) = msg {
            let text: String = content
                .iter()
                .filter_map(|c| match c {
                    yoagent::Content::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// Extract the last fenced code block (` ```...``` `) from a markdown string.
/// Returns the code content without the fence markers.
pub(crate) fn extract_last_code_block(text: &str) -> Option<String> {
    let mut blocks: Vec<String> = Vec::new();
    let mut in_block = false;
    let mut current_block = String::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_block {
                // Closing fence
                blocks.push(current_block.clone());
                current_block.clear();
                in_block = false;
            } else {
                // Opening fence — skip the language tag
                in_block = true;
                current_block.clear();
            }
        } else if in_block {
            if !current_block.is_empty() {
                current_block.push('\n');
            }
            current_block.push_str(line);
        }
    }

    blocks.last().cloned()
}

/// Return the platform-appropriate clipboard command name.
///
/// Returns `(command, args)` for the clipboard tool that should receive text
/// on stdin.  Returns `None` if no known tool is available.
fn clipboard_command() -> Option<(&'static str, Vec<&'static str>)> {
    if cfg!(target_os = "macos") {
        Some(("pbcopy", vec![]))
    } else if cfg!(target_os = "windows") {
        Some(("clip.exe", vec![]))
    } else {
        // Linux: try Wayland first, then X11 tools
        if command_exists("wl-copy") {
            Some(("wl-copy", vec![]))
        } else if command_exists("xclip") {
            Some(("xclip", vec!["-selection", "clipboard"]))
        } else if command_exists("xsel") {
            Some(("xsel", vec!["--clipboard", "--input"]))
        } else {
            None
        }
    }
}

/// Check if a command exists on the system PATH.
fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Copy text to the system clipboard.  Returns `Ok(())` on success.
fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let (cmd, args) = clipboard_command().ok_or_else(|| {
        if cfg!(target_os = "linux") {
            "No clipboard tool found. Install one of: wl-copy (Wayland), xclip, or xsel".to_string()
        } else {
            "No clipboard tool found".to_string()
        }
    })?;

    let mut child = std::process::Command::new(cmd)
        .args(&args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run {cmd}: {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to {cmd}: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Failed to wait for {cmd}: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{cmd} failed: {}", stderr.trim()))
    }
}

/// Handle the `/copy` slash command.
///
/// - `/copy` or `/copy last` — copy the last assistant message text
/// - `/copy code` — copy the last code block from the last assistant message
/// - `/copy <text>` — copy the literal text argument
pub fn handle_copy(input: &str, messages: &[yoagent::AgentMessage]) {
    let args = input.strip_prefix("/copy").unwrap_or("").trim().to_string();

    let text = if args.is_empty() || args == "last" {
        // Copy last assistant message text
        match extract_last_assistant_text(messages) {
            Some(t) => t,
            None => {
                eprintln!("{YELLOW}  No assistant messages to copy{RESET}");
                return;
            }
        }
    } else if args == "code" {
        // Copy last code block from last assistant message
        let assistant_text = match extract_last_assistant_text(messages) {
            Some(t) => t,
            None => {
                eprintln!("{YELLOW}  No assistant messages to copy{RESET}");
                return;
            }
        };
        match extract_last_code_block(&assistant_text) {
            Some(code) => code,
            None => {
                eprintln!("{YELLOW}  No code blocks found in the last response{RESET}");
                return;
            }
        }
    } else {
        // Copy the literal text
        args
    };

    let char_count = text.len();
    match copy_to_clipboard(&text) {
        Ok(()) => {
            eprintln!("{GREEN}  ✓ Copied {char_count} chars to clipboard{RESET}");
        }
        Err(e) => {
            eprintln!("{RED}  ✗ {e}{RESET}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_html_tags ──────────────────────────────────────────────

    #[test]
    fn strip_html_basic_paragraph() {
        let html = "<p>Hello, world!</p>";
        let text = strip_html_tags(html, 5000);
        assert_eq!(text, "Hello, world!");
    }

    #[test]
    fn strip_html_removes_script_and_style() {
        let html =
            "<p>Before</p><script>alert('xss');</script><style>.x{color:red}</style><p>After</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Before"));
        assert!(text.contains("After"));
        assert!(!text.contains("alert"));
        assert!(!text.contains("color:red"));
    }

    #[test]
    fn strip_html_removes_nav_footer_header() {
        let html = "<header>Nav stuff</header><p>Content</p><footer>Footer stuff</footer>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Content"));
        assert!(!text.contains("Nav stuff"));
        assert!(!text.contains("Footer stuff"));
    }

    #[test]
    fn strip_html_converts_br_to_newline() {
        let html = "Line 1<br>Line 2<br/>Line 3";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Line 1\nLine 2\nLine 3"));
    }

    #[test]
    fn strip_html_converts_li_to_bullets() {
        let html = "<ul><li>First</li><li>Second</li><li>Third</li></ul>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("• First"));
        assert!(text.contains("• Second"));
        assert!(text.contains("• Third"));
    }

    #[test]
    fn strip_html_headings() {
        let html = "<h1>Title</h1><p>Content</p><h2>Subtitle</h2>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Title"));
        assert!(text.contains("Content"));
        assert!(text.contains("Subtitle"));
    }

    #[test]
    fn strip_html_decodes_entities() {
        let html = "<p>5 &gt; 3 &amp; 2 &lt; 4</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("5 > 3 & 2 < 4"));
    }

    #[test]
    fn strip_html_decodes_numeric_entities() {
        let html = "<p>&#65;&#66;&#67;</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("ABC"));
    }

    #[test]
    fn strip_html_decodes_quotes_and_apostrophes() {
        let html = "<p>&quot;hello&quot; &amp; &apos;world&apos;</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("\"hello\" & 'world'"));
    }

    #[test]
    fn strip_html_collapses_whitespace() {
        let html = "<p>Hello</p>   \n\n\n\n\n   <p>World</p>";
        let text = strip_html_tags(html, 5000);
        // Should not have more than 2 consecutive newlines
        assert!(!text.contains("\n\n\n"));
    }

    #[test]
    fn strip_html_truncates_long_content() {
        let html = "<p>".to_string() + &"x".repeat(6000) + "</p>";
        let text = strip_html_tags(&html, 100);
        assert!(text.len() < 200); // truncated text + suffix
        assert!(text.contains("[… truncated at 100 chars]"));
    }

    #[test]
    fn strip_html_empty_input() {
        let text = strip_html_tags("", 5000);
        assert_eq!(text, "");
    }

    #[test]
    fn strip_html_no_tags() {
        let text = strip_html_tags("Just plain text", 5000);
        assert_eq!(text, "Just plain text");
    }

    #[test]
    fn strip_html_nested_tags() {
        let html = "<div><p>Inside <strong>bold</strong> and <em>italic</em></p></div>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Inside bold and italic"));
    }

    #[test]
    fn strip_html_case_insensitive_tags() {
        let html = "<SCRIPT>bad</SCRIPT><P>Good</P>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Good"));
        assert!(!text.contains("bad"));
    }

    #[test]
    fn strip_html_nbsp() {
        let html = "<p>word&nbsp;word</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("word word"));
    }

    #[test]
    fn strip_html_non_ascii_content() {
        // Common non-ASCII characters: middle dot, em dash, accented letters
        let html = "<p>Price · $10 — café résumé</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("·"), "Should preserve middle dot");
        assert!(text.contains("—"), "Should preserve em dash");
        assert!(text.contains("café"), "Should preserve accented chars");
        assert!(text.contains("résumé"), "Should preserve accented chars");
    }

    #[test]
    fn strip_html_non_ascii_in_skip_tag() {
        // Non-ASCII inside script tags should not panic
        let html = "<p>Before</p><script>alert('café — naïve')</script><p>After</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Before"));
        assert!(text.contains("After"));
        assert!(!text.contains("café"));
    }

    #[test]
    fn strip_html_chinese_japanese() {
        let html = "<p>中文测试</p><div>日本語テスト</div>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("中文测试"), "Should preserve Chinese");
        assert!(text.contains("日本語テスト"), "Should preserve Japanese");
    }

    #[test]
    fn strip_html_mixed_multibyte() {
        // Mix of ASCII and multi-byte throughout, including emoji
        let html = "<h1>Hello 🌍 World</h1><p>naïve · recipe — Pro™</p>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("Hello 🌍 World"), "Should preserve emoji");
        assert!(text.contains("naïve"), "Should preserve accented chars");
        assert!(text.contains("·"), "Should preserve middle dot");
        assert!(text.contains("—"), "Should preserve em dash");
        assert!(text.contains("Pro™"), "Should preserve trademark");
    }

    #[test]
    fn strip_html_emoji_in_tags() {
        let html = "<li>🎉 Party</li><li>🚀 Launch</li>";
        let text = strip_html_tags(html, 5000);
        assert!(text.contains("🎉 Party"));
        assert!(text.contains("🚀 Launch"));
    }

    #[test]
    fn strip_html_non_ascii_truncation() {
        // Ensure truncation with non-ASCII doesn't panic
        let html = "<p>".to_string() + &"café ".repeat(1000) + "</p>";
        let text = strip_html_tags(&html, 100);
        assert!(text.contains("[… truncated at 100 chars]"));
    }

    #[test]
    fn valid_urls() {
        assert!(is_valid_url("https://example.com"));
        assert!(is_valid_url("http://docs.rs/yoagent"));
        assert!(is_valid_url(
            "https://doc.rust-lang.org/book/ch01-01-installation.html"
        ));
    }

    #[test]
    fn invalid_urls() {
        assert!(!is_valid_url("not-a-url"));
        assert!(!is_valid_url("ftp://files.com"));
        assert!(!is_valid_url("https://"));
        assert!(!is_valid_url("http://x"));
        assert!(!is_valid_url(""));
    }

    #[test]
    fn test_extract_last_code_block_single() {
        let md = "Here is some code:\n```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n";
        let block = extract_last_code_block(md).unwrap();
        assert!(block.contains("fn main()"));
        assert!(block.contains("println!"));
        assert!(!block.contains("```"));
    }

    #[test]
    fn test_extract_last_code_block_multiple() {
        let md = "First:\n```\nfirst block\n```\nSecond:\n```python\nprint('hi')\n```\n";
        let block = extract_last_code_block(md).unwrap();
        assert_eq!(block, "print('hi')");
    }

    #[test]
    fn test_extract_last_code_block_none() {
        let md = "No code blocks here, just text.";
        assert!(extract_last_code_block(md).is_none());
    }

    #[test]
    fn test_extract_last_code_block_unclosed() {
        // Unclosed fence should not produce a block
        let md = "```rust\nfn foo() {}\n";
        assert!(extract_last_code_block(md).is_none());
    }

    #[test]

    fn test_extract_last_assistant_text_finds_last() {
        use yoagent::*;
        let messages = vec![
            AgentMessage::Llm(Message::User {
                content: vec![Content::Text {
                    text: "hello".into(),
                }],
                timestamp: 0,
            }),
            AgentMessage::Llm(Message::Assistant {
                content: vec![Content::Text {
                    text: "first response".into(),
                }],
                stop_reason: StopReason::Stop,
                model: "test".into(),
                provider: "test".into(),
                usage: Usage::default(),
                timestamp: 1,
                error_message: None,
            }),
            AgentMessage::Llm(Message::User {
                content: vec![Content::Text {
                    text: "followup".into(),
                }],
                timestamp: 2,
            }),
            AgentMessage::Llm(Message::Assistant {
                content: vec![Content::Text {
                    text: "second response".into(),
                }],
                stop_reason: StopReason::Stop,
                model: "test".into(),
                provider: "test".into(),
                usage: Usage::default(),
                timestamp: 3,
                error_message: None,
            }),
        ];
        let text = extract_last_assistant_text(&messages).unwrap();
        assert_eq!(text, "second response");
    }

    #[test]
    fn test_extract_last_assistant_text_empty() {
        let messages: Vec<yoagent::AgentMessage> = vec![];
        assert!(extract_last_assistant_text(&messages).is_none());
    }

    #[test]
    fn test_extract_last_assistant_text_no_assistant() {
        use yoagent::*;
        let messages = vec![AgentMessage::Llm(Message::User {
            content: vec![Content::Text {
                text: "hello".into(),
            }],
            timestamp: 0,
        })];
        assert!(extract_last_assistant_text(&messages).is_none());
    }

    #[test]

    fn test_clipboard_command_platform() {
        // We can't test what clipboard_command returns deterministically
        // across CI environments, but we can verify it doesn't panic
        // and returns a valid shape.
        let result = super::clipboard_command();
        // On CI (Linux without display), it may return None — that's fine.
        if let Some((cmd, args)) = result {
            assert!(!cmd.is_empty());
            // args can be empty (pbcopy, clip.exe) or non-empty (xclip)
            let _ = args;
        }
    }

    #[test]
    fn test_is_valid_url_detection() {
        // URLs should be detected
        assert!(is_valid_url("https://docs.rs/some-crate"));
        assert!(is_valid_url("http://example.com"));
        assert!(is_valid_url(
            "https://doc.rust-lang.org/book/ch01-01-installation.html"
        ));

        // Regular file paths should NOT be detected as URLs
        assert!(!is_valid_url("src/main.rs"));
        assert!(!is_valid_url("Cargo.toml"));
        assert!(!is_valid_url("src/*.rs"));
        assert!(!is_valid_url("./relative/path.txt"));
        assert!(!is_valid_url(""));
        assert!(!is_valid_url("http://x")); // too short, no dot
    }
}
