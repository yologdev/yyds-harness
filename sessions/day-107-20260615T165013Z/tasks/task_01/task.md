Title: Diagnose and fix state lifecycle all-zeros from fresh events
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner (refined from harness-seed, validated_against_assessment=true)

Objective:
Make `yyds state lifecycle --limit 5` show actual run/model-call counts from the live events log instead of returning all zeros.

Why this matters:
The assessment shows `state lifecycle --limit 5` reports 0 runs started, 0 completed, 0 model calls — even when run/model events exist in the events file. The trajectory reports 22 incomplete model calls and 3 incomplete state runs, but the lifecycle command can't surface any of this data. Cold-start `state why last-failure` already works (Day 107 earlier sessions fixed it), but lifecycle querying is still broken. This is the #1 graph pressure: "Close yyds state and model lifecycle gaps."

The primitive `build_state_lifecycle_json` in src/commands_state.rs (line 2444) reads events and counts RunStarted/RunCompleted/ModelCallStarted/ModelCallCompleted — but the `read_limited_events` path or the `default_events_path()` may not point to the live events file, or there's a schema mismatch in event serialization.

Success Criteria:
- `yyds state lifecycle --limit 5` shows >0 runs and >0 model calls when run from a session that has recorded ModelCallStarted and RunStarted events.
- `yyds state lifecycle --limit 5 --json` outputs valid JSON with real counts.
- Existing lifecycle report formatting (run completeness, model call pairing) works correctly with real data.
- Cargo check passes; no regression in `cargo test --bin yyds -- --test-threads=1`.

Verification:
- cargo check
- cargo test --bin yyds -- --test-threads=1
- Manual: `./target/debug/yyds state lifecycle --limit 20` after running any prompt (which records ModelCallStarted)

Expected Evidence:
- lifecycle command output changes from all-zeros to real counts
- task artifacts show a test that records synthetic lifecycle events and verifies the lifecycle command reads them back
- future trajectory reports show reduced lifecycle gap counts

Implementation Notes:
- The seed task originally targeted cold-start `why last-failure` diagnostics. That was fixed in an earlier Day 107 session (assessment confirms it works). This refinement pivots to the remaining lifecycle gap: the `state lifecycle` command itself.
- Root cause candidates (investigate in order):
  1. `default_events_path()` may return a different path than where events are actually written. Compare with the path used by `init_global`/`StateRecorder::new`.
  2. `read_limited_events` may have a filtering or parsing bug that drops lifecycle events.
  3. The event JSON serialization may use different field names than what `build_state_lifecycle_json` expects (it matches on "RunStarted", "RunCompleted", "ModelCallStarted", "ModelCallCompleted" as strings).
- Add a unit test in src/commands_state.rs (or src/state.rs) that:
  1. Creates a temp events file
  2. Writes synthetic RunStarted, ModelCallStarted, ModelCallCompleted, RunCompleted events
  3. Calls `read_limited_events` + `build_state_lifecycle_json`
  4. Asserts run_started=1, run_completed=1, model_started=1, model_completed=1
- If the root cause is in `default_events_path()` returning the wrong path, fix it there. If it's in event parsing, fix the field name matching.
- Keep changes minimal — the harness has been iterating on lifecycle all session; avoid broad refactors.
