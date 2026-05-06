//! Run and loop command handlers: /run, /loop.

use crate::agent_builder::AgentConfig;
use crate::commands::auto_compact_if_needed;
use crate::format::*;
use crate::prompt::run_prompt_auto_retry;
use crate::session::SessionChanges;

use yoagent::agent::Agent;
use yoagent::*;

/// Run a shell command directly and print its output.
pub fn run_shell_command(cmd: &str) {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    let start = std::time::Instant::now();
    let child = Command::new("sh")
        .args(["-c", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{RED}  error running command: {e}{RESET}\n");
            return;
        }
    };

    // Read stderr in a background thread so we don't block on either pipe
    let stderr_pipe = child.stderr.take().expect("stderr was piped");
    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        for line in reader.lines() {
            match line {
                Ok(l) => eprintln!("{RED}{l}{RESET}"),
                Err(_) => break,
            }
        }
    });

    // Stream stdout line-by-line on the main thread
    if let Some(stdout_pipe) = child.stdout.take() {
        let reader = BufReader::new(stdout_pipe);
        for line in reader.lines() {
            match line {
                Ok(l) => println!("{l}"),
                Err(_) => break,
            }
        }
    }

    // Wait for stderr thread to finish
    let _ = stderr_handle.join();

    // Collect exit status
    let elapsed = format_duration(start.elapsed());
    match child.wait() {
        Ok(status) => {
            let code = status.code().unwrap_or(-1);
            if code == 0 {
                println!("{DIM}  ✓ exit {code} ({elapsed}){RESET}\n");
            } else {
                println!("{RED}  ✗ exit {code} ({elapsed}){RESET}\n");
            }
        }
        Err(e) => {
            eprintln!("{RED}  error waiting for command: {e}{RESET}\n");
        }
    }
}

pub fn handle_run(input: &str) {
    let cmd = if input.starts_with("/run ") {
        input.trim_start_matches("/run ").trim()
    } else if input.starts_with('!') && input.len() > 1 {
        input[1..].trim()
    } else {
        ""
    };
    if cmd.is_empty() {
        println!("{DIM}  usage: /run <command>  or  !<command>{RESET}\n");
    } else {
        run_shell_command(cmd);
    }
}

pub fn handle_run_usage() {
    println!("{DIM}  usage: /run <command>  or  !<command>");
    println!("  Runs a shell command directly (no AI, no tokens).{RESET}\n");
}

/// How many times to iterate in a `/loop` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopMode {
    /// Run exactly N times (1..=100).
    Count(usize),
    /// Run until the last tool call succeeds (max 20 iterations).
    UntilPass,
}

const MAX_UNTIL_PASS: usize = 20;
const MAX_LOOP_COUNT: usize = 100;

/// Parse `/loop <N|until-pass> <prompt>`.
///
/// Returns `None` if the input is malformed (missing args, zero count, etc.).
/// Counts above [`MAX_LOOP_COUNT`] are clamped silently.
pub fn parse_loop_args(input: &str) -> Option<(LoopMode, String)> {
    let rest = input.strip_prefix("/loop").unwrap_or(input).trim_start();
    if rest.is_empty() {
        return None;
    }

    // Split into mode token and the remaining prompt.
    let (mode_tok, prompt) = match rest.split_once(char::is_whitespace) {
        Some((m, p)) => (m, p.trim()),
        None => return None, // e.g. "/loop 5" with no prompt
    };

    if prompt.is_empty() {
        return None;
    }

    let mode = if mode_tok == "until-pass" {
        LoopMode::UntilPass
    } else if let Ok(n) = mode_tok.parse::<usize>() {
        if n == 0 {
            return None;
        }
        LoopMode::Count(n.min(MAX_LOOP_COUNT))
    } else {
        return None;
    };

    Some((mode, prompt.to_string()))
}

