Title: Extract event IO and formatting utilities from commands_state.rs
Files: src/commands_state.rs, src/commands_state_io.rs
Issue: none
Origin: planner

Objective:
Reduce commands_state.rs by moving ~600 lines of event file IO and formatting utility functions into a new `commands_state_io.rs` module. This is the second extraction task — it targets the IO/formatting layer that sits between the command handlers and the report builders.

Why this matters:
commands_state.rs is 23,839 lines — the single largest file in the codebase. The IO and formatting utilities (event reading/writing, JSONL validation, timestamp formatting, payload summary rendering) are self-contained infrastructure that doesn't need to live in the main module. Extracting them follows the established pattern (`commands_state_crashes.rs`, `commands_state_memory.rs`, `commands_state_graph.rs`) and improves code organization.

These utilities are re-exported as `pub(crate)` symbols used by the extracted submodules (`commands_state_crashes.rs`, `commands_state_graph.rs`, `commands_state_memory.rs`) and by the report builders within `commands_state.rs` itself. Keeping them in a dedicated IO module makes the dependency graph clearer and reduces the main file's size.

Success Criteria:
- commands_state.rs is reduced by at least 400 lines
- New `commands_state_io.rs` contains event reading/writing/formatting utilities
- `cargo build` and `cargo test` pass
- All existing imports from `crate::commands_state::` in extracted submodules continue to work (via re-exports)

Verification:
- cargo build
- cargo test commands_state
- Verify that `commands_state_crashes.rs`, `commands_state_graph.rs`, and `commands_state_memory.rs` still compile (they import from `crate::commands_state::`)
- ./target/debug/yyds state tail --limit 5
- ./target/debug/yyds state crashes
- ./target/debug/yyds state failures --recent

Expected Evidence:
- commands_state.rs line count decreases measurably (combined with task_01)
- Task lineage shows FileEdited events for commands_state.rs and new commands_state_io.rs
- Build and tests pass; no regression in state subcommand behaviors

## Implementation

commands_state.rs contains these IO/formatting utility functions in the range ~lines 975-1600:

**Event file IO:**
- `read_tail` (line 975) — reads last N lines from a JSONL file
- `read_events` (line 987) — reads all events from JSONL (pub(crate))
- `read_limited_events` (line 991)
- `read_tail_events` (line 1002)
- `export_events` (line 1053)
- `import_events` (line 1070)
- `apply_retention_policy` (line 1108)
- `recover_events` (line 1175)
- `validate_event_jsonl` (line 1242)
- `write_normalized_jsonl` (line 1256)
- `write_text_artifact` (line 1268) — pub(crate)

**Timestamp and path utilities:**
- `event_timestamp_ms` (line 1276) — pub(crate)
- `retention_cutoff_ms` (line 1283)
- `recovered_events_path` (line 1289)
- `backup_events_path` (line 1297)
- `default_retention_archive_path` (line 1305)
- `retention_backup_path` (line 1313)
- `current_time_ms` (line 1321) — pub(crate)
- `format_timestamp_ms` (line 1328) — pub(crate)
- `days_to_ymd` (line 1349)

**CLI argument helper:**
- `flag_value` (line 1364) — pub(crate)
- `preview_line` (line 1370) — pub(crate)

**Event formatting/display:**
- `print_event_line` (line 1381)
- `event_line_value` (line 1388)
- `format_event_value` (line 1394)
- `event_payload_summary` (line 1417)
- `compact_payload_summary` (line 1572)
- `tool_arg_summary` (line 1577)

Note: `format_failure_event_value` (line 1606) should NOT be moved — it belongs with the failure report builders extracted in task_01.

Also include at the top of the file:
- `default_events_path` (line 11) — pub(crate), used by many callers
- `default_store_path` (line 19) — pub(crate)

### Extraction steps

1. Create `src/commands_state_io.rs` with module doc comment "Event file IO, formatting, and utility functions — extracted from commands_state.rs."
2. Move the functions listed above (including `default_events_path` and `default_store_path`). Keep function bodies unchanged.
3. In `src/commands_state.rs`, remove the moved functions and add `mod commands_state_io;` near the other `mod` declarations.
4. In `src/commands_state.rs`, add re-exports so existing callers continue to work:
   ```rust
   pub(crate) use commands_state_io::{
       default_events_path, default_store_path, read_events, write_text_artifact,
       event_timestamp_ms, current_time_ms, format_timestamp_ms,
       flag_value, preview_line,
   };
   ```
   This is critical — `commands_state_crashes.rs`, `commands_state_graph.rs`, and `commands_state_memory.rs` import these symbols from `crate::commands_state::`. The re-exports must preserve all `pub(crate)` symbols that those modules depend on.

### Guidelines

- Do this task AFTER task_01 (failure reports extraction) or coordinate carefully — both tasks modify `commands_state.rs`.
- If the implementation agent runs these sequentially, task_01 should complete first so task_02 has the reduced file to work from.
- Do not change function behavior — this is a pure code-move refactor.
- Pay special attention to any `use` statements at the top of `commands_state.rs` that these functions depend on — those imports may need to be duplicated in the new module if they're not already shared.
- The `default_events_path` and `default_store_path` functions at lines 11-25 should move with the IO utilities since they're path helpers.
