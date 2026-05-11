Title: Add /plan apply and /plan show — structured plan-then-execute workflow
Files: src/commands_plan.rs, src/dispatch.rs, src/help.rs
Issue: none

## What

Currently `/plan <task>` generates a plan and prints "say 'go ahead' to execute it" — but there's no structured way to review or apply the last plan. This adds two subcommands:

- `/plan show` — display the last generated plan
- `/plan apply` — re-inject the last plan as an execution prompt (agent runs it in normal mode)

This closes competitive gap #3 from the assessment: "Plan-then-apply mode with explicit human approval gates." Claude Code and Cursor both have review-before-apply workflows.

## Implementation

### 1. Store last plan in `commands_plan.rs`

Add a `Mutex<Option<String>>` to hold the last plan output:

```rust
use std::sync::Mutex;

static LAST_PLAN: Mutex<Option<String>> = Mutex::new(None);

pub fn set_last_plan(plan: String) {
    if let Ok(mut guard) = LAST_PLAN.lock() {
        *guard = Some(plan);
    }
}

pub fn get_last_plan() -> Option<String> {
    LAST_PLAN.lock().ok().and_then(|g| g.clone())
}

pub fn clear_last_plan() {
    if let Ok(mut guard) = LAST_PLAN.lock() {
        *guard = None;
    }
}
```

In `handle_plan`, after the plan is generated (after `run_prompt` returns), capture the last assistant message and store it via `set_last_plan()`. The plan text can be extracted from the agent's messages (the last assistant message after the plan prompt).

### 2. Add subcommands to `handle_plan`

Add `"show"` and `"apply"` to the match in `handle_plan`:

- `"show"` — retrieve `get_last_plan()` and print it. If None, print "No plan stored. Use `/plan <task>` to create one."
- `"apply"` — retrieve `get_last_plan()`, construct an execution prompt like:
  ```
  Execute the following plan. Implement each step, writing code and running tests as you go.
  
  ## Plan
  {plan_text}
  
  Work through each step. After completing all steps, verify with `cargo build && cargo test` (or the project's equivalent).
  ```
  Then call `run_prompt(agent, &apply_prompt, session_total, model).await` in NORMAL mode (not plan mode). Print a confirmation message before executing. Clear the stored plan after applying.

- `"clear"` — clear the stored plan.

### 3. Update `PLAN_SUBCOMMANDS`

Add `"show"`, `"apply"`, `"clear"` to `PLAN_SUBCOMMANDS` for tab completion.

### 4. Update help text in `help.rs`

Update the `command_help("plan")` entry to document the new subcommands:
- `/plan on/off` — toggle plan mode
- `/plan <task>` — one-shot plan
- `/plan show` — display last plan
- `/plan apply` — execute the last plan
- `/plan clear` — discard stored plan

### 5. Update dispatch.rs

The `/plan apply` subcommand returns the apply prompt as a `CommandResult::Prompt(prompt)` so the REPL can execute it. Check how `/plan` currently returns `Some(plan_prompt)` — `apply` should follow the same pattern but without the plan-mode restriction.

### 6. Add tests

- `test_set_get_clear_last_plan` — store, retrieve, clear
- `test_plan_apply_subcommand_parsing` — verify "apply" is recognized and not treated as a task description
- `test_plan_show_subcommand_parsing` — same for "show"
- Update `test_parse_plan_task_skips_mode_keywords` to also skip "show", "apply", "clear"

## Verification
`cargo test commands_plan` should pass. The workflow: `/plan add error handling` → reviews plan → `/plan apply` → agent executes.
