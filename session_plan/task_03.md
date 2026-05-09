Title: Add /changes summary — AI-generated session change summary
Files: src/commands_retry.rs, src/dispatch.rs
Issue: none (developer workflow improvement — competitive with Cursor's PR decomposition)

## Problem

After a session with multiple file edits, users need to understand "what just
happened?" at a high level. `/changes` shows a list of files touched, and
`/changes --diff` shows the raw diffs. But there's no way to get a natural-language
summary of the session's changes — the kind of thing you'd write in a PR description
or tell a colleague.

This is a stepping stone toward smarter PR workflows. Cursor can "Split Changes into
PRs" — that requires understanding what the changes ARE first. A `/changes summary`
gives us that semantic understanding.

## Solution

Add a `/changes summary` subcommand that uses a side agent to generate a concise
natural-language summary of all changes made during the session.

### Implementation Details

1. **Add `handle_changes_summary()` in `src/commands_retry.rs`**:
   - Collect all changed files from `SessionChanges::snapshot()`
   - For each changed file, get the `git diff` (or "new file" indicator)
   - Build a prompt: "Here are the file changes made during this session.
     Write a concise summary suitable for a PR description or commit message.
     Group related changes. Be specific about what changed and why."
   - Use `build_side_agent` (from `agent_builder`) to run a quick side agent
     with the diffs as context
   - Print the summary to stdout

2. **Update `handle_changes()` in `src/commands_retry.rs`**:
   - Check if the input contains "summary" after `/changes`
   - If `/changes summary`, call `handle_changes_summary()`
   - Otherwise, existing behavior

3. **Wire up in dispatch** — the existing `/changes` dispatch in `src/dispatch.rs`
   already calls `handle_changes()` with the full input, so the `summary`
   subcommand will flow through naturally. Just need to pass `agent` and
   `agent_config` to `handle_changes()` when the summary subcommand is detected.

   Actually, to keep it clean: add a separate match arm in `dispatch.rs` for
   `/changes summary` that calls `handle_changes_summary()` directly, passing
   the agent, agent_config, session_total, and session_changes.

4. **Summary format**: The side agent should produce output like:
   ```
   ## Session Summary

   - **Fixed error handling in provider switching** (src/prompt.rs):
     Replaced 3 `.ok()` calls with proper `if let Err(e)` logging...
   - **Added GPT-5 pricing** (src/format/cost.rs):
     Added cost data for 5 new models...
   ```

5. **Tests**: Add unit tests for the argument parsing (detecting "summary" subcommand).
   The actual AI generation can't be unit-tested but the argument routing can.

6. **Update help text** in `src/help.rs`: Add `/changes summary` to the `/changes`
   help entry.

### Scope

Touch 2-3 files: `commands_retry.rs` (main logic), `dispatch.rs` (routing),
`help.rs` (help text). The side agent call is lightweight — it's a single prompt
with the diffs as context.
