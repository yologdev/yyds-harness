Title: Bound default `state why` event scan to prevent timeout at 121K+ events
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: `yyds state why` (full scan) FAIL — "times out after 10s (121K events)"
- `yyds state why last-failure` PASSES because it uses `DEFAULT_WHY_LIMIT = 10_000`
- Root cause at src/commands_state.rs:109-110: when no `--limit` is given and id != "last-failure", `limit = 0` which `read_tail_events` treats as "scan all 121K events" → timeout
- `BOUNDED_FULL_SCAN_CAP = 100_000` already exists at line 32 but is unused in the `why` code path
- The comment on line 103-104 says: "For specific event IDs (UUID-like), scan the full stream to guarantee the exact event is found." But at 121K events the guarantee is moot — the command doesn't complete.

Edit Surface:
- src/commands_state.rs (line 110: change `0` to `BOUNDED_FULL_SCAN_CAP`)

Verifier:
- cargo build && cargo test --bin yyds -- --test-threads=1

Fallback:
- If `BOUNDED_FULL_SCAN_CAP` at 100K also times out in testing, lower it to 50K. If the approach doesn't fix the timeout, write an obsolete note explaining why and suggest a progressive-scan alternative.

Objective:
Make `yyds state why <event-id>` usable at current event scale (121K+) by capping the default scan to `BOUNDED_FULL_SCAN_CAP` instead of scanning all events unbounded.

Why this matters:
`state why` is the primary diagnostic for investigating specific state events. Right now the command with a specific event ID is broken — it times out before returning any result. The `last-failure` variant proves the bounded approach works. This is a one-line fix that restores a core diagnostic tool.

Success Criteria:
- `yyds state why <recent-event-id>` completes within 10 seconds
- `yyds state why last-failure` continues to work (no regression)
- `yyds state why --limit 50000 <event-id>` still allows larger scans when the user explicitly opts in
- If the event isn't found within the bounded scan, the output should suggest using `--limit` for a deeper search

Verification:
- cargo build && cargo test --bin yyds -- --test-threads=1
- Manual: `yyds state why <event-id-from-recent-session>` completes and shows diagnostic info

Expected Evidence:
- `yyds state why <event-id>` no longer appears as FAIL in assessment self-tests
- State doctor continues to report healthy event stream

Implementation Notes:
- Change line 110 in src/commands_state.rs from `0 // full scan for specific event IDs` to `BOUNDED_FULL_SCAN_CAP`.
- When the event isn't found within the bounded window, the existing "not found" message should already suggest `--limit` — if it doesn't, add a hint: "Event not found in the most recent {BOUNDED_FULL_SCAN_CAP} events. Try: yyds state why --limit 200000 <id> for a deeper scan."
- This is a one-line constant change. No new functions, no restructuring.
- The `DEFAULT_WHY_LIMIT` (10K) and `BOUNDED_FULL_SCAN_CAP` (100K) constants already exist; no new constants needed.