/// Run a prompt in a polling loop.
pub async fn handle_loop(
    input: &str,
    agent: &mut Agent,
    session_total: &mut Usage,
    agent_config: &AgentConfig,
    changes: &SessionChanges,
) {
    let (mode, prompt) = match parse_loop_args(input) {
        Some(v) => v,
        None => {
            println!(
                "{DIM}Usage: /loop <N|until-pass> <prompt>\n\
                 \n  /loop 5 run the tests and fix any failures\
                 \n  /loop until-pass run cargo test{RESET}"
            );
            return;
        }
    };

    let max_iters = match &mode {
        LoopMode::Count(n) => *n,
        LoopMode::UntilPass => MAX_UNTIL_PASS,
    };

    for i in 1..=max_iters {
        // Print iteration header.
        let label = match &mode {
            LoopMode::Count(n) => format!("--- loop iteration {i}/{n} ---"),
            LoopMode::UntilPass => {
                format!("--- loop iteration {i} (until-pass, max {MAX_UNTIL_PASS}) ---")
            }
        };
        println!("\n{BOLD}{CYAN}{label}{RESET}\n");

        let outcome =
            run_prompt_auto_retry(agent, &prompt, session_total, &agent_config.model, changes)
                .await;

        auto_compact_if_needed(agent);

        // For until-pass mode: stop when the last tool call succeeded (no error).
        if mode == LoopMode::UntilPass && outcome.last_tool_error.is_none() {
            println!(
                "\n{GREEN}{BOLD}✓ Loop complete — last tool call succeeded on iteration {i}.{RESET}"
            );
            return;
        }

        // Don't sleep after the last iteration.
        if i < max_iters {
            // Brief pause so the user can Ctrl+C between iterations.
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    // Finished all iterations.
    match &mode {
        LoopMode::Count(n) => {
            println!("\n{DIM}Loop complete — {n} iterations finished.{RESET}");
        }
        LoopMode::UntilPass => {
            println!(
                "\n{YELLOW}Loop exhausted {MAX_UNTIL_PASS} iterations without a passing tool call.{RESET}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_shell_command_basic() {
        // Verify run_shell_command doesn't panic on basic commands
        // (output streams to stdout/stderr line-by-line)
        run_shell_command("echo hello");
    }

    #[test]
    fn test_run_shell_command_failing() {
        // Non-zero exit should not panic
        run_shell_command("false");
    }

    #[test]
    fn test_run_shell_command_streams_multiline() {
        // Multi-line output should stream without panic
        run_shell_command("echo line1; echo line2; echo line3");
    }

    #[test]
    fn test_run_shell_command_mixed_stdout_stderr() {
        // Both stdout and stderr should be handled without deadlock or panic
        run_shell_command("echo out; echo err >&2; echo out2");
    }

    #[test]
    fn test_run_shell_command_large_output() {
        // Ensure streaming handles larger output without buffering issues
        run_shell_command("seq 1 100");
    }

    #[test]
    fn test_bang_shortcut_matching() {
        // ! prefix should match for /run shortcut
        let bang_matches = |s: &str| s.starts_with('!') && s.len() > 1;
        assert!(bang_matches("!ls"));
        assert!(bang_matches("!echo hello"));
        assert!(bang_matches("! ls")); // space after bang is fine
        assert!(!bang_matches("!")); // bare bang alone should not match
    }

    #[test]
    fn test_run_command_matching() {
        // /run should only match /run or /run <cmd>, not /running
        let run_matches = |s: &str| s == "/run" || s.starts_with("/run ");
        assert!(run_matches("/run"));
        assert!(run_matches("/run echo hello"));
        assert!(!run_matches("/running"));
        assert!(!run_matches("/runaway"));
    }

    #[test]
    fn parse_loop_count_with_prompt() {
        let result = parse_loop_args("/loop 5 fix the tests");
        assert_eq!(
            result,
            Some((LoopMode::Count(5), "fix the tests".to_string()))
        );
    }

    #[test]
    fn parse_loop_until_pass() {
        let result = parse_loop_args("/loop until-pass cargo test");
        assert_eq!(
            result,
            Some((LoopMode::UntilPass, "cargo test".to_string()))
        );
    }

    #[test]
    fn parse_loop_missing_args() {
        assert_eq!(parse_loop_args("/loop"), None);
    }

    #[test]
    fn parse_loop_missing_prompt() {
        assert_eq!(parse_loop_args("/loop 5"), None);
    }

    #[test]
    fn parse_loop_zero_not_valid() {
        assert_eq!(parse_loop_args("/loop 0 something"), None);
    }

    #[test]
    fn parse_loop_capped_at_100() {
        let result = parse_loop_args("/loop 200 something");
        assert_eq!(
            result,
            Some((LoopMode::Count(100), "something".to_string()))
        );
    }

    #[test]
    fn parse_loop_invalid_mode() {
        assert_eq!(parse_loop_args("/loop abc do stuff"), None);
    }

    #[test]
    fn parse_loop_one_iteration() {
        let result = parse_loop_args("/loop 1 check it");
        assert_eq!(result, Some((LoopMode::Count(1), "check it".to_string())));
    }

    #[test]
    fn parse_loop_prompt_preserves_spaces() {
        let result = parse_loop_args("/loop 3 check if the server is responding");
        assert_eq!(
            result,
            Some((
                LoopMode::Count(3),
                "check if the server is responding".to_string()
            ))
        );
    }
}
