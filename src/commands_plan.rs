//! Plan command handler: `/plan` — toggle plan mode or create a structured plan.

use crate::commands::auto_compact_if_needed;
use crate::format::*;
use crate::prompt::run_prompt;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use yoagent::agent::Agent;
use yoagent::*;

// ---------------------------------------------------------------------------
// Plan mode — a session toggle that restricts the agent to read-only operations.
// When active, a constraint instruction is prepended to each user message so
// the agent reads and thinks but does not modify files or run destructive commands.
// ---------------------------------------------------------------------------

static PLAN_MODE: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// Plan-apply mode — tracks whether `/plan apply` is currently executing.
// When active, the auto-continue limit is raised so the agent can work
// through the full plan without hitting the normal follow-up cap.
// ---------------------------------------------------------------------------

static PLAN_APPLY_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Set whether a `/plan apply` execution is currently in progress.
pub fn set_plan_apply_active(active: bool) {
    PLAN_APPLY_ACTIVE.store(active, Ordering::Relaxed);
}

/// Check whether a `/plan apply` execution is currently in progress.
pub fn is_plan_apply_active() -> bool {
    PLAN_APPLY_ACTIVE.load(Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Last plan storage — holds the text of the most recently generated plan so
// the user can review (/plan show) or execute (/plan apply) it later.
// ---------------------------------------------------------------------------

static LAST_PLAN: Mutex<Option<String>> = Mutex::new(None);

/// Store the text of the last generated plan.
pub fn set_last_plan(plan: String) {
    if let Ok(mut guard) = LAST_PLAN.lock() {
        *guard = Some(plan);
    }
}

/// Retrieve the last stored plan, if any.
pub fn get_last_plan() -> Option<String> {
    LAST_PLAN.lock().ok().and_then(|g| g.clone())
}

/// Clear the stored plan.
pub fn clear_last_plan() {
    if let Ok(mut guard) = LAST_PLAN.lock() {
        *guard = None;
    }
}

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
pub const PLAN_SUBCOMMANDS: &[&str] = &["on", "off", "open", "close", "show", "apply", "clear"];

/// Parse a `/plan` command and extract the task description.
/// Returns None if no task was provided or if the input is a mode toggle keyword.
pub fn parse_plan_task(input: &str) -> Option<String> {
    let task = input.strip_prefix("/plan").unwrap_or("").trim().to_string();
    if task.is_empty() {
        None
    } else {
        // Don't treat mode toggle keywords as plan tasks
        match task.as_str() {
            "on" | "off" | "open" | "close" | "show" | "apply" | "clear" => None,
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

/// Build a prompt that instructs the agent to execute a previously generated plan.
pub fn build_apply_prompt(plan_text: &str) -> String {
    format!(
        "Execute the following plan. Implement each step, writing code and running tests as you go.\n\n\
         ## Plan\n{plan_text}\n\n\
         Work through each step. After completing all steps, verify with `cargo build && cargo test` \
         (or the project's equivalent)."
    )
}

/// Result of handling a `/plan` command.
pub enum PlanResult {
    /// Command handled internally (toggle, show, clear, or no-op). Continue the REPL.
    Handled,
    /// A plan was generated. Contains the plan prompt used (stored as last_input).
    PlanGenerated(String),
    /// The user requested `/plan apply` — the returned string should be sent to the agent.
    Apply(String),
}

/// Handle the `/plan` command: toggle plan mode, create a structured plan,
/// or manage stored plans.
///
/// - `/plan on` or `/plan open` — enable plan mode (read-only)
/// - `/plan off` or `/plan close` — disable plan mode
/// - `/plan` (no args) — show current mode + usage
/// - `/plan <task>` — single-shot plan (generates + stores for later apply)
/// - `/plan show` — display the last generated plan
/// - `/plan apply` — execute the last plan via the agent
/// - `/plan clear` — discard the stored plan
pub async fn handle_plan(
    input: &str,
    agent: &mut Agent,
    session_total: &mut Usage,
    model: &str,
) -> PlanResult {
    let arg = input.strip_prefix("/plan").unwrap_or("").trim();

    // Handle mode toggle subcommands
    match arg {
        "on" | "open" => {
            set_plan_mode(true);
            println!(
                "{GREEN}  📋 Plan mode ON — agent will read and think but not modify files or run commands.{RESET}"
            );
            println!("{DIM}  Use /plan off to return to normal mode.{RESET}\n");
            return PlanResult::Handled;
        }
        "off" | "close" => {
            set_plan_mode(false);
            println!("{DIM}  Plan mode OFF — normal operation resumed.{RESET}\n");
            return PlanResult::Handled;
        }
        "show" => {
            match get_last_plan() {
                Some(plan) => {
                    println!("{BOLD}  📋 Last generated plan:{RESET}\n");
                    println!("{plan}\n");
                }
                None => {
                    println!("{DIM}  No plan stored. Use /plan <task> to create one.{RESET}\n");
                }
            }
            return PlanResult::Handled;
        }
        "apply" => match get_last_plan() {
            Some(plan) => {
                let prompt = build_apply_prompt(&plan);
                println!("{GREEN}  🚀 Applying stored plan…{RESET}\n");
                clear_last_plan();
                return PlanResult::Apply(prompt);
            }
            None => {
                println!("{DIM}  No plan stored. Use /plan <task> to create one first.{RESET}\n");
                return PlanResult::Handled;
            }
        },
        "clear" => {
            clear_last_plan();
            println!("{DIM}  Stored plan cleared.{RESET}\n");
            return PlanResult::Handled;
        }
        "" => {
            // No args: show status + usage
            if is_plan_mode() {
                println!("{GREEN}  📋 Plan mode is ON{RESET}");
                println!("{DIM}  The agent can read and search but will not modify files.{RESET}");
                println!("{DIM}  Use /plan off to return to normal mode.{RESET}\n");
            } else {
                let has_plan = get_last_plan().is_some();
                println!("{DIM}  📋 Plan mode is OFF (normal operation){RESET}");
                println!("{DIM}  usage: /plan on          Enter plan mode (read-only){RESET}");
                println!("{DIM}         /plan off         Return to normal mode{RESET}");
                println!(
                    "{DIM}         /plan <task>      One-shot plan without executing tools{RESET}"
                );
                println!("{DIM}         /plan show        Display the last generated plan{RESET}");
                println!("{DIM}         /plan apply       Execute the last generated plan{RESET}");
                println!("{DIM}         /plan clear       Discard the stored plan{RESET}");
                if has_plan {
                    println!(
                        "{GREEN}  ✓ A plan is currently stored. Use /plan show to view it.{RESET}"
                    );
                }
                println!();
            }
            return PlanResult::Handled;
        }
        _ => {}
    }

    // Fall through to single-shot planning
    let task = match parse_plan_task(input) {
        Some(t) => t,
        None => {
            // Shouldn't reach here given the match above, but be safe
            return PlanResult::Handled;
        }
    };

    println!("{DIM}  📋 Planning: {task}{RESET}\n");

    let plan_prompt = build_plan_prompt(&task);
    run_prompt(agent, &plan_prompt, session_total, model).await;
    auto_compact_if_needed(agent);

    // Capture the plan text from the last assistant message for later retrieval
    if let Some(plan_text) = crate::commands_web::extract_last_assistant_text(agent.messages()) {
        set_last_plan(plan_text);
    }

    println!(
        "\n{DIM}  💡 Review the plan above. Use /plan apply to execute it, or refine it.{RESET}\n"
    );

    PlanResult::PlanGenerated(plan_prompt)
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
        assert!(PLAN_SUBCOMMANDS.contains(&"show"));
        assert!(PLAN_SUBCOMMANDS.contains(&"apply"));
        assert!(PLAN_SUBCOMMANDS.contains(&"clear"));
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
