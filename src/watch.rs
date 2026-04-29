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

/// The currently active watch command (None = watch mode off).
static WATCH_COMMAND: RwLock<Option<String>> = RwLock::new(None);

/// Set the watch command, enabling watch mode.
pub fn set_watch_command(cmd: &str) {
    let mut guard = rw_write_or_recover(&WATCH_COMMAND);
    *guard = Some(cmd.to_string());
}

/// Get the current watch command, if watch mode is active.
pub fn get_watch_command() -> Option<String> {
    let guard = rw_read_or_recover(&WATCH_COMMAND);
    guard.clone()
}

/// Clear the watch command, disabling watch mode.
pub fn clear_watch_command() {
    let mut guard = rw_write_or_recover(&WATCH_COMMAND);
    *guard = None;
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

/// Build a prompt asking the agent to fix failures from a watch command.
pub fn build_watch_fix_prompt(watch_cmd: &str, output: &str) -> String {
    let truncated = if output.len() > WATCH_OUTPUT_MAX {
        format!("{}... (truncated)", safe_truncate(output, WATCH_OUTPUT_MAX))
    } else {
        output.to_string()
    };
    format!(
        "Your changes caused test/lint failures. Here's the output from `{watch_cmd}`:\n\
         ```\n{truncated}\n```\n\
         Please fix the issues."
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

/// Run the watch command after a prompt completes.
///
/// If a watch command is active, this runs the watch command and auto-fixes
/// failures up to [`MAX_WATCH_FIX_ATTEMPTS`] times. Used by the REPL main
/// loop, `/side` handler, single-prompt mode, and piped mode.
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
    let watch_cmd = match get_watch_command() {
        Some(cmd) => cmd,
        None => {
            return WatchResult {
                passed: true,
                last_tool_error: None,
            }
        }
    };

    let (ok, output) = run_watch_command(&watch_cmd);
    if ok {
        eprintln!("{GREEN}  ✓ Watch passed: `{watch_cmd}`{RESET}");
        return WatchResult {
            passed: true,
            last_tool_error: None,
        };
    }

    eprintln!("{RED}  ✗ Watch failed: `{watch_cmd}`{RESET}");
    let display_output = if output.len() > 2000 {
        format!("{}...\n(truncated)", safe_truncate(&output, 2000))
    } else {
        output.clone()
    };
    eprintln!("{DIM}{display_output}{RESET}");

    // Multi-attempt auto-fix loop
    let mut current_output = output;
    let mut last_tool_error: Option<String> = None;
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
        eprintln!("{YELLOW}  → Auto-fixing (attempt {attempt}/{MAX_WATCH_FIX_ATTEMPTS})...{RESET}");

        let fix_prompt = build_watch_fix_prompt(&watch_cmd, &current_output);
        let fix_outcome =
            run_prompt_auto_retry(agent, &fix_prompt, session_total, model, changes).await;
        last_tool_error = fix_outcome.last_tool_error.clone();

        // Re-run watch command to see if fix worked
        let (fix_ok, fix_output) = run_watch_command(&watch_cmd);
        if fix_ok {
            eprintln!("{GREEN}  ✓ Watch passed after fix (attempt {attempt}){RESET}");
            return WatchResult {
                passed: true,
                last_tool_error,
            };
        } else if attempt == MAX_WATCH_FIX_ATTEMPTS {
            eprintln!(
                "{RED}  ✗ Watch still failing after {MAX_WATCH_FIX_ATTEMPTS} attempts — manual fix needed{RESET}"
            );
        } else {
            eprintln!("{RED}  ✗ Attempt {attempt} failed, retrying...{RESET}");
            current_output = fix_output;
        }
    }

    WatchResult {
        passed: false,
        last_tool_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_watch_command_none_by_default() {
        // After clearing, there should be no watch command
        clear_watch_command();
        assert!(
            get_watch_command().is_none(),
            "should have no watch command after clear"
        );
    }

    #[test]
    fn test_watch_command_roundtrip() {
        // Set a command, get it back, clear it
        set_watch_command("cargo test --release");
        let cmd = get_watch_command();
        assert_eq!(cmd.as_deref(), Some("cargo test --release"));
        clear_watch_command();
        assert!(get_watch_command().is_none());
    }

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
}
