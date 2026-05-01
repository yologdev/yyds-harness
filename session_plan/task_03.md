Title: Add /goal command for persistent session goals
Files: src/commands_goal.rs, src/dispatch.rs, src/help.rs
Issue: none

## Goal

Codex v0.128.0 just shipped "persisted /goal workflows with pause/resume." yoyo has nothing
equivalent. Add a simple `/goal` command that lets users set a persistent goal for their
session/project that gets injected into the system prompt context.

## Design

A `/goal` command with subcommands:

- `/goal set <description>` — save goal to `.yoyo/goal.md` (project-local)
- `/goal` or `/goal show` — display the current goal
- `/goal clear` — remove the current goal
- `/goal check` — ask the agent to evaluate progress against the goal

Storage: plain text file at `.yoyo/goal.md`. Simple, human-readable, version-controllable.

## Implementation

1. **New file `src/commands_goal.rs`** (~150-200 lines):
   - `pub fn handle_goal(input: &str) -> CommandResult` — parse subcommand and dispatch
   - `pub fn load_goal() -> Option<String>` — read `.yoyo/goal.md` if it exists
   - `fn save_goal(goal: &str)` — write to `.yoyo/goal.md`, creating `.yoyo/` if needed
   - `fn clear_goal()` — remove `.yoyo/goal.md`
   - `fn show_goal()` — display current goal with formatting
   - Tests for save/load/clear round-trip, show with no goal, subcommand parsing

2. **Wire into dispatch.rs**:
   - Add `/goal` to the command dispatch match
   - Import `commands_goal::handle_goal`

3. **Wire into help.rs**:
   - Add `/goal` to the command list with subcommands `set`, `show`, `clear`, `check`
   - Add short description: "Set, view, or check progress on a session goal"

4. **Context injection** (lightweight):
   - In `commands_goal.rs`, export `load_goal()` so it can be called from context loading
   - The actual context injection (calling `load_goal()` during prompt building) can be
     a follow-up task to keep this one scoped. For now, `/goal check` will manually
     include the goal text in a prompt to the agent.

## What NOT to do

- Don't build pause/resume workflow orchestration (that's Codex's full version — too complex)
- Don't modify the system prompt injection path yet (follow-up task)
- Don't add goal history/versioning

## Testing

- Test subcommand parsing (set, show, clear, check, empty, unknown)
- Test save/load/clear round-trip using a temp directory
- Test `load_goal()` returns None when no file exists
- Test that `.yoyo/` directory is created if missing

## Why

This is a competitive response to Codex v0.128.0's goal persistence feature. Even a simple
version — "here's what I'm trying to accomplish" persisted to disk — gives developers a way
to maintain focus across sessions. The full pause/resume workflow can be built incrementally
on top of this foundation.
