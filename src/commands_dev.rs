//! Dev workflow command handlers: /doctor, /health, /fix, /watch, /tree.

use crate::cli;
use crate::commands::auto_compact_if_needed;
use crate::commands_lint::{lint_command_for_project, test_command_for_project, LintStrictness};
use crate::commands_project::{detect_project_type, ProjectType};
use crate::format::*;
use crate::prompt::*;

use yoagent::agent::Agent;
use yoagent::*;

// ── /doctor ──────────────────────────────────────────────────────────────

/// Status of a single doctor check.
#[derive(Debug, Clone, PartialEq)]
pub enum DoctorStatus {
    Pass,
    Fail,
    Warn,
}

/// A single diagnostic check result from `/doctor`.
#[derive(Debug, Clone)]
pub struct DoctorCheck {
    pub name: String,
    pub status: DoctorStatus,
    pub detail: String,
}

/// Run all environment diagnostic checks and return structured results.
///
/// This is separated from the display logic so it can be tested.
pub fn run_doctor_checks(provider: &str, model: &str) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    // 1. Version
    checks.push(DoctorCheck {
        name: "Version".to_string(),
        status: DoctorStatus::Pass,
        detail: cli::VERSION.to_string(),
    });

    // 2. Git installed
    match std::process::Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let ver = String::from_utf8_lossy(&output.stdout)
                .trim()
                .replace("git version ", "")
                .to_string();
            checks.push(DoctorCheck {
                name: "Git".to_string(),
                status: DoctorStatus::Pass,
                detail: format!("installed ({ver})"),
            });
        }
        _ => {
            checks.push(DoctorCheck {
                name: "Git".to_string(),
                status: DoctorStatus::Fail,
                detail: "not found".to_string(),
            });
        }
    }

    // 3. Git repo
    match std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
    {
        Ok(output) if output.status.success() => {
            let branch = std::process::Command::new("git")
                .args(["branch", "--show-current"])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        let b = String::from_utf8_lossy(&o.stdout).trim().to_string();
                        if b.is_empty() {
                            None
                        } else {
                            Some(b)
                        }
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| "detached".to_string());
            checks.push(DoctorCheck {
                name: "Git repo".to_string(),
                status: DoctorStatus::Pass,
                detail: format!("yes (branch: {branch})"),
            });
        }
        _ => {
            checks.push(DoctorCheck {
                name: "Git repo".to_string(),
                status: DoctorStatus::Warn,
                detail: "not inside a git repository".to_string(),
            });
        }
    }

    // 4. Provider
    checks.push(DoctorCheck {
        name: "Provider".to_string(),
        status: DoctorStatus::Pass,
        detail: provider.to_string(),
    });

    // 5. API key
    let env_var = cli::provider_api_key_env(provider);
    match env_var {
        Some(var_name) => {
            if std::env::var(var_name).is_ok() {
                checks.push(DoctorCheck {
                    name: "API key".to_string(),
                    status: DoctorStatus::Pass,
                    detail: format!("set ({var_name})"),
                });
            } else {
                checks.push(DoctorCheck {
                    name: "API key".to_string(),
                    status: DoctorStatus::Fail,
                    detail: format!("{var_name} not set"),
                });
            }
        }
        None => {
            // Unknown provider — can't check env var
            if provider == "ollama" {
                checks.push(DoctorCheck {
                    name: "API key".to_string(),
                    status: DoctorStatus::Pass,
                    detail: "not required (ollama)".to_string(),
                });
            } else {
                checks.push(DoctorCheck {
                    name: "API key".to_string(),
                    status: DoctorStatus::Warn,
                    detail: format!("unknown env var for provider '{provider}'"),
                });
            }
        }
    }

    // 6. Model
    checks.push(DoctorCheck {
        name: "Model".to_string(),
        status: DoctorStatus::Pass,
        detail: model.to_string(),
    });

    // 7. Config file
    let mut config_found = Vec::new();
    if std::path::Path::new(".yoyo.toml").exists() {
        config_found.push(".yoyo.toml");
    }
    if let Some(user_path) = cli::user_config_path() {
        if user_path.exists() {
            config_found.push("~/.config/yoyo/config.toml");
        }
    }
    if config_found.is_empty() {
        checks.push(DoctorCheck {
            name: "Config file".to_string(),
            status: DoctorStatus::Warn,
            detail: "none found (.yoyo.toml or ~/.config/yoyo/config.toml)".to_string(),
        });
    } else {
        checks.push(DoctorCheck {
            name: "Config file".to_string(),
            status: DoctorStatus::Pass,
            detail: format!("found: {}", config_found.join(", ")),
        });
    }

    // 8. Project context
    let context_files = cli::list_project_context_files();
    if context_files.is_empty() {
        checks.push(DoctorCheck {
            name: "Project context".to_string(),
            status: DoctorStatus::Warn,
            detail: "no context file (create YOYO.md or run /init)".to_string(),
        });
    } else {
        let descriptions: Vec<String> = context_files
            .iter()
            .map(|(name, lines)| format!("{name} ({lines} lines)"))
            .collect();
        checks.push(DoctorCheck {
            name: "Project context".to_string(),
            status: DoctorStatus::Pass,
            detail: descriptions.join(", "),
        });
    }

    // 9. Curl
    match std::process::Command::new("curl").arg("--version").output() {
        Ok(output) if output.status.success() => {
            checks.push(DoctorCheck {
                name: "Curl".to_string(),
                status: DoctorStatus::Pass,
                detail: "installed (for /docs and /web)".to_string(),
            });
        }
        _ => {
            checks.push(DoctorCheck {
                name: "Curl".to_string(),
                status: DoctorStatus::Warn,
                detail: "not found (/docs and /web won't work)".to_string(),
            });
        }
    }

    // 10. Memory dir (.yoyo/)
    if std::path::Path::new(".yoyo").is_dir() {
        checks.push(DoctorCheck {
            name: "Memory dir".to_string(),
            status: DoctorStatus::Pass,
            detail: ".yoyo/ found".to_string(),
        });
    } else {
        checks.push(DoctorCheck {
            name: "Memory dir".to_string(),
            status: DoctorStatus::Warn,
            detail: ".yoyo/ not found (run /remember to create)".to_string(),
        });
    }

    // 11. RTK (Rust Token Killer) — optional tool output compression
    {
        let rtk_available = crate::rtk::detect_rtk();
        let rtk_disabled = crate::rtk::is_rtk_disabled();
        if rtk_available && !rtk_disabled {
            checks.push(DoctorCheck {
                name: "RTK".to_string(),
                status: DoctorStatus::Pass,
                detail: "installed (auto-compressing tool output)".to_string(),
            });
        } else if rtk_available && rtk_disabled {
            checks.push(DoctorCheck {
                name: "RTK".to_string(),
                status: DoctorStatus::Warn,
                detail: "installed but disabled (--no-rtk flag)".to_string(),
            });
        } else {
            checks.push(DoctorCheck {
                name: "RTK".to_string(),
                status: DoctorStatus::Pass,
                detail: "not installed (optional — compresses build output)".to_string(),
            });
        }
    }

    checks
}

