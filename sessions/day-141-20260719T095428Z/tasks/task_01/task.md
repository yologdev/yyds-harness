Title: Fix SQLite projection rebuild to skip unknown event types instead of failing
Files: src/state.rs
Issue: #122
Origin: planner

Evidence:
- `cargo run -- state doctor` output (Day 141 09:54): "Projection: 34 events — stale! Raw store has 188859 events. Run `state project --rebuild`"
- This means 0.018% of event history is in the projection. All state graph queries, hotness lookups, trajectory computation, and dashboard projections operate on ~34 events instead of 188k.
- Agent-self issue #122: prior attempt reverted by evaluator timeout (not code failure), implementation notes already validated.
- The fix is known and small: ~10 lines in src/state.rs changing `.map_err()` to skip+count in `rebuild_sqlite_projection`.

Edit Surface:
- src/state.rs

Verifier:
- cargo test state -- --test-threads=1
- cargo run -- state project --rebuild 2>&1 | head -10
- cargo run -- state doctor 2>&1 | head -10

Fallback:
- If serde deserialization is the better fix point (adding `#[serde(other)]` to EventType enum instead of skipping in rebuild), do that instead, but prefer the skip approach because it's more defensive.
- If the rebuild succeeds but takes >60s, note it and consider batching in follow-up — do not scope-creep.
- If state.rs has changed significantly since the issue was filed, re-verify the exact line numbers before editing.

Objective:
Make `state project --rebuild` succeed on the current events.jsonl (188k+ lines, containing unknown event types like `TestEvent`), restoring full state query accuracy.

Why this matters:
All state graph queries, hotness lookups, trajectory computation, and dashboard projections are operating on 34 of 188,859 events. Every planning session runs with degraded evidence. The state doctor can detect the staleness but can't fix it. This is the highest-priority gate because it blocks all other state-driven work — including assessment quality, task selection, and gnome measurement.

Success Criteria:
- `state project --rebuild` completes without error on the current events.jsonl (188k+ lines)
- `state doctor` shows projection event count approximately equal to raw event count
- Unknown event types are skipped with a stderr warning
- `ProjectionReport` includes a `skipped_unknown` count
- `cargo test state` passes

Verification:
- cargo test state -- --test-threads=1
- cargo build
- cargo run -- state project --rebuild 2>&1 | head -5
- cargo run -- state doctor 2>&1 | head -10

Expected Evidence:
- `state doctor` shows projection matching raw count (not 34 vs 188,859)
- `state graph hotspots` returns results from full event history
- Future trajectory snapshots have accurate state-derived metrics
- No `TestEvent`-class failures on subsequent rebuilds

Implementation Notes:
The fix is in `rebuild_sqlite_projection` in src/state.rs. Change the loop from `.map_err()?` (fail-fast) to `match` with warn+skip+continue:

BEFORE (approximate, verify line numbers):
```rust
for (idx, line) in raw.lines().enumerate() {
    let event = parse_state_event_line(line)
        .map_err(|e| format!("parse event line {}: {e}", idx + 1))?;
    project_event_with_conn(&tx, &event, &mut report)?;
}
```

AFTER:
```rust
for (idx, line) in raw.lines().enumerate() {
    let event = match parse_state_event_line(line) {
        Ok(event) => event,
        Err(e) => {
            eprintln!("warning: skipping event line {}: {e}", idx + 1);
            report.skipped_unknown += 1;
            continue;
        }
    };
    project_event_with_conn(&tx, &event, &mut report)?;
}
```

Also add `pub skipped_unknown: usize` to `ProjectionReport` struct and initialize it to 0 (update Default impl or explicit init). Keep the change minimal — one field, one loop change, ~10 lines total.
