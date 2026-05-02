//! Plan command handler: `/plan` — toggle plan mode or create a structured plan.

use crate::commands::auto_compact_if_needed;
use crate::format::*;
use crate::prompt::run_prompt;

use std::sync::atomic::{AtomicBool, Ordering};
use yoagent::agent::Agent;
use yoagent::*;

// ---------------------------------------------------------------------------
// Plan mode — a session toggle that restricts the agent to read-only operations.
// When active, a constraint instruction is prepended to each user message so
// the agent reads and thinks but does not modify files or run destructive commands.
// ---------------------------------------------------------------------------

static PLAN_MODE: AtomicBool = AtomicBool::new(false);

/// Enable or disable plan mode.
pub fn set_plan_mode(enabled: bool) {
    PLAN_MODE.store(enabled, Ordering::Relaxed);
}

/// Check whether plan mode is currently active.
pub fn is_plan_mode() -> bool {
    PLAN_MODE.load(Ordering::Relaxed)
}

/// Instruction prepended to user messages when plan mode is on.
pub const PLAN_MODE_PROMPT: &str = "\
[PLAN MODE] You are in planning mode. You may read files, search, and analyze the codebase, \
but you MUST NOT modify any files or run destructive commands. Specifically:
- DO NOT use write_file or edit_file
- DO NOT use bash commands that create, modify, or delete files
- You MAY use read_file, list_files, search, and read-only bash commands (cat, grep, find, git log, git status, git diff)
Analyze the codebase, explain your plan, and describe what changes you WOULD make without making them.";

/// Subcommand names for `/plan <Tab>` completion.
pub const PLAN_SUBCOMMANDS: &[&str] = &["on", "off", "open", "close"];

/// Parse a `/plan` command and extract the task description.
/// Returns None if no task was provided or if the input is a mode toggle keyword.
pub fn parse_plan_task(input: &str) -> Option<String> {
    let task = input.strip_prefix("/plan").unwrap_or("").trim().to_string();
    if task.is_empty() {
        None
    } else {
        // Don't treat mode toggle keywords as plan tasks
        match task.as_str() {
            "on" | "off" | "open" | "close" => None,
            _ => Some(task),
        }
    }
}

/// Build a planning-mode prompt that asks the agent to create a structured plan
/// WITHOUT executing any tools. This is the "architect mode" equivalent.
pub fn build_plan_prompt(task: &str) -> String {
    format!(
        r#"Create a detailed step-by-step plan for the following task. Do NOT execute any tools — this is planning only.

## Task
{task}

## Instructions
Analyze the task and produce a structured plan covering:

1. **Files to examine** — which existing files need to be read to understand the current state
2. **Files to modify** — which files will be created or changed, and what changes
3. **Step-by-step approach** — ordered list of concrete implementation steps
4. **Tests to write** — what tests should be added or updated
5. **Potential risks** — what could go wrong, edge cases, backwards compatibility concerns
6. **Verification** — how to confirm the changes work correctly

Be specific: mention file paths, function names, and concrete code changes where possible.
Keep the plan actionable — someone (or you, in the next step) should be able to execute it directly."#
    )
}

