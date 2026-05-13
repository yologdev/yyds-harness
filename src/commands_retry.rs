//! `/retry` and `/changes` REPL command handlers.
//!
//! Extracted from `commands.rs` as another slice of issue #260, which tracks
//! splitting the multi-thousand-line `commands.rs` into focused modules.
//! These two handlers are self-contained and only touch session state through
//! well-defined helpers (`build_retry_prompt`, `run_prompt`,
//! `auto_compact_if_needed`, `format_changes`), which makes them a safe,
//! mechanical slice to pull out.

use crate::commands_session::auto_compact_if_needed;
use crate::format::*;
use crate::git::{colorize_diff, run_git};
use crate::prompt::run_prompt;
use crate::prompt_retry::build_retry_prompt;
use crate::session::{format_changes, ChangeKind, FileChange, SessionChanges};
use crate::AgentConfig;

use std::time::Instant;
use yoagent::agent::Agent;
use yoagent::*;

pub async fn handle_retry(
    agent: &mut Agent,
    last_input: &Option<String>,
    last_error: &Option<String>,
    session_total: &mut Usage,
    model: &str,
) -> Option<String> {
    match last_input {
        Some(prev) => {
            let retry_input = build_retry_prompt(prev, last_error);
            if last_error.is_some() {
                println!("{DIM}  (retrying with error context){RESET}");
            } else {
                println!("{DIM}  (retrying last input){RESET}");
            }
            let outcome = run_prompt(agent, &retry_input, session_total, model).await;
            auto_compact_if_needed(agent);
            outcome.last_tool_error
        }
        None => {
            eprintln!("{DIM}  (nothing to retry — no previous input){RESET}\n");
            None
        }
    }
}

/// Returns a compact 2–3 line session summary for display on REPL exit, or
/// `None` if neither files were modified nor tokens were used (i.e., no real
/// interaction happened).
///
/// Example output:
/// ```text
///   ─── session summary ────────────────────────────────
///   Duration: 4m 32s · Tokens: 12.5K in / 3.2K out · Cost: ~$0.05
///   Files changed: 3 (src/main.rs, src/lib.rs +new, tests/test.rs)
/// ```
pub fn format_exit_summary(
    changes: &SessionChanges,
    session_total: &Usage,
    model: &str,
    session_start: Instant,
) -> Option<String> {
    let snapshot = changes.snapshot();
    let has_files = !snapshot.is_empty();
    let has_tokens = session_total.input > 0 || session_total.output > 0;

    if !has_files && !has_tokens {
        return None;
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "{DIM}  ─── session summary ────────────────────────────────{RESET}"
    ));

    // Stats line: duration · tokens · cost (all on one line)
    let elapsed = session_start.elapsed();
    let mut stats = vec![format!("Duration: {}", format_duration(elapsed))];

    if has_tokens {
        stats.push(format!(
            "Tokens: {} in / {} out",
            format_token_count(session_total.input),
            format_token_count(session_total.output),
        ));
    }

    if let Some(cost) = estimate_cost(session_total, model) {
        stats.push(format!("Cost: ~{}", format_cost(cost)));
    }

    lines.push(format!("{DIM}  {}{RESET}", stats.join(" · ")));

    // Files line with names
    if has_files {
        lines.push(format!(
            "{DIM}  Files changed: {} ({}){RESET}",
            snapshot.len(),
            format_file_list(&snapshot),
        ));
    }

    Some(lines.join("\n"))
}

/// Format a list of changed files for the exit summary. Files that were
/// newly written (not edited) get a `+new` suffix. If the combined list
/// would exceed 60 chars, it is truncated with `…`.
fn format_file_list(snapshot: &[FileChange]) -> String {
    let mut names: Vec<String> = snapshot
        .iter()
        .map(|fc| {
            let name = fc.path.as_str();
            if fc.kind == ChangeKind::Write {
                format!("{name} +new")
            } else {
                name.to_string()
            }
        })
        .collect();
    names.sort();
    let joined = names.join(", ");
    if joined.len() <= 60 {
        joined
    } else {
        // Find a safe char boundary for truncation
        let mut b = 57;
        while b > 0 && !joined.is_char_boundary(b) {
            b -= 1;
        }
        format!("{}…", &joined[..b])
    }
}

/// Returns `true` if the raw `/changes` input contains the `--diff` flag.
fn wants_diff(input: &str) -> bool {
    input
        .split_whitespace()
        .skip(1) // skip "/changes" itself
        .any(|arg| arg == "--diff")
}

