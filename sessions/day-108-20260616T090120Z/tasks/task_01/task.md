Title: Add retention health advice to state doctor when stale data accumulates
Files: src/commands_state.rs
Issue: none
Origin: planner

Objective:
When `yyds state doctor` finds 0 events but non-trivial disk usage (≥5 MB for events or store), it should surface a specific actionable recommendation: `state retention --prune` to clean up stale data from prior runs. Currently it only shows a generic "Issues found" message without telling the user what to do.

Why this matters:
The state doctor is the first diagnostic stop for cold-start sessions. The assessment found it shows "Issues found" when events=0 but disk=76MB consumed. A user seeing this has no idea why 76MB is used or how to fix it. Adding a concrete recommendation turns confusion into action. This also aligns with the trajectory's pressure to improve state observability and lifecycle completeness.

Success Criteria:
- `state doctor` with events=0 and events disk ≥ 5MB shows: "Stale event data from prior runs detected (X MB). Run `yyds state retention --prune` to clean up."
- `state doctor` with events=0 and store ≥ 5MB shows a similar recommendation for the SQLite store.
- `state doctor` with active events (events > 0) shows no retention advice — stale detection only triggers on empty state.
- Existing doctor output (Events, Store, Disk, Schema, Health lines) is unchanged.
- `cargo build && cargo test` passes.

Verification:
- cargo build
- cargo test commands_state::tests::doctor
- cargo test commands_state::tests::retention

Expected Evidence:
- Future cold-start assessments can cite the retention recommendation instead of a generic "Issues found".
- State doctor output distinguishes "empty and clean" from "empty with stale data".
- Dashboard health claims show fewer "issues found" false positives for fresh-state sessions.

Implementation Notes:
- The doctor function is in `src/commands_state.rs`. The health status is computed around line 334.
- Parse the disk usage values already displayed at line 225 to detect stale data.
- Add a `stale_data_warnings: Vec<String>` field that accumulates actionable recommendations.
- When events=0 AND disk usage > 5MB threshold, push the retention recommendation.
- When events=0 AND store size > 5MB, push a separate recommendation about SQLite vacuum/rebuild.
- Append these warnings after the "Issues found" line, not before — existing output must not shift.
- Threshold should be a `const STALE_DATA_THRESHOLD_BYTES: u64 = 5 * 1024 * 1024;`.
- Use `{YELLOW}` for warning text to distinguish from red errors.
- Add a test in the existing doctor test module that verifies stale-data warnings appear when events=0 and disk is above threshold, and don't appear when events > 0.
