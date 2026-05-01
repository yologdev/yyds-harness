//! Code review command handlers: /review and /blame.

use crate::commands::auto_compact_if_needed;
use crate::format::*;
use crate::git::*;
use crate::prompt::*;

use yoagent::agent::Agent;
use yoagent::*;

/// Build a review prompt for either staged changes or a specific file.
/// Returns None if there's nothing to review, Some(prompt) otherwise.
pub fn build_review_content(arg: &str) -> Option<(String, String)> {
    let arg = arg.trim();
    if arg.is_empty() {
        // Review staged changes
        match get_staged_diff() {
            None => {
                eprintln!("{RED}  error: not in a git repository{RESET}\n");
                None
            }
            Some(diff) if diff.trim().is_empty() => {
                // Fall back to unstaged diff if nothing staged
                let unstaged = run_git(&["diff"]).unwrap_or_default();
                if unstaged.trim().is_empty() {
                    println!("{DIM}  nothing to review — no staged or unstaged changes{RESET}\n");
                    None
                } else {
                    println!("{DIM}  reviewing unstaged changes...{RESET}");
                    Some(("unstaged changes".to_string(), unstaged))
                }
            }
            Some(diff) => {
                println!("{DIM}  reviewing staged changes...{RESET}");
                Some(("staged changes".to_string(), diff))
            }
        }
    } else {
        // Review a specific file
        let path = std::path::Path::new(arg);
        if !path.exists() {
            eprintln!("{RED}  error: file not found: {arg}{RESET}\n");
            return None;
        }
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if content.trim().is_empty() {
                    println!("{DIM}  file is empty — nothing to review{RESET}\n");
                    None
                } else {
                    println!("{DIM}  reviewing {arg}...{RESET}");
                    Some((arg.to_string(), content))
                }
            }
            Err(e) => {
                eprintln!("{RED}  error reading {arg}: {e}{RESET}\n");
                None
            }
        }
    }
}

/// Build the review prompt to send to the AI.
pub fn build_review_prompt(label: &str, content: &str) -> String {
    // Truncate if very large
    let max_chars = 30_000;
    let content_preview = if content.len() > max_chars {
        let truncated = safe_truncate(content, max_chars);
        format!(
            "{truncated}\n\n... (truncated, {} more chars)",
            content.len() - max_chars
        )
    } else {
        content.to_string()
    };

    format!(
        r#"Review the following code ({label}). Look for:

1. **Bugs** — logic errors, off-by-one errors, null/None handling, race conditions
2. **Security** — injection vulnerabilities, unsafe operations, credential exposure
3. **Style** — naming, idiomatic patterns, unnecessary complexity, dead code
4. **Performance** — obvious inefficiencies, unnecessary allocations, N+1 patterns
5. **Suggestions** — improvements, missing error handling, better approaches

Be specific: reference line numbers or code snippets. Be concise — skip things that look fine.
If the code looks good overall, say so briefly and note any minor suggestions.

```
{content_preview}
```"#
    )
}

/// Handle the /review command: review staged changes or a specific file.
/// Returns the review prompt if sent to AI, None otherwise.
pub async fn handle_review(
    input: &str,
    agent: &mut Agent,
    session_total: &mut Usage,
    model: &str,
) -> Option<String> {
    let arg = input.strip_prefix("/review").unwrap_or("").trim();

    match build_review_content(arg) {
        Some((label, content)) => {
            let prompt = build_review_prompt(&label, &content);
            run_prompt(agent, &prompt, session_total, model).await;
            auto_compact_if_needed(agent);
            Some(prompt)
        }
        None => None,
    }
}

/// Parsed arguments for `/blame`.
#[derive(Debug, PartialEq)]
pub struct BlameArgs {
    pub file: String,
    pub range: Option<(usize, usize)>,
}

