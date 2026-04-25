Title: /plan mode toggle — think without acting
Files: src/commands_project.rs, src/tools.rs, src/repl.rs
Issue: none

## What

Add `/plan on` and `/plan off` (or `/plan open`/`/plan close`) to toggle a "plan mode" where the agent can read files and think but cannot write files, edit files, or execute bash commands that modify state. This matches Claude Code's `/plan open`/`/plan close` feature.

## Why

Claude Code's plan mode lets users say "think about this but don't change anything yet." Our `/plan <task>` does a single-shot plan, but there's no way to enter a sustained planning mode where the agent freely explores the codebase (reading files, searching) without making changes. This is useful for:
- Understanding a codebase before making changes
- Getting the agent's analysis without risk
- Reviewing proposed changes before execution

## Implementation

### 1. Add plan mode state to `src/commands_project.rs`
Add a global `AtomicBool` for plan mode state (same pattern as teach mode in `commands_config.rs`):

```rust
static PLAN_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_plan_mode(enabled: bool) {
    PLAN_MODE.store(enabled, Ordering::Relaxed);
}

pub fn is_plan_mode() -> bool {
    PLAN_MODE.load(Ordering::Relaxed)
}
```

Update `handle_plan` to handle subcommands:
- `/plan on` or `/plan open` → enable plan mode, print indicator
- `/plan off` or `/plan close` → disable plan mode, print indicator
- `/plan <task>` → existing single-shot plan behavior (unchanged)
- `/plan` (no args) → show current mode + usage

When plan mode activates, print:
```
  📋 Plan mode ON — agent will read and think but not modify files or run commands.
  Use /plan off to return to normal mode.
```

### 2. Guard write operations in `src/tools.rs`
In the `StreamingBashTool::execute()` and in the `TruncatingTool`/tool execution path, check `is_plan_mode()`. When plan mode is on:

- `bash` tool: Allow read-only commands (grep, cat, ls, find, git log, git diff, etc.). Block commands that could modify state. The simplest approach: in `StreamingBashTool::execute()`, if plan mode is on, prepend a check. Actually, the cleanest approach is to inject a system message when plan mode is on, telling the agent "You are in PLAN MODE. You may use read_file, list_files, search, and read-only bash commands (cat, grep, find, git log, git status, git diff). Do NOT use write_file, edit_file, or bash commands that modify files. Think through your plan and explain what you WOULD do." This is simpler and more robust than trying to block individual tools.

Best approach: When plan mode is on, inject a plan-mode instruction into the system prompt or as a user message prefix. The agent is instruction-following and will respect the constraint. This avoids modifying tool internals.

In `src/repl.rs`, before sending a prompt to the agent, check `is_plan_mode()`. If on, prepend a plan-mode instruction to the user message:
```
[PLAN MODE] You are in planning mode. You may read files and search but MUST NOT modify any files or run destructive commands. Analyze the codebase, explain your plan, and describe what changes you WOULD make. Do not use write_file, edit_file, or bash commands that create/modify/delete files.
```

### 3. Show plan mode in REPL prompt in `src/repl.rs`
When plan mode is on, modify the prompt line to show it:
```
main 📋 🐙 ›    (plan mode on)
```
vs normal:
```
main 🐙 ›
```

### Tests
- Test `set_plan_mode` / `is_plan_mode` toggle
- Test that `/plan on` and `/plan off` parse correctly
- Test that `/plan <task>` still works (single-shot plan)
- Test that plan mode instruction is prepended when mode is on
- Test prompt includes 📋 indicator when plan mode is on

### Docs
- Add plan mode to help text for `/plan`
- Mention in CLAUDE.md under Architecture if it changes tool behavior