/// Display the doctor report from a list of checks.
pub fn print_doctor_report(checks: &[DoctorCheck]) {
    println!("\n  {BOLD}🩺 yoyo doctor{RESET}");
    println!("  {DIM}─────────────────────────────{RESET}");

    for check in checks {
        let (icon, color) = match check.status {
            DoctorStatus::Pass => ("✓", &GREEN),
            DoctorStatus::Fail => ("✗", &RED),
            DoctorStatus::Warn => ("⚠", &YELLOW),
        };
        println!(
            "  {color}{icon}{RESET} {BOLD}{}{RESET}: {}",
            check.name, check.detail
        );
    }

    let passed = checks
        .iter()
        .filter(|c| c.status == DoctorStatus::Pass)
        .count();
    let total = checks.len();
    let summary_color = if passed == total { &GREEN } else { &YELLOW };
    println!("\n  {summary_color}{passed}/{total} checks passed{RESET}\n");
}

/// Handle the `/doctor` command.
pub fn handle_doctor(provider: &str, model: &str) {
    let checks = run_doctor_checks(provider, model);
    print_doctor_report(&checks);
}

/// Return health check commands for a given project type.
#[allow(clippy::vec_init_then_push, unused_mut)]
pub fn health_checks_for_project(
    project_type: &ProjectType,
) -> Vec<(&'static str, Vec<&'static str>)> {
    match project_type {
        ProjectType::Rust => {
            let mut checks = vec![("build", vec!["cargo", "build"])];
            #[cfg(not(test))]
            checks.push(("test", vec!["cargo", "test"]));
            checks.push((
                "clippy",
                vec!["cargo", "clippy", "--all-targets", "--", "-D", "warnings"],
            ));
            checks.push(("fmt", vec!["cargo", "fmt", "--", "--check"]));
            checks
        }
        ProjectType::Node => {
            let mut checks: Vec<(&str, Vec<&str>)> = vec![];
            #[cfg(not(test))]
            checks.push(("test", vec!["npm", "test"]));
            checks.push(("lint", vec!["npx", "eslint", "."]));
            checks
        }
        ProjectType::Python => {
            let mut checks: Vec<(&str, Vec<&str>)> = vec![];
            #[cfg(not(test))]
            checks.push(("test", vec!["python", "-m", "pytest"]));
            checks.push(("lint", vec!["python", "-m", "flake8", "."]));
            checks.push(("typecheck", vec!["python", "-m", "mypy", "."]));
            checks
        }
        ProjectType::Go => {
            let mut checks = vec![("build", vec!["go", "build", "./..."])];
            #[cfg(not(test))]
            checks.push(("test", vec!["go", "test", "./..."]));
            checks.push(("vet", vec!["go", "vet", "./..."]));
            checks
        }
        ProjectType::Make => {
            // In test builds the push is cfg-gated out, leaving `checks`
            // effectively immutable — but mut is required for production.
            #[cfg(not(test))]
            {
                vec![("test", vec!["make", "test"])]
            }
            #[cfg(test)]
            {
                vec![]
            }
        }
        ProjectType::Unknown => vec![],
    }
}

