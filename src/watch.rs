//! Watch mode — auto-run a test/lint command after agent edits.
//!
//! Extracted from `prompt.rs` (Day 58). The watch system lets users set a
//! command (e.g. `cargo test`) that runs automatically after each agent turn.
//! If the command fails, the agent gets a fix prompt and retries up to
//! [`MAX_WATCH_FIX_ATTEMPTS`] times.

use crate::format::*;
use crate::prompt::run_prompt_auto_retry;
use crate::prompt_budget::session_budget_exhausted;
use crate::session::SessionChanges;
use std::io::{self, IsTerminal, Write};
use std::sync::RwLock;
use yoagent::agent::Agent;
use yoagent::*;

/// Acquire a read-guard, recovering from a poisoned RwLock instead of panicking.
fn rw_read_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

/// Acquire a write-guard, recovering from a poisoned RwLock instead of panicking.
fn rw_write_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}

// Global state for `/watch` — auto-run a test command after agent edits.

/// The currently active watch commands (empty = watch mode off).
/// When multiple commands are stored, each is run as its own phase with
/// its own fix loop (e.g. lint → fix lint → test → fix test).
static WATCH_COMMANDS: RwLock<Vec<String>> = RwLock::new(Vec::new());

/// Set a single watch command, enabling watch mode.
/// This is the backward-compatible API — stores a single-element vec internally.
pub fn set_watch_command(cmd: &str) {
    let mut guard = rw_write_or_recover(&WATCH_COMMANDS);
    *guard = vec![cmd.to_string()];
}

/// Set multiple watch commands for multi-phase execution.
/// Each command runs as its own phase with its own fix loop.
/// For example: `["cargo clippy ...", "cargo test"]` runs lint first,
/// fixes lint errors, then runs tests, fixes test errors.
pub fn set_watch_commands(cmds: &[&str]) {
    let mut guard = rw_write_or_recover(&WATCH_COMMANDS);
    *guard = cmds.iter().map(|s| s.to_string()).collect();
}

/// Get the current watch command for display purposes.
/// If multiple commands are stored, returns them joined with ` && `.
/// Returns None if watch mode is off.
pub fn get_watch_command() -> Option<String> {
    let guard = rw_read_or_recover(&WATCH_COMMANDS);
    if guard.is_empty() {
        None
    } else {
        Some(guard.join(" && "))
    }
}

/// Get the individual watch commands (phases).
/// Returns an empty vec if watch mode is off.
pub fn get_watch_commands() -> Vec<String> {
    let guard = rw_read_or_recover(&WATCH_COMMANDS);
    guard.clone()
}

/// Clear the watch command, disabling watch mode.
pub fn clear_watch_command() {
    let mut guard = rw_write_or_recover(&WATCH_COMMANDS);
    *guard = Vec::new();
}

/// Maximum characters of watch command output to include in fix prompts.
const WATCH_OUTPUT_MAX: usize = 5000;

/// Maximum number of auto-fix attempts when watch mode detects failures.
pub const MAX_WATCH_FIX_ATTEMPTS: usize = 3;

/// Result from [`run_watch_after_prompt`] — carries pass/fail status plus
/// the last tool error from any auto-fix attempts (if the watch failed).
#[derive(Debug, Clone)]
pub struct WatchResult {
    /// Whether the watch command ultimately passed.
    pub passed: bool,
    /// The last tool error from auto-fix attempts, if any.
    pub last_tool_error: Option<String>,
}

/// Classify a watch command as "lint", "test", or "command" for fix prompt hints.
fn classify_watch_command(cmd: &str) -> &'static str {
    let lower = cmd.to_lowercase();
    // Check for lint-like commands
    if lower.contains("clippy")
        || lower.contains("eslint")
        || lower.contains("pylint")
        || lower.contains("flake8")
        || lower.contains("ruff")
        || lower.contains("golint")
        || lower.contains("lint")
    {
        "lint"
    // Check for test-like commands
    } else if lower.contains("test")
        || lower.contains("pytest")
        || lower.contains("jest")
        || lower.contains("vitest")
        || lower.contains("mocha")
    {
        "test"
    } else {
        "command"
    }
}