/// Returns `true` if the raw `/changes` input contains the `summary` subcommand.
pub(crate) fn wants_summary(input: &str) -> bool {
    input
        .split_whitespace()
        .skip(1) // skip "/changes" itself
        .any(|arg| arg == "summary")
}

/// Collect colorized git diffs for the given file paths.
///
/// For each file we try both unstaged (`git diff`) and staged
/// (`git diff --cached`) so we catch changes regardless of staging state.
fn collect_diffs(paths: &[String]) -> String {
    let mut out = String::new();
    for path in paths {
        // Try unstaged diff first, then staged
        let unstaged = run_git(&["diff", "--", path]).unwrap_or_default();
        let staged = run_git(&["diff", "--cached", "--", path]).unwrap_or_default();

        let combined = match (unstaged.is_empty(), staged.is_empty()) {
            (false, false) => format!("{unstaged}\n{staged}"),
            (false, true) => unstaged,
            (true, false) => staged,
            (true, true) => String::new(),
        };

        if combined.is_empty() {
            out.push_str(&format!("    {DIM}({path}: no diff available){RESET}\n"));
        } else {
            out.push_str(&colorize_diff(&combined));
            out.push('\n');
        }
    }
    out
}

/// Handle `/changes summary` — generate an AI-written summary of session changes.
///
/// Collects diffs for every file modified this session, sends them to a
/// side agent, and streams the natural-language summary to stdout.
pub(crate) async fn handle_changes_summary(changes: &SessionChanges, agent_config: &AgentConfig) {
    let snapshot = changes.snapshot();
    if snapshot.is_empty() {
        eprintln!("{DIM}  No files modified yet this session — nothing to summarize.{RESET}\n");
        return;
    }

    // Collect diffs for all changed files
    let paths: Vec<String> = snapshot.iter().map(|c| c.path.clone()).collect();
    let mut diff_sections = String::new();
    for path in &paths {
        let unstaged = run_git(&["diff", "--", path]).unwrap_or_default();
        let staged = run_git(&["diff", "--cached", "--", path]).unwrap_or_default();
        let combined = match (unstaged.is_empty(), staged.is_empty()) {
            (false, false) => format!("{unstaged}\n{staged}"),
            (false, true) => unstaged,
            (true, false) => staged,
            (true, true) => String::new(),
        };

        diff_sections.push_str(&format!("### {path}\n"));
        if combined.is_empty() {
            diff_sections.push_str("(new file or no diff available)\n\n");
        } else {
            diff_sections.push_str("```diff\n");
            diff_sections.push_str(&combined);
            diff_sections.push_str("\n```\n\n");
        }
    }

    let prompt = format!(
        "Here are the file changes made during this coding session.\n\
         Write a concise summary suitable for a PR description or commit message.\n\
         Group related changes together. Be specific about what changed and why \
         (infer the purpose from the diff content).\n\
         Use markdown with bullet points. Start with a one-line overall summary, \
         then list each logical change.\n\n\
         ## Changed files\n\n{diff_sections}"
    );

    eprintln!("{DIM}  [summary] generating...{RESET}");

    let mut side_agent = agent_config.build_side_agent();
    let mut rx = side_agent.prompt(&prompt).await;

    let mut md_renderer = MarkdownRenderer::new();
    let mut started = false;

    loop {
        match rx.recv().await {
            Some(AgentEvent::MessageUpdate {
                delta: StreamDelta::Text { delta },
                ..
            }) => {
                if !started {
                    print!("\n{DIM}[summary]{RESET} ");
                    started = true;
                }
                let rendered = md_renderer.render_delta(&delta);
                if !rendered.is_empty() {
                    print!("{rendered}");
                }
            }
            Some(AgentEvent::MessageEnd { .. }) => {
                let tail = md_renderer.flush();
                if !tail.is_empty() {
                    print!("{tail}");
                }
            }
            Some(AgentEvent::AgentEnd { .. }) => break,
            None => break,
            _ => {}
        }
    }

    side_agent.finish().await;

    if !started {
        eprintln!("{DIM}  (no summary generated){RESET}");
    }
    println!();
}