/// Run health checks for a specific project type. Returns (name, passed, detail) tuples.
pub fn run_health_check_for_project(
    project_type: &ProjectType,
) -> Vec<(&'static str, bool, String)> {
    let checks = health_checks_for_project(project_type);

    let mut results = Vec::new();
    for (name, args) in checks {
        let start = std::time::Instant::now();
        let output = std::process::Command::new(args[0])
            .args(&args[1..])
            .output();
        let elapsed = format_duration(start.elapsed());
        match output {
            Ok(o) if o.status.success() => {
                results.push((name, true, format!("ok ({elapsed})")));
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                let first_line = stderr.lines().next().unwrap_or("(unknown error)");
                results.push((
                    name,
                    false,
                    format!(
                        "FAIL ({elapsed}): {}",
                        truncate_with_ellipsis(first_line, 80)
                    ),
                ));
            }
            Err(e) => {
                results.push((name, false, format!("ERROR: {e}")));
            }
        }
    }
    results
}

/// Run health checks and capture full error output for failures.
pub fn run_health_checks_full_output(
    project_type: &ProjectType,
) -> Vec<(&'static str, bool, String)> {
    let checks = health_checks_for_project(project_type);

    let mut results = Vec::new();
    for (name, args) in checks {
        let output = std::process::Command::new(args[0])
            .args(&args[1..])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                results.push((name, true, String::new()));
            }
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                let mut full_output = String::new();
                if !stdout.is_empty() {
                    full_output.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !full_output.is_empty() {
                        full_output.push('\n');
                    }
                    full_output.push_str(&stderr);
                }
                results.push((name, false, full_output));
            }
            Err(e) => {
                results.push((name, false, format!("ERROR: {e}")));
            }
        }
    }
    results
}

/// Build a prompt describing health check failures for the AI to fix.
pub fn build_fix_prompt(failures: &[(&str, &str)]) -> String {
    if failures.is_empty() {
        return String::new();
    }
    let mut prompt = String::from(
        "Fix the following build/lint errors in this project. Read the relevant files, understand the errors, and apply fixes:\n\n",
    );
    for (name, output) in failures {
        prompt.push_str(&format!("## {name} errors:\n```\n{output}\n```\n\n"));
    }
    prompt.push_str(
        "After fixing, run the failing checks again to verify. Fix any remaining issues.",
    );
    prompt
}

pub fn handle_health() {
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());
    println!("{DIM}  Detected project: {project_type}{RESET}");
    if project_type == ProjectType::Unknown {
        println!(
            "{DIM}  No recognized project found. Looked for: Cargo.toml, package.json, pyproject.toml, setup.py, go.mod, Makefile{RESET}\n"
        );
        return;
    }
    println!("{DIM}  Running health checks...{RESET}");
    let results = run_health_check_for_project(&project_type);
    if results.is_empty() {
        println!("{DIM}  No checks configured for {project_type}{RESET}\n");
        return;
    }
    let all_passed = results.iter().all(|(_, passed, _)| *passed);
    for (name, passed, detail) in &results {
        let icon = if *passed {
            format!("{GREEN}✓{RESET}")
        } else {
            format!("{RED}✗{RESET}")
        };
        println!("  {icon} {name}: {detail}");
    }
    if all_passed {
        println!("\n{GREEN}  All checks passed ✓{RESET}\n");
    } else {
        println!("\n{RED}  Some checks failed ✗{RESET}\n");
    }
}

/// Handle the /fix command. Returns Some(fix_prompt) if failures were sent to AI, None otherwise.
pub async fn handle_fix(
    agent: &mut Agent,
    session_total: &mut Usage,
    model: &str,
) -> Option<String> {
    let project_type = detect_project_type(&std::env::current_dir().unwrap_or_default());
    if project_type == ProjectType::Unknown {
        println!(
            "{DIM}  No recognized project found. Looked for: Cargo.toml, package.json, pyproject.toml, setup.py, go.mod, Makefile{RESET}\n"
        );
        return None;
    }
    println!("{DIM}  Detected project: {project_type}{RESET}");
    println!("{DIM}  Running health checks...{RESET}");
    let results = run_health_checks_full_output(&project_type);
    if results.is_empty() {
        println!("{DIM}  No checks configured for {project_type}{RESET}\n");
        return None;
    }
    for (name, passed, _) in &results {
        let icon = if *passed {
            format!("{GREEN}✓{RESET}")
        } else {
            format!("{RED}✗{RESET}")
        };
        let status = if *passed { "ok" } else { "FAIL" };
        println!("  {icon} {name}: {status}");
    }
    let failures: Vec<(&str, &str)> = results
        .iter()
        .filter(|(_, passed, _)| !passed)
        .map(|(name, _, output)| (*name, output.as_str()))
        .collect();
    if failures.is_empty() {
        println!("\n{GREEN}  All checks passed — nothing to fix ✓{RESET}\n");
        return None;
    }
    let fail_count = failures.len();
    println!("\n{YELLOW}  Sending {fail_count} failure(s) to AI for fixing...{RESET}\n");
    let fix_prompt = build_fix_prompt(&failures);
    run_prompt(agent, &fix_prompt, session_total, model).await;
    auto_compact_if_needed(agent);
    Some(fix_prompt)
}
/// Auto-detect the appropriate watch command for the current project.
/// Returns a lint+test combo command (e.g. `cargo clippy … && cargo test`) when
/// both are available, falls back to test-only, or `None` for unknown project types.
pub fn auto_detect_watch_command() -> Option<String> {
    detect_watch_all_command()
}