/// Build a prompt asking the agent to fix failures from a watch command.
///
/// Includes a hint about the command type (lint, test, or general command)
/// so the agent can choose an appropriate fix strategy. Lint failures are
/// usually mechanical (unused imports, formatting), while test failures
/// require understanding the intended behavior.
pub fn build_watch_fix_prompt(watch_cmd: &str, output: &str) -> String {
    let truncated = if output.len() > WATCH_OUTPUT_MAX {
        format!("{}... (truncated)", safe_truncate(output, WATCH_OUTPUT_MAX))
    } else {
        output.to_string()
    };
    let cmd_type = classify_watch_command(watch_cmd);
    let hint = match cmd_type {
        "lint" => "\n\nThis is a **lint** failure — fixes are usually mechanical (unused imports, \
                   missing derives, formatting issues). Apply targeted fixes without changing logic.",
        "test" => "\n\nThis is a **test** failure — understand what the test expects before \
                   changing code. Fix the implementation to match the intended behavior, \
                   or fix the test if the new behavior is correct.",
        _ => "",
    };
    format!(
        "Your changes caused {cmd_type} failures. Here's the output from `{watch_cmd}`:\n\
         ```\n{truncated}\n```\n\
         Please fix the issues.{hint}"
    )
}

/// Run a watch command and return (success, output).
///
/// Streams output line-by-line in real time: when stderr is a terminal,
/// prints a compact progress indicator (`⟳ 42 lines...`) so the user
/// sees something happening during long test/build runs.  The full
/// combined stdout+stderr is still collected and returned for the agent
/// to analyse.
pub fn run_watch_command(cmd: &str) -> (bool, String) {
    use std::io::BufRead;
    use std::process::{Command, Stdio};

    let is_tty = io::stderr().is_terminal();

    let child = Command::new("sh")
        .args(["-c", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => return (false, format!("Failed to run watch command: {e}")),
    };

    // Collect stderr lines in a background thread.
    let stderr_pipe = child.stderr.take().expect("stderr was piped");
    let stderr_handle = std::thread::spawn(move || {
        let reader = io::BufReader::new(stderr_pipe);
        let mut lines = Vec::new();
        for line in reader.lines() {
            match line {
                Ok(l) => lines.push(l),
                Err(_) => break,
            }
        }
        lines
    });

    // Stream stdout on the main thread, collecting lines.
    let mut stdout_lines: Vec<String> = Vec::new();
    if let Some(stdout_pipe) = child.stdout.take() {
        let reader = io::BufReader::new(stdout_pipe);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    stdout_lines.push(l);
                    if is_tty {
                        let count = stdout_lines.len();
                        eprint!("\r{DIM}  ⟳ {count} lines...{RESET}");
                        let _ = io::stderr().flush();
                    }
                }
                Err(_) => break,
            }
        }
    }

    let stderr_lines = stderr_handle.join().unwrap_or_default();

    // Clear the progress indicator if we printed one.
    if is_tty && !stdout_lines.is_empty() {
        eprint!("\r{DIM}                          {RESET}\r");
        let _ = io::stderr().flush();
    }

    let status = match child.wait() {
        Ok(s) => s.success(),
        Err(_) => false,
    };

    // Combine stdout + stderr the same way the old implementation did.
    let stdout_text = stdout_lines.join("\n");
    let stderr_text = stderr_lines.join("\n");
    let combined = if stderr_text.is_empty() {
        stdout_text
    } else if stdout_text.is_empty() {
        stderr_text
    } else {
        format!("{stdout_text}\n{stderr_text}")
    };

    (status, combined)
}

