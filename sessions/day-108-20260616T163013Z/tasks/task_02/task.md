Title: Add claim-health diagnostics to state doctor
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Add a claim-health section to `yyds state doctor` that reports which claim types have
missing or incomplete evidence. The trajectory shows 112 non-proven claims (85 missing,
27 observed), dominated by run_lifecycle and model_lifecycle. Showing this gap in the
doctor output makes the recording completeness visible and actionable.

Why this matters:
The trajectory dashboard reports 455/567 claims proven (80.2%), with the largest gaps in
run_lifecycle (missing) and model_lifecycle (observed but unproven). The state doctor
currently shows event counts, run/failure totals, type distribution, and SQLite integrity
— but not claim completeness. When agents run `state doctor` before or after sessions,
they should see which claim types are incomplete so they know what evidence to collect.

Claim-health is a leading indicator of harness recording quality. Missing run_lifecycle
claims suggest runs aren't being stamped with completion events; missing model_lifecycle
claims suggest model calls aren't being fully tracked. Surfacing this in the doctor
output lets the agent diagnose recording gaps before they compound into larger
dashboard/verification gaps.

Success Criteria:
- `yyds state doctor` output includes a "Claims" section after the existing "Events" and
  "Store" sections
- The claims section shows: total claims, proven count, non-proven count (broken into
  missing vs observed), and a list of the top non-proven claim types with their counts
- Falls back gracefully when the SQLite store doesn't exist: "Store not present — claim
  data unavailable"
- Does not break existing `state doctor` behavior
- The new section fits within the existing doctor output style (indented, colored labels)

Verification:
- cargo check
- cargo test --bin yyds -- --test-threads=1
- cargo fmt --check
- Manual: `yyds state doctor` shows claim-health section

Expected Evidence:
- State doctor output now shows claim completeness, enabling agents to identify
  recording gaps
- Dashboard claim-health metrics become more actionable (agents know WHAT to fix)
- Non-proven claim count trends downward in subsequent trajectories as agents address
  gaps surfaced by doctor

Implementation Notes:
- In `handle_doctor()` (`src/commands_state.rs`), after the existing "Events" and "Store"
  sections, add a "Claims" section.
- Query the SQLite projection for claim data. Use the existing `Connection::open` pattern.
- The claims table structure in state.rs's SQLite schema may already have a `claims` or
  similar table. Check the schema in `src/state.rs` for `CREATE TABLE IF NOT EXISTS claims`
  or equivalent. If no dedicated claims table exists, derive claim completeness from
  existing projection tables (failures, run lifecycle events, model call events).
- Minimal implementation: query for run_lifecycle and model_lifecycle claim status,
  report totals. If the schema supports broader claim-type enumeration, show a summary
  table. If not, show the two known gap types explicitly.
- Format: "Claims: N/M proven (K missing, L observed)" followed by a breakdown of
  non-proven types.
- Use color coding: GREEN for proven ratio ≥ 90%, YELLOW for ≥ 70%, RED below 70%.
- If the SQLite store doesn't exist or can't be opened, print a dim note and skip the
  section — don't error out.