/// Parse `/blame <file>` or `/blame <file>:<start>-<end>`.
pub fn parse_blame_args(input: &str) -> Result<BlameArgs, String> {
    let arg = input.strip_prefix("/blame").unwrap_or(input).trim();

    if arg.is_empty() {
        return Err("Usage: /blame <file> or /blame <file>:<start>-<end>".to_string());
    }

    // Check for <file>:<start>-<end> pattern
    if let Some(colon_pos) = arg.rfind(':') {
        let file_part = &arg[..colon_pos];
        let range_part = &arg[colon_pos + 1..];

        if let Some(dash_pos) = range_part.find('-') {
            let start_str = &range_part[..dash_pos];
            let end_str = &range_part[dash_pos + 1..];

            if let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>()) {
                if start == 0 || end == 0 {
                    return Err("Line numbers must be >= 1".to_string());
                }
                if start > end {
                    return Err(format!("Invalid range: start ({start}) > end ({end})"));
                }
                if !file_part.is_empty() {
                    return Ok(BlameArgs {
                        file: file_part.to_string(),
                        range: Some((start, end)),
                    });
                }
            }
        }
    }

    // No valid range found — treat entire input as file path
    Ok(BlameArgs {
        file: arg.to_string(),
        range: None,
    })
}

/// Colorize a single line of `git blame` output.
///
/// Typical git blame line format:
/// `abc1234f (Author Name  2024-01-15 10:30:00 +0000  42) line content`
///
/// We colorize:
/// - Commit hash → DIM
/// - Author name → CYAN
/// - Date/time → DIM
/// - Line number → YELLOW
/// - Code content → default
pub fn colorize_blame_line(line: &str) -> String {
    // git blame output: <hash> (<author> <date> <time> <tz> <lineno>) <code>
    // Find the opening paren that starts the author section
    let Some(paren_open) = line.find('(') else {
        return line.to_string();
    };
    let Some(paren_close) = line.find(')') else {
        return line.to_string();
    };
    if paren_close <= paren_open {
        return line.to_string();
    }

    let hash = &line[..paren_open];
    let annotation = &line[paren_open + 1..paren_close];
    let code = if paren_close + 1 < line.len() {
        &line[paren_close + 1..]
    } else {
        ""
    };

    // Inside the annotation: "Author Name  2024-01-15 10:30:00 +0000  42"
    // Try to find the date pattern (YYYY-MM-DD) to split author from date+lineno
    let mut author = annotation;
    let mut date_and_lineno = "";

    // Look for a date pattern: 4-digit year followed by -
    for (i, _) in annotation.char_indices() {
        if i + 10 <= annotation.len() {
            let slice = &annotation[i..];
            if slice.len() >= 10
                && slice.as_bytes()[4] == b'-'
                && slice.as_bytes()[7] == b'-'
                && slice[..4].chars().all(|c| c.is_ascii_digit())
                && slice[5..7].chars().all(|c| c.is_ascii_digit())
                && slice[8..10].chars().all(|c| c.is_ascii_digit())
            {
                author = annotation[..i].trim_end();
                date_and_lineno = &annotation[i..];
                break;
            }
        }
    }

    // Try to split the lineno from date portion
    // The lineno is typically the last whitespace-separated token
    let (date_part, lineno_part) =
        if let Some(last_space) = date_and_lineno.rfind(char::is_whitespace) {
            let candidate = date_and_lineno[last_space..].trim();
            if candidate.chars().all(|c| c.is_ascii_digit()) && !candidate.is_empty() {
                (&date_and_lineno[..last_space], candidate)
            } else {
                (date_and_lineno, "")
            }
        } else {
            (date_and_lineno, "")
        };

    format!(
        "{DIM}{hash}{RESET}({CYAN}{author}{RESET} {DIM}{date_part}{RESET} {YELLOW}{lineno_part}{RESET}){code}"
    )
}

