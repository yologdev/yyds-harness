Title: Fix `state failures --recent` returning "no parseable events" on valid state log
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner (refined from harness-seed)

Objective:
Make `yyds state failures --recent` return actual failure reports instead of "no parseable events found" when the state log contains valid events.

Why this matters:
Assessment self-test revealed: `state tail --limit 20` shows live events ✓ but `state failures --recent` reports "no parseable events found at .yoyo/state/events.jsonl". The root cause is a divergence in how the two commands consume the log:

- `state tail` → `read_tail()` → raw string lines (no parsing) — always works.
- `state failures --recent` → `read_tail_events(limit=0)` → `read_events_lenient()` → `compatibility_event_json_line()` → `parse_state_event_line()` — strict schema parsing that can reject all lines.

`read_events_lenient` (line 1044) reads the entire file, runs each line through `compatibility_event_json_line` (which calls `parse_state_event_line`), and collects only successes. If all lines fail `parse_state_event_line`, the result is an empty Vec, and `handle_failures` at line 786 prints "no parseable events found."

The Day 108 (12:54) fix added `read_events_lenient` as the lenient path, but it doesn't report *how many* lines were skipped vs parsed, making the failure silent. The fix needs to add skip-count diagnostics so the command can report "X events parsed, Y lines skipped" instead of silently returning empty.

Success Criteria:
- `yyds state failures --recent` produces a report (or a "no failures in recent events" message) when events.jsonl contains parseable events.
- When some lines are unparseable, the command still processes the valid ones and reports skip count.
- The `read_events_lenient` function returns both parsed events and a skip count (or at minimum, diagnostics are available at the call site).
- `state tail` behavior is unchanged.

Verification:
- cargo test commands_state:: -- --test-threads=1
- cargo check

Expected Evidence:
- Future assessments show `state failures --recent` returning actual failure data (or "no failures found" for clean runs) instead of "no parseable events."
- State/dashboard tool-failure reconciliation becomes possible because the command pipeline works end-to-end.

Implementation Notes:
- The fix should be minimal: add a skip counter to `read_events_lenient` (return `(Vec<Value>, usize)` or add skip diagnostics to the call site).
- If `parse_state_event_line` is rejecting lines that should be valid, also fix that specific rejection — but start with diagnostics first so the root cause is visible.
- The `compatibility_event_json_line` function in `src/state.rs` (line 3150) calls `parse_state_event_line` which requires `event_id`, `event_type`, etc. If the events.jsonl contains raw events without these exact fields, they'll be skipped. Consider whether `read_events_lenient` should fall back to raw `serde_json::from_str` without the compatibility normalization.
- Do NOT change `read_tail` or `handle_tail` — they work correctly and serve a different purpose (raw line display).
- Verify against the live events.jsonl at `.yoyo/state/events.jsonl` if it exists.
