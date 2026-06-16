Title: Deduplicate incomplete run IDs in state why last-failure display
Files: src/commands_state.rs
Issue: none
Origin: planner

Objective:
Prevent the same run ID from appearing multiple times in the incomplete-runs
list shown by `yyds state why last-failure` when no failure is found.

Why this matters:
The assessment noted that `state why last-failure` shows duplicate run IDs in
the incomplete runs list (e.g., `github-actions-27593785970` appearing twice).
This is confusing — it suggests two incomplete runs when there's only one.
The duplicate comes from `find_incomplete_runs()` finding the same run ID in
multiple session state records without deduplicating.

The harness's state legibility depends on clear, honest reporting. Duplicate
run IDs erode trust in state diagnostics.

Success Criteria:
- Each run ID appears at most once in the incomplete runs list
- If the same run appears in multiple sessions, only the most recent start
  timestamp is shown
- `cargo build && cargo test` passes

Verification:
- cargo build
- cargo test --lib commands_state
- Manual smoke: `yyds state why last-failure` (if events.jsonl has incomplete runs)

Expected Evidence:
- Assessment logs no longer report duplicate run IDs
- State diagnostics are cleaner and more trustworthy

Implementation:
In `src/commands_state.rs`, the `find_incomplete_runs` function (line 940) currently:

```rust
fn find_incomplete_runs(events: &[Value]) -> Vec<(String, u64)> {
    let completed: BTreeSet<&str> = events
        .iter()
        .filter(|e| event_string(e, "event_type") == Some("RunCompleted"))
        .filter_map(|e| event_string(e, "run_id"))
        .collect();
    let mut incomplete: BTreeMap<String, u64> = BTreeMap::new();
    for e in events {
        let event_type = event_string(e, "event_type");
        if event_type == Some("RunStarted") {
            if let Some(run_id) = event_string(e, "run_id") {
                if !completed.contains(run_id.as_str()) {
                    let ts = e.get("timestamp_ms").and_then(|v| v.as_u64()).unwrap_or(0);
                    // Keep the most recent (largest) timestamp for each run_id
                    incomplete
                        .entry(run_id)
                        .and_modify(|existing| {
                            if ts > *existing {
                                *existing = ts;
                            }
                        })
                        .or_insert(ts);
                }
            }
        }
    }
    let mut result: Vec<(String, u64)> = incomplete.into_iter().collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}
```

Key changes:
1. Use `BTreeMap<String, u64>` instead of pushing to `Vec<(String, u64)>` — the
   map deduplicates by run_id and keeps the latest timestamp
2. Sort by timestamp descending before returning
3. Import `BTreeMap` if not already imported (check existing imports)

This is a low-risk change that only affects the display path of `handle_why`.