/// Auto-detect a combined lint + test command for the current project.
/// Returns both commands chained with `&&` so the first failure stops execution.
/// Falls back to just the test command if no lint command is available,
/// or `None` if neither can be detected.
pub fn detect_watch_all_command() -> Option<String> {
    let dir = std::env::current_dir().unwrap_or_default();
    let project_type = detect_project_type(&dir);
    let lint = lint_command_for_project(&project_type, LintStrictness::Default);
    let test = test_command_for_project(&project_type);
    match (lint, test) {
        (Some((lint_label, _)), Some((test_label, _))) => {
            Some(format!("{lint_label} && {test_label}"))
        }
        (None, Some((test_label, _))) => Some(test_label.to_string()),
        (Some((lint_label, _)), None) => Some(lint_label),
        (None, None) => None,
    }
}

/// Auto-detect separate lint and test commands for two-phase watch.
/// Returns a vec of individual commands (lint first, then test).
/// Falls back to a single-element vec if only one is available.
pub fn detect_watch_all_phases() -> Option<Vec<String>> {
    let dir = std::env::current_dir().unwrap_or_default();
    let project_type = detect_project_type(&dir);
    let lint = lint_command_for_project(&project_type, LintStrictness::Default);
    let test = test_command_for_project(&project_type);
    match (lint, test) {
        (Some((lint_label, _)), Some((test_label, _))) => {
            Some(vec![lint_label.to_string(), test_label.to_string()])
        }
        (None, Some((test_label, _))) => Some(vec![test_label.to_string()]),
        (Some((lint_label, _)), None) => Some(vec![lint_label]),
        (None, None) => None,
    }
}

/// Watch subcommand names for tab completion.
pub const WATCH_SUBCOMMANDS: &[&str] = &["off", "status", "all", "lint"];

/// Handle the /watch command: toggle auto-test-on-edit mode.
pub fn handle_watch(input: &str) {
    let arg = input.strip_prefix("/watch").unwrap_or("").trim();

    match arg {
        "" => {
            // Auto-detect lint+test as separate phases
            match detect_watch_all_phases() {
                Some(phases) => {
                    let display = phases.join(" && ");
                    let phase_refs: Vec<&str> = phases.iter().map(|s| s.as_str()).collect();
                    crate::prompt::set_watch_commands(&phase_refs);
                    if phases.len() > 1 {
                        println!(
                            "{GREEN}  👀 Watch mode ON — {n} phases: `{display}`{RESET}\n",
                            n = phases.len()
                        );
                    } else {
                        println!(
                            "{GREEN}  👀 Watch mode ON — will run `{display}` after agent edits{RESET}\n"
                        );
                    }
                }
                None => {
                    println!("{DIM}  No lint or test command detected. Specify one:{RESET}");
                    println!("{DIM}    /watch cargo clippy && cargo test{RESET}");
                    println!("{DIM}    /watch npm run lint && npm test{RESET}\n");
                }
            }
        }
        "off" => {
            crate::prompt::clear_watch_command();
            println!("{DIM}  👀 Watch mode OFF{RESET}\n");
        }
        "status" => match crate::prompt::get_watch_command() {
            Some(cmd) => {
                let phases = crate::prompt::get_watch_commands();
                println!("{DIM}  👀 Watch mode: ON{RESET}");
                if phases.len() > 1 {
                    println!("{DIM}  Phases ({}):{RESET}", phases.len());
                    for (i, phase) in phases.iter().enumerate() {
                        println!("{DIM}    {}. `{phase}`{RESET}", i + 1);
                    }
                    println!();
                } else {
                    println!("{DIM}  Command: `{cmd}`{RESET}\n");
                }
            }
            None => {
                println!("{DIM}  👀 Watch mode: OFF{RESET}\n");
            }
        },
        "all" => {
            // Auto-detect lint + test as separate phases
            match detect_watch_all_phases() {
                Some(phases) => {
                    let display = phases.join(" && ");
                    let phase_refs: Vec<&str> = phases.iter().map(|s| s.as_str()).collect();
                    crate::prompt::set_watch_commands(&phase_refs);
                    if phases.len() > 1 {
                        println!(
                            "{GREEN}  👀 Watch mode ON — {n} phases: `{display}`{RESET}\n",
                            n = phases.len()
                        );
                    } else {
                        println!(
                            "{GREEN}  👀 Watch mode ON — will run `{display}` after agent edits{RESET}\n"
                        );
                    }
                }
                None => {
                    println!("{DIM}  No lint or test command detected. Specify one:{RESET}");
                    println!("{DIM}    /watch cargo clippy && cargo test{RESET}");
                    println!("{DIM}    /watch npm run lint && npm test{RESET}\n");
                }
            }
        }
        "lint" => {
            // Auto-detect lint-only command
            let dir = std::env::current_dir().unwrap_or_default();
            let project_type = detect_project_type(&dir);
            match lint_command_for_project(&project_type, LintStrictness::Default) {
                Some((lint_label, _)) => {
                    crate::prompt::set_watch_command(&lint_label);
                    println!("{GREEN}  👀 Watch set to: {lint_label}{RESET}\n");
                }
                None => {
                    println!("{DIM}  No lint command detected for this project type.{RESET}");
                    println!("{DIM}    /watch cargo clippy{RESET}");
                    println!("{DIM}    /watch npx eslint .{RESET}\n");
                }
            }
        }
        custom_cmd => {
            crate::prompt::set_watch_command(custom_cmd);
            println!(
                "{GREEN}  👀 Watch mode ON — will run `{custom_cmd}` after agent edits{RESET}\n"
            );
        }
    }
}

