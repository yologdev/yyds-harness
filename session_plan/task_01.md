Title: Inject persistent goal into agent system prompt
Files: src/cli.rs, src/commands_goal.rs
Issue: none

## What

When `.yoyo/goal.md` exists (set via `/goal set`), its content should be injected into the
agent's system prompt so the AI is always aware of the user's current goal. Currently, `/goal`
stores text to a file but the agent never sees it — the user has to manually run `/goal check`
or paste the goal into conversation.

Claude Code injects persistent project context and goals into every turn. This is a key
competitive gap.

## Implementation

1. In `src/commands_goal.rs`, ensure `load_goal()` is `pub` (it already is).

2. In `src/cli.rs`, after the repo map injection block (~line 750), add a goal injection block:
   ```rust
   // Append current goal for persistent awareness
   if let Some(goal) = crate::commands_goal::load_goal() {
       system_prompt.push_str("\n\n# Current Goal\n\n");
       system_prompt.push_str(&goal);
       system_prompt.push_str("\n\n(Set via /goal set. The user is working toward this. Keep it in mind.)");
   }
   ```

3. Add tests in `src/commands_goal.rs`:
   - Test that `load_goal()` returns `None` when file doesn't exist (may already exist)
   - Test that `load_goal()` returns content when file exists (use temp dir)

4. Update the `/goal` help text in `src/help_data.rs` to mention that the goal is injected
   into the system prompt:
   > "Your goal is automatically included in the AI's context, so it stays aware of what
   > you're working toward across the entire conversation."

## Verification
- `cargo build && cargo test`
- Check that the system prompt assembly in `parse_args` includes the goal block
