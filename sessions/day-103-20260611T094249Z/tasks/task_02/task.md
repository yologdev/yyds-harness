Title: Extract state memory synthesis from commands_state.rs
Files: src/commands_state.rs, src/commands_state_memory.rs (new), src/lib.rs
Issue: none
Origin: planner

Objective:
Extract the state memory synthesis subsystem (~435 lines, lines 12553–12987) from the
monolithic `commands_state.rs` (23,648 lines) into a dedicated `commands_state_memory.rs`
module. This reduces structural debt and makes state memory logic independently testable.

Why this matters:
`commands_state.rs` is 17% of the codebase (23,648 lines, 451 items) — the most acute
structural debt in the project. Day 103 already extracted `commands_state_crashes.rs`
(209 lines). Continuing this module-by-module extraction is the right cadence: one
cohesive subsystem per session, always independently verifiable.

The memory synthesis subsystem is a natural extraction target: it has clear boundaries
(five functions + one struct + one helper), no cross-dependencies with unrelated parts
of `commands_state.rs`, and a single public entry point (`build_state_memory_synthesis`).

Success Criteria:
- `cargo build` passes with no errors
- `cargo test` passes (89 tests)
- `src/commands_state_memory.rs` exists with all memory synthesis code
- `src/commands_state.rs` is ~435 lines shorter
- `src/lib.rs` has `pub(crate) mod commands_state_memory;` added

Verification:
- cargo build && cargo test
- grep -c 'fn build_state_memory_synthesis' src/commands_state.rs (should be 0)
- grep -c 'fn build_state_memory_synthesis' src/commands_state_memory.rs (should be 1)

Expected Evidence:
- Task lineage shows `src/commands_state_memory.rs` created, `src/commands_state.rs` modified, `src/lib.rs` modified
- `src/commands_state.rs` line count decreases by ~435
- Dashboard file stats show one fewer oversized file

Description:

Extract lines 12553–12987 from `src/commands_state.rs` into a new file `src/commands_state_memory.rs`. The extraction includes:

1. `pub(crate) fn build_state_memory_synthesis(events: &[Value]) -> Result<String, String>` (line 12553)
2. `pub(crate) struct StateMemoryRecord { ... }` (line 12725)
3. `pub(crate) fn record_state_memory_candidates(...)` (line 12827)
4. `pub(crate) fn build_state_memory_records(events: &[Value]) -> Vec<StateMemoryRecord>` (line 12864)
5. `pub(crate) fn record_state_memory_decision(...)` (line 12923)
6. `fn memory_candidate_id(kind: &str, key: &str) -> String` (line 12969) — private helper

Steps:
1. Create `src/commands_state_memory.rs` with the necessary `use` imports:
   - `use serde_json::Value;`
   - `use std::path::Path;`
   - `use crate::state::*;` (for StateRecorder, StateConfig, EventType, Actor, current_time_ms)
   - `use crate::commands_state::*;` (for event_string, default_store_path, format_timestamp_ms, human_bytes)

2. Copy the six items (functions + struct) from `src/commands_state.rs` into the new file.

3. Add `pub(crate) mod commands_state_memory;` to `src/lib.rs` near the other `commands_state_*` module declarations.

4. Delete the six items from `src/commands_state.rs`.

Keep the extracted code identical — no refactoring, renaming, or reformatting. This is a pure move.
