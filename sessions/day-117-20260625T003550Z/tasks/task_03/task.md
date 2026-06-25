Title: Add event scanning limit to `state doctor` to prevent timeout with 50k+ events
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: `yyds state doctor` timed out at 30s
- Assessment: "With 50,841 total events, a full scan with relation computation could exceed 30s"
- `handle_doctor()` at line 137 reads ALL events via `read_events()` then iterates — O(n) on 50k+ JSON objects
- This blocks diagnostic reliability: can't quickly check state health during assessment

Edit Surface:
- src/commands_state.rs — modify `handle_doctor()` to accept an optional `--limit` flag or cap event loading

Verifier:
- cargo build && yyds state doctor --limit 5000 (should complete in <5s)
- yyds state doctor (default, should complete in <30s with capped scan)

Fallback:
- If the full event scan is architecturally required (e.g., type counts must be exact), add a streaming pass that batches in chunks with progress output, or write a blocked note explaining why limits can't work.

Objective:
Make `yyds state doctor` complete in under 10 seconds regardless of event count, so it's usable during assessment phases.

Why this matters:
State doctor is the primary diagnostic tool for assessing harness health during evolution sessions. When it times out, the assessment loses event type distribution, run/failure counts, and SQLite integrity checks. This directly impacts the planner's ability to detect state-level problems before selecting tasks, and blocks fitness measurement gated on state health visibility.

Success Criteria:
- `yyds state doctor` completes in under 10s with 50k+ events
- `yyds state doctor --limit N` limits event scanning to N most recent events
- Default (no flag) uses a reasonable cap (e.g., 20,000 events) or streams without full load
- Type counts and run/failure counts show "(sampled from last N events)" when capped

Verification:
- cargo build
- yyds state doctor --limit 1000  # should be near-instant
- yyds state doctor               # should complete in <10s
- Compare output with and without --limit to verify sampling is clearly indicated

Expected Evidence:
- Future assessment self-tests show `yyds state doctor` completing in seconds
- No more state doctor timeouts in CI assessment phases
- Dashboard state health widget loads without timeout

Implementation Notes:
- Keep the change minimal. The `handle_doctor()` function already prints type counts and run/failure tallies.
- Option A (preferred): Add a `--limit` CLI flag to the `state doctor` subcommand, read the most recent N events from the events file (read from end, parse JSON lines in reverse).
- Option B: Stream the events file line-by-line instead of loading all at once — but this requires changing `read_events()` or adding a streaming variant.
- The events file is JSONL (one JSON object per line), so tail-based reading is straightforward: seek to end, read backwards to find N line boundaries, parse those lines.
- If the events file is too large to seek backwards efficiently, fall back to streaming from the beginning with a counter.
- Indicate when counts are sampled: "Events: 50841 total (showing last 20000)" or similar.
