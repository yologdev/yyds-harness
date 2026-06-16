Title: Fix `state failures tools` returning "no parseable events" when events exist
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: "`state failures tools` returned 'no parseable events found at .yoyo/state/events.jsonl' despite `state tail` working"
- Source analysis: `handle_tool_failures` (line 822) uses `read_events_lenient` to parse events, while `state tail` (line 440) uses `read_tail` which reads raw lines without JSON parsing. The discrepancy suggests `read_events_lenient` may reject lines that exist and are valid JSON.
- Other commands like `handle_trace` (line 461) use `read_events` (not `read_events_lenient`) and work correctly.
- `handle_failures` (the parent, line 773) also uses `read_events_lenient` and may have the same bug, but the assessment only tested `failures tools`.

Edit Surface:
- src/commands_state.rs

Verifier:
- cargo build && cargo test --bin yyds -- commands_state --test-threads=1

Fallback:
- If `read_events_lenient` is intentionally different from `read_events` for a valid reason (e.g., it's the only reader that handles corrupted lines gracefully), the fix may instead need to make `handle_tool_failures` fall back to `read_events` when `read_events_lenient` returns zero events.
- If the root cause is in `read_events_lenient` itself (a bug in the parsing logic), fix it there but keep the edit surface within src/commands_state.rs.

Objective:
Make `yoyo state failures tools` display tool-failure events when they exist in the state log, matching the behavior of `state tail` and `state trace`.

Why this matters:
`state failures tools` was added in the Day 108 15:25 session as a new diagnostic — it's the command users reach for when they suspect tool-call problems. If it reports empty when events exist, it's worse than useless: it actively misleads. This is a regression in a freshly-added feature. The fix also benefits `state failures --recent` (the parent command) since it shares the same `read_events_lenient` path.

Success Criteria:
- `cargo run -- state failures tools` shows tool-failure events when they exist in the state log
- `cargo run -- state tail` continues to work (no regression)
- `cargo run -- state failures --recent` also works if it shares the same root cause
- All existing state tests pass

Verification:
- cargo build
- cargo test --bin yyds -- commands_state --test-threads=1
- Manual: ensure `state failures tools` returns non-empty output on a state log that has events

Expected Evidence:
- Task lineage links src/commands_state.rs change to this task
- Future self-tests show `state failures tools` returning real events
- The "no parseable events" error no longer appears in a state log known to have events

Implementation:
1. Investigate the difference between `read_events_lenient` and `read_events` in src/commands_state.rs:
   - Find both function definitions (search for `fn read_events_lenient` and `fn read_events`)
   - Compare their parsing logic — what does lenient skip that regular reads accept?
2. If `read_events_lenient` has a bug: fix the bug.
3. If the fix is in `read_events_lenient`, both `handle_tool_failures` and `handle_failures` benefit.
4. Alternative approach: change `handle_tool_failures` to use `read_events` instead of `read_events_lenient` if that's the safer fix. But prefer fixing `read_events_lenient` if it serves a real purpose (graceful handling of corrupted lines).
5. The simplest safe fix may be: in `handle_tool_failures`, try `read_events` first; if it fails (file not found), fall back to the current error path. This keeps lenient reading for the main `handle_failures` path if needed.