/// Handle the `/plan` command: toggle plan mode or create a structured plan.
///
/// - `/plan on` or `/plan open` — enable plan mode (read-only)
/// - `/plan off` or `/plan close` — disable plan mode
/// - `/plan` (no args) — show current mode + usage
/// - `/plan <task>` — existing single-shot plan behavior (unchanged)
///
/// Returns Some(plan_prompt) if a single-shot plan was requested, None otherwise.
pub async fn handle_plan(
    input: &str,
    agent: &mut Agent,
    session_total: &mut Usage,
    model: &str,
) -> Option<String> {
    let arg = input.strip_prefix("/plan").unwrap_or("").trim();

    // Handle mode toggle subcommands
    match arg {
        "on" | "open" => {
            set_plan_mode(true);
            println!(
                "{GREEN}  📋 Plan mode ON — agent will read and think but not modify files or run commands.{RESET}"
            );
            println!("{DIM}  Use /plan off to return to normal mode.{RESET}\n");
            return None;
        }
        "off" | "close" => {
            set_plan_mode(false);
            println!("{DIM}  Plan mode OFF — normal operation resumed.{RESET}\n");
            return None;
        }
        "" => {
            // No args: show status + usage
            if is_plan_mode() {
                println!("{GREEN}  📋 Plan mode is ON{RESET}");
                println!("{DIM}  The agent can read and search but will not modify files.{RESET}");
                println!("{DIM}  Use /plan off to return to normal mode.{RESET}\n");
            } else {
                println!("{DIM}  📋 Plan mode is OFF (normal operation){RESET}");
                println!("{DIM}  usage: /plan on         Enter plan mode (read-only){RESET}");
                println!("{DIM}         /plan off        Return to normal mode{RESET}");
                println!(
                    "{DIM}         /plan <task>     One-shot plan without executing tools{RESET}\n"
                );
            }
            return None;
        }
        _ => {}
    }

    // Fall through to single-shot planning
    let task = match parse_plan_task(input) {
        Some(t) => t,
        None => {
            // Shouldn't reach here given the match above, but be safe
            return None;
        }
    };

    println!("{DIM}  📋 Planning: {task}{RESET}\n");

    let plan_prompt = build_plan_prompt(&task);
    run_prompt(agent, &plan_prompt, session_total, model).await;
    auto_compact_if_needed(agent);

    println!(
        "\n{DIM}  💡 Review the plan above. Say \"go ahead\" to execute it, or refine it.{RESET}\n"
    );

    Some(plan_prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_task_with_description() {
        let result = parse_plan_task("/plan add error handling to the parser");
        assert_eq!(result, Some("add error handling to the parser".to_string()));
    }

    #[test]
    fn parse_plan_task_empty() {
        let result = parse_plan_task("/plan");
        assert!(result.is_none(), "Empty /plan should return None");
    }

    #[test]
    fn parse_plan_task_whitespace_only() {
        let result = parse_plan_task("/plan   ");
        assert!(result.is_none(), "Whitespace-only /plan should return None");
    }

    #[test]
    fn parse_plan_task_preserves_full_description() {
        let result = parse_plan_task("/plan refactor main.rs into smaller modules with tests");
        assert_eq!(
            result,
            Some("refactor main.rs into smaller modules with tests".to_string())
        );
    }

    #[test]
    fn build_plan_prompt_contains_task() {
        let prompt = build_plan_prompt("add a /plan command");
        assert!(
            prompt.contains("add a /plan command"),
            "Plan prompt should contain the task"
        );
    }

    #[test]
    fn build_plan_prompt_contains_no_tools_instruction() {
        let prompt = build_plan_prompt("something");
        assert!(
            prompt.contains("Do NOT execute any tools"),
            "Plan prompt should instruct not to use tools"
        );
    }

    #[test]
    fn build_plan_prompt_contains_structure_sections() {
        let prompt = build_plan_prompt("add feature X");
        assert!(
            prompt.contains("Files to examine"),
            "Should mention files to examine"
        );
        assert!(
            prompt.contains("Files to modify"),
            "Should mention files to modify"
        );
        assert!(
            prompt.contains("Step-by-step"),
            "Should mention step-by-step approach"
        );
        assert!(prompt.contains("Tests to write"), "Should mention tests");
        assert!(prompt.contains("Potential risks"), "Should mention risks");
        assert!(
            prompt.contains("Verification"),
            "Should mention verification"
        );
    }

    #[test]
    fn test_parse_plan_task_extracts_task() {
        let result = parse_plan_task("/plan add error handling");
        assert_eq!(result, Some("add error handling".to_string()));
    }

    #[test]
    fn test_parse_plan_task_empty_returns_none() {
        assert!(parse_plan_task("/plan").is_none());
        assert!(parse_plan_task("/plan  ").is_none());
    }

    #[test]
    fn test_build_plan_prompt_structure() {
        let prompt = build_plan_prompt("migrate database schema");
        assert!(prompt.contains("migrate database schema"));
        assert!(prompt.contains("Do NOT execute any tools"));
        assert!(prompt.contains("Files to examine"));
        assert!(prompt.contains("Step-by-step"));
    }

    #[test]
    fn test_plan_mode_toggle() {
        // Ensure clean state
        set_plan_mode(false);
        assert!(!is_plan_mode());

        set_plan_mode(true);
        assert!(is_plan_mode());

        set_plan_mode(false);
        assert!(!is_plan_mode());
    }

    #[test]
    fn test_parse_plan_task_skips_mode_keywords() {
        // Mode toggle keywords should NOT be treated as plan tasks
        assert!(parse_plan_task("/plan on").is_none());
        assert!(parse_plan_task("/plan off").is_none());
        assert!(parse_plan_task("/plan open").is_none());
        assert!(parse_plan_task("/plan close").is_none());

        // But actual task descriptions should still work
        assert_eq!(
            parse_plan_task("/plan add error handling"),
            Some("add error handling".to_string())
        );
        assert_eq!(
            parse_plan_task("/plan on-boarding flow"),
            Some("on-boarding flow".to_string())
        );
    }

    #[test]
    fn test_plan_mode_prompt_content() {
        // The plan mode prompt should instruct the agent not to modify files
        assert!(PLAN_MODE_PROMPT.contains("PLAN MODE"));
        assert!(PLAN_MODE_PROMPT.contains("MUST NOT"));
        assert!(PLAN_MODE_PROMPT.contains("write_file"));
        assert!(PLAN_MODE_PROMPT.contains("edit_file"));
        assert!(PLAN_MODE_PROMPT.contains("read_file"));
    }

    #[test]
    fn test_plan_subcommands() {
        assert!(PLAN_SUBCOMMANDS.contains(&"on"));
        assert!(PLAN_SUBCOMMANDS.contains(&"off"));
        assert!(PLAN_SUBCOMMANDS.contains(&"open"));
        assert!(PLAN_SUBCOMMANDS.contains(&"close"));
    }

    #[test]
    fn test_plan_in_known_commands() {
        use crate::commands::KNOWN_COMMANDS;
        assert!(
            KNOWN_COMMANDS.contains(&"/plan"),
            "/plan should be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_plan_in_help_text() {
        use crate::help::help_text;
        let help = help_text();
        assert!(help.contains("/plan"), "/plan should appear in help text");
        assert!(
            help.contains("architect"),
            "Help text should mention architect mode"
        );
    }
}
