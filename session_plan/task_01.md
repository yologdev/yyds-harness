Title: Selective context summarization — /compact N to keep recent turns full
Files: src/commands_session.rs, src/help.rs, src/dispatch.rs
Issue: none

## What to do

Add a `/compact N` argument to the existing `/compact` command that lets users control how many recent turns to keep at full fidelity while summarizing/dropping everything before them. This is the "summarize up to here" feature that Claude Code has and we don't — the #1 actionable competitive gap from the assessment.

### Current behavior
- `/compact` calls `compact_agent()` which uses yoagent's `compact_messages()` with `ContextConfig::default()` (`keep_recent: 10`).
- No way for the user to control how aggressive the compaction is.

### New behavior
- `/compact` (no args) — same as today, uses default `keep_recent: 10`.
- `/compact N` — sets `keep_recent` to N for this compaction. E.g., `/compact 4` keeps the last 4 messages at full fidelity and aggressively summarizes/drops everything before them.
- `/compact all` — summarize everything older than the last 2 messages (minimum safe).

### Implementation plan

1. **`commands_session.rs`**: Modify `handle_compact` to parse an optional argument:
   - Parse the text after `/compact ` — if it's a number, use it as `keep_recent`. If it's "all", use `keep_recent: 2`.
   - Create a new helper `compact_agent_with_keep(agent, keep_recent)` that builds a `ContextConfig` with the specified `keep_recent` value and a very large `max_context_tokens` (to force Level 2 summarization even when not near the limit — or better, call `level2_summarize_old_turns` logic directly if yoagent exposes it).
   - Actually, yoagent's `compact_messages` only triggers compaction when tokens exceed budget. For selective compaction, we need a different approach: set `max_context_tokens` to 0 or 1 to force compaction to always trigger, then let `keep_recent` control what survives.
   - Display before/after stats with the `keep_recent` value used.

2. **`help.rs`**: Update the `/compact` help entry to document the new argument: `/compact [N]` — optionally specify how many recent messages to keep.

3. **`dispatch.rs`**: The `/compact` route already passes to `handle_compact(ctx.agent)`. Change it to also pass the argument string (the text after `/compact`). Check how other commands pass args — look at patterns like `handle_save(agent, input)`.

### Tests
- Test that `compact_agent_with_keep(agent, 2)` compacts more aggressively than default.
- Test that parsing "5" returns `keep_recent: 5`, "all" returns `keep_recent: 2`, empty returns default 10.
- Test invalid input like "/compact abc" gives a helpful error.

### Verify
`cargo build && cargo test`