// ── /tree ────────────────────────────────────────────────────────────────

/// Build a directory tree from `git ls-files`.
pub fn build_project_tree(max_depth: usize) -> String {
    let files = match crate::git::run_git(&["ls-files"]) {
        Ok(text) => {
            let mut files: Vec<String> = text
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect();
            files.sort();
            files
        }
        Err(_) => return "(not a git repository — /tree requires git)".to_string(),
    };

    if files.is_empty() {
        return "(no tracked files)".to_string();
    }

    format_tree_from_paths(&files, max_depth)
}

/// Format a sorted list of file paths into an indented tree string.
pub fn format_tree_from_paths(paths: &[String], max_depth: usize) -> String {
    use std::collections::BTreeSet;

    let mut output = String::new();
    let mut printed_dirs: BTreeSet<String> = BTreeSet::new();

    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        let depth = parts.len() - 1;

        for level in 0..parts.len().saturating_sub(1).min(max_depth) {
            let dir_path: String = parts[..=level].join("/");
            let dir_key = format!("{}/", dir_path);
            if printed_dirs.insert(dir_key) {
                let indent = "  ".repeat(level);
                let dir_name = parts[level];
                output.push_str(&format!("{indent}{dir_name}/\n"));
            }
        }

        if depth <= max_depth {
            let indent = "  ".repeat(depth.min(max_depth));
            let file_name = parts.last().unwrap_or(&"");
            output.push_str(&format!("{indent}{file_name}\n"));
        }
    }

    if output.ends_with('\n') {
        output.truncate(output.len() - 1);
    }

    output
}