pub fn handle_changes(changes: &SessionChanges, input: &str) {
    let output = format_changes(changes);
    if output.is_empty() {
        println!("{DIM}  No files modified yet this session.");
        println!(
            "  Files touched by write_file or edit_file tool calls will appear here.{RESET}\n"
        );
        return;
    }

    println!("{DIM}{output}{RESET}");

    if wants_diff(input) {
        let snapshot = changes.snapshot();
        let paths: Vec<String> = snapshot.iter().map(|c| c.path.clone()).collect();
        let diffs = collect_diffs(&paths);
        if !diffs.is_empty() {
            println!("{diffs}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a Usage with given input/output token counts.
    fn make_usage(input: u64, output: u64) -> Usage {
        Usage {
            input,
            output,
            ..Usage::default()
        }
    }

    #[test]
    fn test_handle_changes_empty_does_not_panic() {
        let changes = SessionChanges::new();
        // Should not panic -- just prints a message
        handle_changes(&changes, "/changes");
    }

    #[test]
    fn test_handle_changes_with_entries_does_not_panic() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/cli.rs", ChangeKind::Edit);
        // Should not panic
        handle_changes(&changes, "/changes");
    }

    #[test]
    fn test_handle_changes_diff_flag_does_not_panic() {
        let changes = SessionChanges::new();
        // Empty session with --diff should not panic
        handle_changes(&changes, "/changes --diff");
    }

    #[test]
    fn test_handle_changes_diff_flag_with_entries_does_not_panic() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        // With files and --diff -- may not produce real diffs in test env, but shouldn't panic
        handle_changes(&changes, "/changes --diff");
    }

    #[test]
    fn test_wants_diff_flag_parsing() {
        assert!(!wants_diff("/changes"));
        assert!(wants_diff("/changes --diff"));
        assert!(wants_diff("/changes   --diff"));
        assert!(!wants_diff("/changes --dif"));
        assert!(!wants_diff("/changes --verbose"));
    }

    #[test]
    fn test_wants_summary_parsing() {
        assert!(!wants_summary("/changes"));
        assert!(!wants_summary("/changes --diff"));
        assert!(wants_summary("/changes summary"));
        assert!(wants_summary("/changes   summary"));
        // summary is a subcommand, not a flag — "summari" shouldn't match
        assert!(!wants_summary("/changes summari"));
        assert!(!wants_summary("/changes --summary"));
    }

    #[test]
    fn test_format_exit_summary_empty_returns_none() {
        let changes = SessionChanges::new();
        let usage = Usage::default();
        assert!(format_exit_summary(&changes, &usage, "unknown-model", Instant::now()).is_none());
    }

    #[test]
    fn test_format_exit_summary_single_write() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        let usage = make_usage(1000, 200);
        let summary =
            format_exit_summary(&changes, &usage, "unknown-model", Instant::now()).unwrap();
        assert!(summary.contains("session summary"), "header missing");
        assert!(summary.contains("Duration:"), "duration missing");
        assert!(summary.contains("Tokens:"), "tokens missing");
        // File name should appear with +new marker
        assert!(summary.contains("Files changed: 1"), "file count missing");
        assert!(summary.contains("src/main.rs +new"), "+new marker missing");
    }

    #[test]
    fn test_format_exit_summary_single_edit() {
        let changes = SessionChanges::new();
        changes.record("src/cli.rs", ChangeKind::Edit);
        let usage = make_usage(500, 100);
        let summary =
            format_exit_summary(&changes, &usage, "unknown-model", Instant::now()).unwrap();
        assert!(summary.contains("Files changed: 1"), "file count missing");
        // Edited files should appear without +new
        assert!(summary.contains("src/cli.rs"), "file name missing");
        assert!(
            !summary.contains("src/cli.rs +new"),
            "edit should not have +new"
        );
    }

    #[test]
    fn test_format_exit_summary_mixed() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/cli.rs", ChangeKind::Edit);
        changes.record("src/tools.rs", ChangeKind::Edit);
        let usage = make_usage(5000, 1500);
        let summary =
            format_exit_summary(&changes, &usage, "unknown-model", Instant::now()).unwrap();
        assert!(summary.contains("Files changed: 3"), "file count missing");
        assert!(
            summary.contains("src/main.rs +new"),
            "written file missing +new"
        );
        assert!(summary.contains("src/cli.rs"), "edited file missing");
        assert!(
            !summary.contains("src/cli.rs +new"),
            "edited file should not have +new"
        );
        assert!(summary.contains("src/tools.rs"), "edited file missing");
    }

    #[test]
    fn test_format_exit_summary_all_writes() {
        let changes = SessionChanges::new();
        changes.record("a.rs", ChangeKind::Write);
        changes.record("b.rs", ChangeKind::Write);
        let usage = make_usage(100, 50);
        let summary =
            format_exit_summary(&changes, &usage, "unknown-model", Instant::now()).unwrap();
        assert!(summary.contains("Files changed: 2"), "file count missing");
        assert!(summary.contains("a.rs +new"), "first file missing +new");
        assert!(summary.contains("b.rs +new"), "second file missing +new");
    }

    #[test]
    fn test_exit_summary_with_tokens_no_files() {
        // Pure Q&A session: tokens used but no file changes -- should still
        // produce a summary showing duration/tokens/cost.
        let changes = SessionChanges::new();
        let usage = make_usage(12_450, 3_200);
        let summary =
            format_exit_summary(&changes, &usage, "claude-sonnet-4-20250514", Instant::now())
                .unwrap();
        assert!(summary.contains("session summary"), "header missing");
        assert!(summary.contains("Duration:"), "duration missing");
        assert!(summary.contains("Tokens:"), "tokens missing");
        // Should NOT contain a Files line
        assert!(
            !summary.contains("Files changed:"),
            "should have no files line"
        );
        // Known model should produce a cost segment
        assert!(summary.contains("Cost:"), "cost missing for known model");
    }

    #[test]
    fn test_exit_summary_with_files_and_cost() {
        let changes = SessionChanges::new();
        changes.record("src/main.rs", ChangeKind::Write);
        changes.record("src/cli.rs", ChangeKind::Edit);
        let usage = make_usage(50_000, 10_000);
        let summary =
            format_exit_summary(&changes, &usage, "claude-sonnet-4-20250514", Instant::now())
                .unwrap();
        assert!(summary.contains("session summary"), "header missing");
        assert!(summary.contains("Duration:"), "duration missing");
        assert!(summary.contains("Tokens:"), "tokens missing");
        assert!(summary.contains("Cost:"), "cost missing");
        assert!(summary.contains("Files changed: 2"), "file count missing");
        assert!(summary.contains("src/main.rs +new"), "written file missing");
        assert!(summary.contains("src/cli.rs"), "edited file missing");
    }

    #[test]
    fn test_exit_summary_unknown_model_omits_cost() {
        let changes = SessionChanges::new();
        let usage = make_usage(1000, 500);
        let summary =
            format_exit_summary(&changes, &usage, "totally-unknown-model", Instant::now()).unwrap();
        assert!(summary.contains("Tokens:"));
        // Unknown model has no pricing -- cost should be absent
        assert!(!summary.contains("Cost:"));
    }

    #[test]
    fn test_exit_summary_compact_format() {
        // Verify the summary is compact: duration, tokens, and cost on one line
        // separated by " · "
        let changes = SessionChanges::new();
        let usage = make_usage(1000, 200);
        let summary =
            format_exit_summary(&changes, &usage, "claude-sonnet-4-20250514", Instant::now())
                .unwrap();
        // Duration and Tokens should be on the same line, joined by " · "
        let plain = strip_ansi(&summary);
        let stats_line = plain.lines().find(|l| l.contains("Duration:")).unwrap();
        assert!(
            stats_line.contains("Tokens:"),
            "Duration and Tokens should be on the same line, got: {stats_line}"
        );
    }

    #[test]
    fn test_format_file_list_sorted() {
        // File list should be sorted alphabetically
        let snapshot = vec![
            FileChange {
                path: "z.rs".to_string(),
                kind: ChangeKind::Edit,
            },
            FileChange {
                path: "a.rs".to_string(),
                kind: ChangeKind::Write,
            },
            FileChange {
                path: "m.rs".to_string(),
                kind: ChangeKind::Edit,
            },
        ];
        let list = format_file_list(&snapshot);
        assert_eq!(list, "a.rs +new, m.rs, z.rs");
    }

    #[test]
    fn test_format_file_list_truncation() {
        // A very long file list should be truncated with an ellipsis
        let snapshot: Vec<FileChange> = (0..20)
            .map(|i| FileChange {
                path: format!("src/very_long_module_name_{i}.rs"),
                kind: ChangeKind::Edit,
            })
            .collect();
        let list = format_file_list(&snapshot);
        assert!(list.len() <= 61, "list too long: {} chars", list.len());
        assert!(
            list.contains('\u{2026}'),
            "should contain ellipsis character"
        );
    }

    /// Strip ANSI escape sequences for test assertions.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::new();
        let mut in_escape = false;
        for ch in s.chars() {
            if ch == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if ch == 'm' {
                    in_escape = false;
                }
            } else {
                out.push(ch);
            }
        }
        out
    }

    #[test]
    fn test_changes_command_recognized() {
        use crate::commands::{is_unknown_command, KNOWN_COMMANDS};
        assert!(!is_unknown_command("/changes"));
        assert!(
            KNOWN_COMMANDS.contains(&"/changes"),
            "/changes should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_changes_command_not_confused_with_other_commands() {
        use crate::commands::is_unknown_command;
        // /changes should match exactly, unrelated words should be unknown
        assert!(is_unknown_command("/changed"));
        // /changelog is now a valid command (Issue #226)
        assert!(!is_unknown_command("/changelog"));
    }
}
