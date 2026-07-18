Title: Add SQLite projection staleness detection to state doctor
Files: src/commands_state.rs, src/state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-tests: `state summary` shows 189 events; `state doctor` shows 187,189 events. The SQLite projection is 1000x stale — it hasn't been rebuilt in many sessions.
- `state doctor` currently reports event count from the raw events file, and SQLite store integrity, but never compares the two counts. Any code path querying the SQLite projection (state why, graph commands) sees incomplete data silently.
- The `rebuild_sqlite_projection` function exists in `src/state.rs` (line 852) and is callable from `src/commands_state.rs` (line 18978). The infrastructure exists; the missing piece is detection and reporting.

Edit Surface:
- src/commands_state.rs (handle_doctor: add projection event count query and staleness warning)
- src/state.rs (add a `count_projection_events` helper if one doesn't exist; use existing `query_sqlite_relations` or raw SQL)

Verifier:
- cargo test commands_state state -- --test-threads=1
- cargo build

Fallback:
- If the SQLite projection file doesn't exist (no events projected yet), skip the check with a note "projection not built yet — run `state project --rebuild`".
- If counting projection events is too complex to add cleanly, add at minimum a note in the doctor output: "run `state project --rebuild` to refresh the SQLite projection."
- If the existing `count_projection_events` or equivalent helper already exists, use it directly.

Objective:
Make `state doctor` detect and warn when the SQLite projection event count has drifted >10% from the raw event store count, so operators and the harness can detect silent data corruption before it affects diagnostics.

Why this matters:
The SQLite projection drives `state summary`, `state why`, and graph queries. When it's 1000x stale, these commands silently return wrong results — showing 189 events instead of 187,189. The state doctor is the natural place for this integrity check since it already reports event count and store health separately. Without this check, the drift is invisible unless someone cross-references `state doctor` with `state summary`.

Success Criteria:
- `state doctor` output includes a line comparing raw event count to projection event count.
- When projection count is within 10% of raw count: green "Projection: N events (in sync)".
- When projection count is >10% behind raw count: yellow warning "Projection: N events — stale! Raw store has M events. Run `state project --rebuild`".
- When projection file doesn't exist: "Projection: not built yet".
- The check runs quickly (a single SQL COUNT query, not a full scan).

Verification:
- cargo test commands_state state -- --test-threads=1
- cargo build
- Manual: `cargo run -- state doctor` should show the new projection line.

Expected Evidence:
- State doctor output shows projection staleness warning when drift exists.
- Future dashboard/state summary metrics are accurate because operators are alerted to rebuild.
- No regression in existing state doctor output format.

Implementation Notes:
- In `handle_doctor` (src/commands_state.rs ~line 165): after the existing store integrity and disk usage blocks (~line 311), add a new section that queries the SQLite projection event count.
- Use a simple SQL query against the projection: `SELECT COUNT(*) FROM events` or equivalent. The projection schema is managed by `ensure_projection_schema` in state.rs; check the actual table name.
- If no `count_projection_events` helper exists in state.rs, add a minimal one: `pub fn count_projection_events(sqlite_path: &Path) -> Result<u64, String>` that opens the DB and runs COUNT(*).
- The projection path is available via `sqlite_projection_path(&events_path)` from state.rs.
- Keep the change minimal — one new helper in state.rs (if needed), one new check block in commands_state.rs.
- Do not modify the rebuild logic itself.