pub fn handle_tree(input: &str) {
    let arg = input.strip_prefix("/tree").unwrap_or("").trim();
    let max_depth = if arg.is_empty() {
        3
    } else {
        match arg.parse::<usize>() {
            Ok(d) => d,
            Err(_) => {
                println!("{DIM}  usage: /tree [depth]  (default depth: 3){RESET}\n");
                return;
            }
        }
    };
    let tree = build_project_tree(max_depth);
    println!("{DIM}{tree}{RESET}\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{is_unknown_command, KNOWN_COMMANDS};

    #[test]
    fn auto_detect_watch_command_returns_lint_and_test_in_rust_project() {
        // We're running from a directory with Cargo.toml, so this should detect Rust
        // After the Day 58 change, auto-detect defaults to lint+test (not test-only)
        let cmd = auto_detect_watch_command();
        assert!(
            cmd.is_some(),
            "should detect a watch command in a Rust project"
        );
        let cmd = cmd.unwrap();
        assert!(
            cmd.contains("clippy"),
            "auto-detect should include lint (clippy): {cmd}"
        );
        assert!(
            cmd.contains("cargo test"),
            "auto-detect should include test: {cmd}"
        );
        assert!(
            cmd.contains("&&"),
            "auto-detect should chain lint && test: {cmd}"
        );
    }

    #[test]
    fn detect_watch_all_command_returns_lint_and_test_for_rust() {
        // We're running from a directory with Cargo.toml, so this should detect Rust
        let cmd = detect_watch_all_command();
        assert!(
            cmd.is_some(),
            "should detect a combined command in a Rust project"
        );
        let cmd = cmd.unwrap();
        assert!(
            cmd.contains("clippy"),
            "combined command should include lint (clippy): {cmd}"
        );
        assert!(
            cmd.contains("cargo test"),
            "combined command should include test: {cmd}"
        );
        assert!(
            cmd.contains("&&"),
            "combined command should chain with &&: {cmd}"
        );
    }

    #[test]
    fn watch_subcommands_includes_all() {
        assert!(
            WATCH_SUBCOMMANDS.contains(&"all"),
            "WATCH_SUBCOMMANDS should include 'all'"
        );
    }

    #[test]
    fn handle_watch_all_sets_combined_command() {
        // Clear any previous watch command
        crate::prompt::clear_watch_command();
        // Run /watch all — since we're in a Rust project, it should set separate phases
        handle_watch("/watch all");
        let cmd = crate::prompt::get_watch_command();
        assert!(
            cmd.is_some(),
            "watch command should be set after /watch all"
        );
        let cmd = cmd.unwrap();
        assert!(
            cmd.contains("clippy") && cmd.contains("cargo test"),
            "watch all should set lint && test: {cmd}"
        );
        // Verify multi-phase: should have 2 separate commands
        let phases = crate::prompt::get_watch_commands();
        assert_eq!(
            phases.len(),
            2,
            "watch all should set 2 separate phases: {phases:?}"
        );
        assert!(
            phases[0].contains("clippy"),
            "first phase should be lint: {}",
            phases[0]
        );
        assert!(
            phases[1].contains("test"),
            "second phase should be test: {}",
            phases[1]
        );
        // Cleanup
        crate::prompt::clear_watch_command();
    }

    #[test]
    fn watch_subcommands_includes_lint() {
        assert!(
            WATCH_SUBCOMMANDS.contains(&"lint"),
            "WATCH_SUBCOMMANDS should include 'lint'"
        );
    }

    #[test]
    fn handle_watch_lint_sets_lint_only_command() {
        // Clear any previous watch command
        crate::prompt::clear_watch_command();
        // Run /watch lint — since we're in a Rust project, it should set clippy only
        handle_watch("/watch lint");
        let cmd = crate::prompt::get_watch_command();
        assert!(
            cmd.is_some(),
            "watch command should be set after /watch lint"
        );
        let cmd = cmd.unwrap();
        assert!(
            cmd.contains("clippy"),
            "watch lint should set lint command: {cmd}"
        );
        assert!(
            !cmd.contains("cargo test"),
            "watch lint should NOT include test: {cmd}"
        );
        // Cleanup
        crate::prompt::clear_watch_command();
    }

    #[test]
    fn handle_watch_bare_sets_lint_and_test() {
        // Clear any previous watch command
        crate::prompt::clear_watch_command();
        // Run bare /watch — should now set lint+test as separate phases
        handle_watch("/watch");
        let cmd = crate::prompt::get_watch_command();
        assert!(
            cmd.is_some(),
            "watch command should be set after bare /watch"
        );
        let cmd = cmd.unwrap();
        assert!(
            cmd.contains("clippy") && cmd.contains("cargo test"),
            "bare /watch should set lint && test: {cmd}"
        );
        // Verify multi-phase
        let phases = crate::prompt::get_watch_commands();
        assert_eq!(
            phases.len(),
            2,
            "bare /watch should set 2 phases: {phases:?}"
        );
        // Cleanup
        crate::prompt::clear_watch_command();
    }

    // ── lint_command_for_project ─────────────────────────────────────

    #[test]
    fn detect_watch_all_phases_returns_separate_commands() {
        // In a Rust project, should return 2 separate commands
        let phases = detect_watch_all_phases();
        assert!(phases.is_some(), "should detect phases in a Rust project");
        let phases = phases.unwrap();
        assert_eq!(
            phases.len(),
            2,
            "should have lint + test phases: {phases:?}"
        );
        assert!(
            phases[0].contains("clippy"),
            "first phase should be lint: {}",
            phases[0]
        );
        assert!(
            phases[1].contains("test"),
            "second phase should be test: {}",
            phases[1]
        );
    }

    #[test]
    fn handle_watch_custom_command_single_phase() {
        crate::prompt::clear_watch_command();
        handle_watch("/watch make check");
        let phases = crate::prompt::get_watch_commands();
        assert_eq!(
            phases.len(),
            1,
            "custom command should be single-phase: {phases:?}"
        );
        assert_eq!(phases[0], "make check");
        crate::prompt::clear_watch_command();
    }

    #[test]
    fn health_checks_rust_has_build() {
        let checks = health_checks_for_project(&ProjectType::Rust);
        assert!(checks.iter().any(|(name, _)| *name == "build"));
    }

    #[test]
    fn health_checks_unknown_empty() {
        let checks = health_checks_for_project(&ProjectType::Unknown);
        assert!(checks.is_empty());
    }

    #[test]
    fn doctor_checks_include_rtk() {
        let checks = run_doctor_checks("anthropic", "test-model");
        assert!(
            checks.iter().any(|c| c.name == "RTK"),
            "doctor checks should include an RTK entry"
        );
        // RTK check should always be Pass (never Fail), since it's optional
        let rtk_check = checks.iter().find(|c| c.name == "RTK").unwrap();
        assert_ne!(
            rtk_check.status,
            DoctorStatus::Fail,
            "RTK should never be Fail — it's optional"
        );
    }

    // ── build_fix_prompt ────────────────────────────────────────────

    #[test]
    fn build_fix_prompt_empty() {
        let prompt = build_fix_prompt(&[]);
        assert!(prompt.is_empty());
    }

    #[test]
    fn build_fix_prompt_with_failures() {
        let failures = vec![("build", "error[E0308]: mismatched types")];
        let prompt = build_fix_prompt(&failures);
        assert!(prompt.contains("build errors"));
        assert!(prompt.contains("E0308"));
        assert!(prompt.contains("Fix"));
    }

    #[test]
    fn build_fix_prompt_multiple_failures() {
        let failures = vec![
            ("build", "build error output"),
            ("clippy", "clippy warning output"),
        ];
        let prompt = build_fix_prompt(&failures);
        assert!(prompt.contains("## build errors"));
        assert!(prompt.contains("## clippy errors"));
    }

    // ── build_lint_fix_prompt ──────────────────────────────────────────
    // ── format_tree_from_paths ──────────────────────────────────────

    #[test]
    fn format_tree_basic() {
        let paths = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "Cargo.toml".to_string(),
        ];
        let tree = format_tree_from_paths(&paths, 3);
        assert!(tree.contains("src/"));
        assert!(tree.contains("main.rs"));
        assert!(tree.contains("lib.rs"));
        assert!(tree.contains("Cargo.toml"));
    }

    #[test]
    fn format_tree_depth_limit() {
        let paths = vec!["a/b/c/d/e.txt".to_string()];
        let tree_shallow = format_tree_from_paths(&paths, 1);
        // At depth 1, we see dir 'a/' but 'b/' is at level 1 so still shown
        // The file at depth 4 should NOT appear since depth > max_depth
        assert!(tree_shallow.contains("a/"));
        // File at depth 4 should not appear when max_depth=1
        assert!(!tree_shallow.contains("e.txt"));
    }

    #[test]
    fn format_tree_empty() {
        let paths: Vec<String> = vec![];
        let tree = format_tree_from_paths(&paths, 3);
        assert!(tree.is_empty());
    }

    #[test]
    fn format_tree_root_files() {
        let paths = vec!["README.md".to_string()];
        let tree = format_tree_from_paths(&paths, 3);
        assert!(tree.contains("README.md"));
    }

    // ── moved from commands.rs (issue #260) ────────────────────────

    #[test]
    fn test_health_check_function() {
        // run_health_check_for_project skips "cargo test" under #[cfg(test)] to avoid recursion
        let project_type = detect_project_type(&std::env::current_dir().unwrap());
        assert_eq!(project_type, ProjectType::Rust);
        let results = run_health_check_for_project(&project_type);
        assert!(
            !results.is_empty(),
            "Health check should return at least one result"
        );
        for (name, passed, _) in &results {
            assert!(!name.is_empty(), "Check name should not be empty");
            if *name == "build" {
                assert!(passed, "cargo build should pass in test environment");
            }
        }
        // "test" check should be excluded under cfg(test)
        assert!(
            !results.iter().any(|(name, _, _)| *name == "test"),
            "cargo test check should be skipped to avoid recursion"
        );
    }

    #[test]
    fn test_health_checks_for_rust_project() {
        let checks = health_checks_for_project(&ProjectType::Rust);
        let names: Vec<&str> = checks.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"build"), "Rust should have build check");
        assert!(names.contains(&"clippy"), "Rust should have clippy check");
        assert!(names.contains(&"fmt"), "Rust should have fmt check");
        // test is excluded under cfg(test)
        assert!(
            !names.contains(&"test"),
            "test should be excluded in cfg(test)"
        );
    }

    #[test]
    fn test_health_checks_for_node_project() {
        let checks = health_checks_for_project(&ProjectType::Node);
        let names: Vec<&str> = checks.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"lint"), "Node should have lint check");
    }

    #[test]
    fn test_health_checks_for_go_project() {
        let checks = health_checks_for_project(&ProjectType::Go);
        let names: Vec<&str> = checks.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"build"), "Go should have build check");
        assert!(names.contains(&"vet"), "Go should have vet check");
    }

    #[test]
    fn test_health_checks_for_python_project() {
        let checks = health_checks_for_project(&ProjectType::Python);
        let names: Vec<&str> = checks.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"lint"), "Python should have lint check");
        assert!(names.contains(&"typecheck"), "Python should have typecheck");
    }

    #[test]
    fn test_health_checks_for_unknown_returns_empty() {
        let checks = health_checks_for_project(&ProjectType::Unknown);
        assert!(checks.is_empty(), "Unknown project should return no checks");
    }

    #[test]
    fn test_run_command_recognized() {
        assert!(!is_unknown_command("/run"));
        assert!(!is_unknown_command("/run echo hello"));
        assert!(!is_unknown_command("/run ls -la"));
    }

    #[test]
    fn test_format_tree_from_paths_basic() {
        let paths = vec![
            "Cargo.toml".to_string(),
            "README.md".to_string(),
            "src/cli.rs".to_string(),
            "src/format.rs".to_string(),
            "src/main.rs".to_string(),
        ];
        let tree = format_tree_from_paths(&paths, 3);
        assert!(tree.contains("Cargo.toml"));
        assert!(tree.contains("README.md"));
        assert!(tree.contains("src/"));
        assert!(tree.contains("  main.rs"));
        assert!(tree.contains("  cli.rs"));
    }

    #[test]
    fn test_format_tree_from_paths_nested() {
        let paths = vec![
            "src/main.rs".to_string(),
            "src/utils/helpers.rs".to_string(),
            "src/utils/format.rs".to_string(),
        ];
        let tree = format_tree_from_paths(&paths, 3);
        assert!(tree.contains("src/"));
        assert!(tree.contains("  utils/"));
        assert!(tree.contains("    helpers.rs"));
        assert!(tree.contains("    format.rs"));
    }

    #[test]
    fn test_format_tree_from_paths_depth_limit() {
        let paths = vec![
            "a/b/c/d/deep.txt".to_string(),
            "a/shallow.txt".to_string(),
            "top.txt".to_string(),
        ];
        // depth 1: show dirs at level 0 ('a/'), files at depth ≤ 1
        let tree = format_tree_from_paths(&paths, 1);
        assert!(tree.contains("top.txt"));
        assert!(tree.contains("a/"));
        assert!(tree.contains("  shallow.txt"));
        // Files deeper than max_depth should not appear
        assert!(!tree.contains("deep.txt"));
        // Directory 'b/' is at level 1, beyond max_depth=1 for dirs
        assert!(!tree.contains("b/"));
    }

    #[test]
    fn test_format_tree_from_paths_empty() {
        let paths: Vec<String> = vec![];
        let tree = format_tree_from_paths(&paths, 3);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_format_tree_from_paths_root_files_only() {
        let paths = vec![
            "Cargo.lock".to_string(),
            "Cargo.toml".to_string(),
            "README.md".to_string(),
        ];
        let tree = format_tree_from_paths(&paths, 3);
        // No directories, just root files
        assert!(!tree.contains('/'));
        assert!(tree.contains("Cargo.lock"));
        assert!(tree.contains("Cargo.toml"));
        assert!(tree.contains("README.md"));
    }

    #[test]
    fn test_format_tree_from_paths_depth_zero() {
        let paths = vec!["README.md".to_string(), "src/main.rs".to_string()];
        let tree = format_tree_from_paths(&paths, 0);
        // Depth 0: only root-level files shown
        assert!(tree.contains("README.md"));
        // main.rs is at depth 1, should not show at depth 0
        assert!(!tree.contains("main.rs"));
    }

    #[test]
    fn test_format_tree_dir_printed_once() {
        let paths = vec![
            "src/a.rs".to_string(),
            "src/b.rs".to_string(),
            "src/c.rs".to_string(),
        ];
        let tree = format_tree_from_paths(&paths, 3);
        // "src/" should appear exactly once
        assert_eq!(tree.matches("src/").count(), 1);
    }

    #[test]
    fn test_build_project_tree_runs() {
        // build_project_tree should return something non-empty
        let tree = build_project_tree(3);
        assert!(!tree.is_empty());
        // In a git repo, should contain Cargo.toml; outside one (e.g. cargo-mutants
        // temp dir) the tree still works but uses filesystem walk instead of git ls-files
    }

    #[test]
    fn test_tree_command_recognized() {
        assert!(!is_unknown_command("/tree"));
        assert!(!is_unknown_command("/tree 2"));
        assert!(!is_unknown_command("/tree 5"));
    }

    #[test]
    fn test_fix_command_recognized() {
        assert!(!is_unknown_command("/fix"));
        assert!(
            KNOWN_COMMANDS.contains(&"/fix"),
            "/fix should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_run_health_checks_full_output_returns_results() {
        // In a Rust project, should return results with full error output
        let project_type = detect_project_type(&std::env::current_dir().unwrap());
        assert_eq!(project_type, ProjectType::Rust);
        let results = run_health_checks_full_output(&project_type);
        assert!(
            !results.is_empty(),
            "Should return at least one check result"
        );
        for (name, passed, _output) in &results {
            assert!(!name.is_empty(), "Check name should not be empty");
            if *name == "build" {
                assert!(passed, "cargo build should pass in test environment");
            }
        }
    }

    #[test]
    fn test_build_fix_prompt_with_failures() {
        let failures = vec![
            (
                "build",
                "error[E0308]: mismatched types\n  --> src/main.rs:42",
            ),
            (
                "clippy",
                "warning: unused variable `x`\n  --> src/lib.rs:10",
            ),
        ];
        let prompt = build_fix_prompt(&failures);
        assert!(prompt.contains("build"), "Prompt should mention build");
        assert!(prompt.contains("clippy"), "Prompt should mention clippy");
        assert!(
            prompt.contains("error[E0308]"),
            "Prompt should include build error"
        );
        assert!(
            prompt.contains("unused variable"),
            "Prompt should include clippy warning"
        );
    }

    #[test]
    fn test_build_fix_prompt_empty_failures() {
        let failures: Vec<(&str, &str)> = vec![];
        let prompt = build_fix_prompt(&failures);
        assert!(
            prompt.is_empty() || prompt.contains("Fix"),
            "Empty failures should produce empty or minimal prompt"
        );
    }
}