/// Run the watch command(s) after a prompt completes.
///
/// If watch commands are active, iterates through each phase in order.
/// For each phase: runs the command, and if it fails, enters the fix loop
/// (up to [`MAX_WATCH_FIX_ATTEMPTS`] times). Only proceeds to the next
/// phase if the current one passes. This means lint gets fixed before tests
/// even run.
///
/// Returns a [`WatchResult`] with pass/fail status and the last tool error
/// from any fix attempts. If no watch command is set, returns
/// `WatchResult { passed: true, last_tool_error: None }`.
pub async fn run_watch_after_prompt(
    agent: &mut Agent,
    session_total: &mut Usage,
    model: &str,
    changes: &SessionChanges,
) -> WatchResult {
    let commands = get_watch_commands();
    if commands.is_empty() {
        return WatchResult {
            passed: true,
            last_tool_error: None,
        };
    }

    let total_phases = commands.len();
    let mut last_tool_error: Option<String> = None;

    for (phase_idx, watch_cmd) in commands.iter().enumerate() {
        let phase_num = phase_idx + 1;
        let phase_label = if total_phases > 1 {
            format!(" (phase {phase_num}/{total_phases})")
        } else {
            String::new()
        };

        let (ok, output) = run_watch_command(watch_cmd);
        if ok {
            eprintln!("{GREEN}  ✓ Watch passed{phase_label}: `{watch_cmd}`{RESET}");
            continue;
        }

        eprintln!("{RED}  ✗ Watch failed{phase_label}: `{watch_cmd}`{RESET}");
        let display_output = if output.len() > 2000 {
            format!("{}...\n(truncated)", safe_truncate(&output, 2000))
        } else {
            output.clone()
        };
        eprintln!("{DIM}{display_output}{RESET}");

        // Multi-attempt auto-fix loop for this phase
        let mut current_output = output;
        let mut phase_passed = false;
        for attempt in 1..=MAX_WATCH_FIX_ATTEMPTS {
            if session_budget_exhausted(30) {
                eprintln!(
                    "{DIM}  ⏱ session budget nearly exhausted, stopping watch fix loop early{RESET}"
                );
                return WatchResult {
                    passed: false,
                    last_tool_error,
                };
            }
            eprintln!("{YELLOW}  → Auto-fixing{phase_label} (attempt {attempt}/{MAX_WATCH_FIX_ATTEMPTS})...{RESET}");

            let fix_prompt = build_watch_fix_prompt(watch_cmd, &current_output);
            let fix_outcome =
                run_prompt_auto_retry(agent, &fix_prompt, session_total, model, changes).await;
            last_tool_error = fix_outcome.last_tool_error.clone();

            // Re-run this phase's command to see if fix worked
            let (fix_ok, fix_output) = run_watch_command(watch_cmd);
            if fix_ok {
                eprintln!(
                    "{GREEN}  ✓ Watch passed{phase_label} after fix (attempt {attempt}){RESET}"
                );
                phase_passed = true;
                break;
            } else if attempt == MAX_WATCH_FIX_ATTEMPTS {
                eprintln!(
                    "{RED}  ✗ Watch still failing{phase_label} after {MAX_WATCH_FIX_ATTEMPTS} attempts — manual fix needed{RESET}"
                );
            } else {
                eprintln!("{RED}  ✗ Attempt {attempt} failed{phase_label}, retrying...{RESET}");
                current_output = fix_output;
            }
        }

        if !phase_passed {
            // Stop: don't proceed to later phases if this one can't be fixed
            return WatchResult {
                passed: false,
                last_tool_error,
            };
        }
    }

    WatchResult {
        passed: true,
        last_tool_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_build_watch_fix_prompt() {
        let prompt = build_watch_fix_prompt("cargo test", "error[E0308]: mismatched types");
        assert!(
            prompt.contains("cargo test"),
            "prompt should include the command name"
        );
        assert!(
            prompt.contains("error[E0308]: mismatched types"),
            "prompt should include the output"
        );
        assert!(prompt.contains("Please fix"), "prompt should ask for a fix");
        assert!(
            prompt.contains("```"),
            "prompt should wrap output in code fence"
        );
    }

    #[test]
    fn test_max_watch_fix_attempts_constant() {
        // The constant should exist and be a reasonable retry count (1..=10)
        let attempts = MAX_WATCH_FIX_ATTEMPTS;
        assert!(attempts >= 1, "should allow at least 1 attempt");
        assert!(attempts <= 10, "should not retry excessively");
        assert_eq!(attempts, 3, "default should be 3 attempts");
    }

    #[test]
    fn test_build_watch_fix_prompt_truncates_long_output() {
        let long_output = "x".repeat(6000);
        let prompt = build_watch_fix_prompt("cargo test", &long_output);
        assert!(
            prompt.contains("... (truncated)"),
            "long output should be truncated"
        );
        // The output in the prompt should not contain the full 6000 chars
        assert!(
            !prompt.contains(&"x".repeat(6000)),
            "full output should not appear"
        );
        // But should contain the first 5000
        assert!(
            prompt.contains(&"x".repeat(5000)),
            "first 5000 chars should appear"
        );
    }

    #[test]
    fn test_run_watch_command_success() {
        let (ok, output) = run_watch_command("echo hello");
        assert!(ok, "echo should succeed");
        assert_eq!(output.trim(), "hello");
    }

    #[test]
    fn test_run_watch_command_failure() {
        let (ok, _output) = run_watch_command("exit 1");
        assert!(!ok, "exit 1 should fail");
    }

    #[test]
    fn test_run_watch_command_captures_all_output() {
        let (ok, output) = run_watch_command("for i in 1 2 3 4 5; do echo line$i; done");
        assert!(ok);
        assert!(output.contains("line1"));
        assert!(output.contains("line5"));
        // Should have all 5 lines
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 5, "should capture all 5 lines");
    }

    #[test]
    fn test_run_watch_command_captures_stderr() {
        let (ok, output) = run_watch_command("echo err_msg >&2");
        assert!(ok, "writing to stderr is not a failure");
        assert!(
            output.contains("err_msg"),
            "stderr should be captured: {output}"
        );
    }

    #[test]
    fn test_run_watch_command_combines_stdout_stderr() {
        let (ok, output) = run_watch_command("echo out_msg; echo err_msg >&2");
        assert!(ok);
        assert!(output.contains("out_msg"), "should contain stdout");
        assert!(output.contains("err_msg"), "should contain stderr");
    }

    #[test]
    fn test_run_watch_command_invalid_command() {
        let (ok, output) = run_watch_command("nonexistent_command_xyz_123");
        assert!(!ok, "nonexistent command should fail");
        assert!(
            !output.is_empty(),
            "should have some error output: {output}"
        );
    }

    #[serial]
    #[test]
    fn test_watch_command_none_by_default() {
        // After clearing, there should be no watch command
        clear_watch_command();
        assert!(
            get_watch_command().is_none(),
            "should have no watch command after clear"
        );
    }

    #[serial]
    #[test]
    fn test_watch_command_roundtrip() {
        // Set a command, get it back, clear it
        set_watch_command("cargo test --release");
        let cmd = get_watch_command();
        assert_eq!(cmd.as_deref(), Some("cargo test --release"));
        clear_watch_command();
        assert!(get_watch_command().is_none());
    }

    #[serial]
    #[test]
    fn test_run_watch_after_prompt_no_watch_returns_passed() {
        // When no watch command is set, run_watch_after_prompt should return
        // WatchResult { passed: true, last_tool_error: None } immediately.
        // We verify the guard condition that makes it return early.
        clear_watch_command();
        assert!(
            get_watch_command().is_none(),
            "precondition: no watch command set"
        );
        // The function checks get_watch_command() first and returns a passing
        // WatchResult if None. We can't call the async function in a sync test,
        // but we verify the guard condition that makes it return early.
    }

    #[serial]
    #[test]
    fn test_run_watch_command_pass_with_set_watch() {
        // Simulate: set a watch command that passes, run it
        set_watch_command("echo ok");
        if let Some(cmd) = get_watch_command() {
            let (ok, output) = run_watch_command(&cmd);
            assert!(ok, "echo ok should succeed");
            assert!(output.contains("ok"));
        } else {
            panic!("watch command should be set");
        }
        clear_watch_command();
    }

    #[serial]
    #[test]
    fn test_run_watch_command_fail_with_set_watch() {
        // Simulate: set a watch command that fails, run it, check output
        set_watch_command("sh -c 'echo FAIL; exit 1'");
        if let Some(cmd) = get_watch_command() {
            let (ok, output) = run_watch_command(&cmd);
            assert!(!ok, "command should fail");
            assert!(output.contains("FAIL"), "output should contain FAIL");
            // Verify build_watch_fix_prompt works with the output
            let fix_prompt = build_watch_fix_prompt(&cmd, &output);
            assert!(fix_prompt.contains("FAIL"));
            assert!(fix_prompt.contains("Please fix"));
        } else {
            panic!("watch command should be set");
        }
        clear_watch_command();
    }

    #[test]
    fn test_watch_result_passed() {
        let result = WatchResult {
            passed: true,
            last_tool_error: None,
        };
        assert!(result.passed);
        assert!(result.last_tool_error.is_none());
    }

    #[test]
    fn test_watch_result_failed_with_error() {
        let result = WatchResult {
            passed: false,
            last_tool_error: Some("compilation error".to_string()),
        };
        assert!(!result.passed);
        assert_eq!(result.last_tool_error.as_deref(), Some("compilation error"));
    }

    #[test]
    fn test_watch_result_clone() {
        let result = WatchResult {
            passed: false,
            last_tool_error: Some("test failure".to_string()),
        };
        let cloned = result.clone();
        assert_eq!(cloned.passed, result.passed);
        assert_eq!(cloned.last_tool_error, result.last_tool_error);
    }

    #[test]
    fn test_watch_result_debug() {
        let result = WatchResult {
            passed: true,
            last_tool_error: None,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("passed: true"));
        assert!(debug.contains("last_tool_error: None"));
    }

    // --- Multi-phase watch tests ---

    #[serial]
    #[test]
    fn test_set_get_watch_commands_roundtrip() {
        set_watch_commands(&["cargo clippy", "cargo test"]);
        let cmds = get_watch_commands();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0], "cargo clippy");
        assert_eq!(cmds[1], "cargo test");
        clear_watch_command();
        assert!(get_watch_commands().is_empty());
    }

    #[serial]
    #[test]
    fn test_get_watch_command_joins_multi_phase() {
        set_watch_commands(&["cargo clippy", "cargo test"]);
        let display = get_watch_command();
        assert_eq!(
            display.as_deref(),
            Some("cargo clippy && cargo test"),
            "get_watch_command should join phases with &&"
        );
        clear_watch_command();
    }

    #[serial]
    #[test]
    fn test_single_command_still_works() {
        set_watch_command("cargo test");
        let cmds = get_watch_commands();
        assert_eq!(cmds.len(), 1, "single command should store one-element vec");
        assert_eq!(cmds[0], "cargo test");
        let display = get_watch_command();
        assert_eq!(display.as_deref(), Some("cargo test"));
        clear_watch_command();
    }

    #[serial]
    #[test]
    fn test_clear_clears_multi_phase() {
        set_watch_commands(&["a", "b", "c"]);
        assert_eq!(get_watch_commands().len(), 3);
        clear_watch_command();
        assert!(get_watch_commands().is_empty());
        assert!(get_watch_command().is_none());
    }

    #[test]
    fn test_classify_watch_command_lint() {
        assert_eq!(classify_watch_command("cargo clippy"), "lint");
        assert_eq!(
            classify_watch_command("cargo clippy --all-targets -- -D warnings"),
            "lint"
        );
        assert_eq!(classify_watch_command("npx eslint ."), "lint");
        assert_eq!(classify_watch_command("ruff check ."), "lint");
        assert_eq!(classify_watch_command("npm run lint"), "lint");
    }

    #[test]
    fn test_classify_watch_command_test() {
        assert_eq!(classify_watch_command("cargo test"), "test");
        assert_eq!(classify_watch_command("npm test"), "test");
        assert_eq!(classify_watch_command("python -m pytest"), "test");
        assert_eq!(classify_watch_command("npx jest"), "test");
        assert_eq!(classify_watch_command("npx vitest"), "test");
    }

    #[test]
    fn test_classify_watch_command_general() {
        assert_eq!(classify_watch_command("cargo build"), "command");
        assert_eq!(classify_watch_command("make"), "command");
        assert_eq!(classify_watch_command("echo hello"), "command");
    }

    #[test]
    fn test_fix_prompt_includes_lint_hint() {
        let prompt = build_watch_fix_prompt("cargo clippy --all-targets", "warning: unused import");
        assert!(
            prompt.contains("lint"),
            "lint command prompt should mention lint: {prompt}"
        );
        assert!(
            prompt.contains("mechanical"),
            "lint prompt should mention mechanical fixes: {prompt}"
        );
    }

    #[test]
    fn test_fix_prompt_includes_test_hint() {
        let prompt = build_watch_fix_prompt("cargo test", "test result: FAILED");
        assert!(
            prompt.contains("test"),
            "test command prompt should mention test: {prompt}"
        );
        assert!(
            prompt.contains("intended behavior"),
            "test prompt should mention understanding behavior: {prompt}"
        );
    }

    #[test]
    fn test_fix_prompt_general_command_no_extra_hint() {
        let prompt = build_watch_fix_prompt("cargo build", "error: linking failed");
        assert!(
            prompt.contains("command failures"),
            "general command should say 'command failures': {prompt}"
        );
        // Should NOT contain the lint or test specific hints
        assert!(
            !prompt.contains("mechanical"),
            "general command should not have lint hint"
        );
        assert!(
            !prompt.contains("intended behavior"),
            "general command should not have test hint"
        );
    }

    #[serial]
    #[test]
    fn test_run_watch_after_prompt_empty_commands_returns_passed() {
        // When no watch commands are set, should return passed immediately
        clear_watch_command();
        assert!(
            get_watch_commands().is_empty(),
            "precondition: no commands set"
        );
        // The function checks get_watch_commands() first and returns a passing
        // WatchResult if empty. We verify the guard condition.
    }
}