/// Colorize full `git blame` output (multiple lines).
pub fn colorize_blame(output: &str) -> String {
    output
        .lines()
        .map(colorize_blame_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Handle the `/blame` command.
pub fn handle_blame(input: &str) {
    let args = match parse_blame_args(input) {
        Ok(a) => a,
        Err(e) => {
            println!("  {RED}✗{RESET} {e}");
            return;
        }
    };

    let mut cmd = vec!["blame".to_string()];
    if let Some((start, end)) = args.range {
        cmd.push(format!("-L{start},{end}"));
    }
    cmd.push(args.file.clone());

    let cmd_refs: Vec<&str> = cmd.iter().map(|s| s.as_str()).collect();
    match run_git(&cmd_refs) {
        Ok(output) => {
            if output.trim().is_empty() {
                println!("  {DIM}(no blame output){RESET}");
            } else {
                println!();
                println!("{}", colorize_blame(&output));
            }
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("no such path") || msg.contains("No such file") {
                println!("  {RED}✗{RESET} File not found: {DIM}{}{RESET}", args.file);
            } else if msg.contains("not a git repository") || msg.contains("fatal: not a git") {
                println!("  {RED}✗{RESET} Not in a git repository");
            } else {
                println!("  {RED}✗{RESET} {msg}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{is_unknown_command, KNOWN_COMMANDS};

    #[test]
    fn build_review_prompt_contains_label() {
        let prompt = build_review_prompt("staged changes", "fn main() {}");
        assert!(
            prompt.contains("staged changes"),
            "Prompt should include the label"
        );
    }

    #[test]
    fn build_review_prompt_contains_content() {
        let code = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let prompt = build_review_prompt("test.rs", code);
        assert!(prompt.contains(code), "Prompt should include the code");
    }

    #[test]
    fn build_review_prompt_contains_review_criteria() {
        let prompt = build_review_prompt("file.rs", "let x = 1;");
        assert!(prompt.contains("Bugs"), "Should mention bugs");
        assert!(prompt.contains("Security"), "Should mention security");
        assert!(prompt.contains("Style"), "Should mention style");
        assert!(prompt.contains("Performance"), "Should mention performance");
        assert!(prompt.contains("Suggestions"), "Should mention suggestions");
    }

    #[test]
    fn build_review_prompt_truncates_large_content() {
        let large_content = "x".repeat(50_000);
        let prompt = build_review_prompt("big.rs", &large_content);
        assert!(
            prompt.contains("truncated"),
            "Large content should be truncated"
        );
        assert!(
            prompt.contains("20000 more chars"),
            "Should show remaining char count"
        );
        // The prompt should be shorter than the original content
        assert!(
            prompt.len() < large_content.len(),
            "Prompt should be shorter than 50k"
        );
    }

    #[test]
    fn build_review_prompt_does_not_truncate_small_content() {
        let small_content = "fn hello() { println!(\"hi\"); }";
        let prompt = build_review_prompt("small.rs", small_content);
        assert!(
            !prompt.contains("truncated"),
            "Small content should not be truncated"
        );
        assert!(
            prompt.contains(small_content),
            "Full content should be present"
        );
    }

    #[test]
    fn build_review_prompt_wraps_in_code_block() {
        let prompt = build_review_prompt("test.rs", "let x = 42;");
        assert!(prompt.contains("```"), "Content should be in a code block");
    }

    #[test]
    fn test_review_command_recognized() {
        assert!(!is_unknown_command("/review"));
        assert!(!is_unknown_command("/review src/main.rs"));
        assert!(
            KNOWN_COMMANDS.contains(&"/review"),
            "/review should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_review_command_matching() {
        // /review should match exact or with space separator, not /reviewing
        let review_matches = |s: &str| s == "/review" || s.starts_with("/review ");
        assert!(review_matches("/review"));
        assert!(review_matches("/review src/main.rs"));
        assert!(review_matches("/review Cargo.toml"));
        assert!(!review_matches("/reviewing"));
        assert!(!review_matches("/reviewer"));
    }

    #[test]
    fn test_build_review_prompt_contains_content() {
        let prompt =
            build_review_prompt("staged changes", "fn main() {\n    println!(\"hello\");\n}");
        assert!(
            prompt.contains("staged changes"),
            "Should mention the label"
        );
        assert!(prompt.contains("fn main()"), "Should contain the code");
        assert!(prompt.contains("Bugs"), "Should ask for bug review");
        assert!(
            prompt.contains("Security"),
            "Should ask for security review"
        );
        assert!(prompt.contains("Style"), "Should ask for style review");
        assert!(
            prompt.contains("Performance"),
            "Should ask for performance review"
        );
        assert!(prompt.contains("Suggestions"), "Should ask for suggestions");
    }

    #[test]
    fn test_build_review_prompt_truncates_large_content() {
        let large_content = "x".repeat(40_000);
        let prompt = build_review_prompt("big file", &large_content);
        assert!(
            prompt.contains("truncated"),
            "Large content should be truncated"
        );
        assert!(
            prompt.len() < 40_000,
            "Prompt should be truncated, got {} chars",
            prompt.len()
        );
    }

    #[test]
    fn test_build_review_content_nonexistent_file() {
        let result = build_review_content("nonexistent_file_xyz_12345.rs");
        assert!(result.is_none(), "Nonexistent file should return None");
    }

    #[test]
    fn test_build_review_content_existing_file() {
        // Use CARGO_MANIFEST_DIR for an absolute path to avoid CWD races
        // with other tests that call set_current_dir
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let cargo_toml = format!("{manifest_dir}/Cargo.toml");
        let result = build_review_content(&cargo_toml);
        assert!(result.is_some(), "Existing file should return Some");
        let (label, content) = result.unwrap();
        assert_eq!(label, cargo_toml);
        assert!(!content.is_empty(), "Content should not be empty");
    }

    #[test]
    fn test_build_review_content_empty_arg_in_git_repo() {
        // Empty arg reviews staged/unstaged changes
        // In CI, this may or may not have changes — just verify it doesn't panic
        let result = build_review_content("");
        // Result depends on git state — either Some or None is valid
        if let Some((label, _content)) = result {
            assert!(
                label.contains("changes"),
                "Label should describe what's being reviewed: {label}"
            );
        }
    }

    #[test]
    fn test_review_help_text_present() {
        // Verify /review appears in the help output by checking the handle_help function output
        // We can't easily capture stdout, but we can verify the command is in KNOWN_COMMANDS
        // and that the help text format is correct
        assert!(KNOWN_COMMANDS.contains(&"/review"));
    }

    #[test]
    fn test_parse_blame_args_file_only() {
        let result = parse_blame_args("/blame src/main.rs").unwrap();
        assert_eq!(result.file, "src/main.rs");
        assert_eq!(result.range, None);
    }

    #[test]
    fn test_parse_blame_args_with_range() {
        let result = parse_blame_args("/blame src/main.rs:10-20").unwrap();
        assert_eq!(result.file, "src/main.rs");
        assert_eq!(result.range, Some((10, 20)));
    }

    #[test]
    fn test_parse_blame_args_single_line_range() {
        let result = parse_blame_args("/blame foo.rs:5-5").unwrap();
        assert_eq!(result.file, "foo.rs");
        assert_eq!(result.range, Some((5, 5)));
    }

    #[test]
    fn test_parse_blame_args_no_args() {
        let result = parse_blame_args("/blame");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Usage"));
    }

    #[test]
    fn test_parse_blame_args_no_args_with_spaces() {
        let result = parse_blame_args("/blame   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_blame_args_invalid_range_reversed() {
        let result = parse_blame_args("/blame foo.rs:20-10");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("start"));
    }

    #[test]
    fn test_parse_blame_args_zero_start() {
        let result = parse_blame_args("/blame foo.rs:0-10");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(">= 1"));
    }

    #[test]
    fn test_parse_blame_args_non_numeric_range_treated_as_file() {
        // If the range part doesn't parse as numbers, treat entire input as filename
        let result = parse_blame_args("/blame some:file:thing").unwrap();
        assert_eq!(result.file, "some:file:thing");
        assert_eq!(result.range, None);
    }

    #[test]
    fn test_colorize_blame_line_typical() {
        let line = "abc1234f (John Doe  2024-01-15 10:30:00 +0000  42) fn main() {";
        let colored = colorize_blame_line(line);
        // Should contain ANSI codes
        assert!(colored.contains("\x1b["));
        // Should still contain the original content
        assert!(colored.contains("John Doe"));
        assert!(colored.contains("fn main()"));
        assert!(colored.contains("abc1234f"));
    }

    #[test]
    fn test_colorize_blame_line_no_paren() {
        // Lines without parens should be returned unchanged
        let line = "some weird line without parens";
        assert_eq!(colorize_blame_line(line), line);
    }

    #[test]
    fn test_colorize_blame_multiple_lines() {
        let input = "abc123 (Alice 2024-01-15 10:00:00 +0000 1) line1\ndef456 (Bob   2024-01-15 10:00:00 +0000 2) line2";
        let colored = colorize_blame(input);
        let lines: Vec<&str> = colored.lines().collect();
        assert_eq!(lines.len(), 2);
        // Both lines should have ANSI codes
        assert!(lines[0].contains("\x1b["));
        assert!(lines[1].contains("\x1b["));
    }
}
