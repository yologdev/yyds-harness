Title: Extract failure/policy/rollback report builders from commands_state.rs
Files: src/commands_state.rs, src/commands_state_reports.rs
Issue: none
Origin: planner

Objective:
Reduce commands_state.rs by moving ~650 lines of failure analysis, policy report, and rollback report builder functions into a new `commands_state_reports.rs` module, following the same extraction pattern as `commands_state_crashes.rs` and `commands_state_memory.rs`.

Why this matters:
commands_state.rs is 23,839 lines — the single largest file in the codebase. The crash, graph, and memory submodules were already extracted, but the core file still carries failure/trace/rollback/policy report building logic that should live in a focused module. This extraction improves compile-time parallelism, reduces the cognitive load of navigating the file, and follows the established architectural pattern.

This was flagged in the Day 107 assessment as the highest-value code organization improvement: "commands_state.rs at 23,839 lines is the single largest file [...] The core file still contains graph reporting, state summary, event tail, lifecycle, and evaluation commands."

Success Criteria:
- commands_state.rs is reduced by at least 500 lines
- New `commands_state_reports.rs` contains failure/trace/rollback/policy report builder functions
- `cargo build` and `cargo test` pass
- All existing state subcommand behaviors are preserved: `state failures --recent`, `state fixes --recent`, `state rollbacks --recent`, `state policies --recent`, `state trace <id>`

Verification:
- cargo build
- cargo test commands_state
- ./target/debug/yyds state failures --recent
- ./target/debug/yyds state fixes --recent
- ./target/debug/yyds state rollbacks --recent
- ./target/debug/yyds state policies --recent

Expected Evidence:
- commands_state.rs line count decreases measurably
- Task lineage shows FileEdited events for commands_state.rs and new commands_state_reports.rs
- Build and tests pass; no regression in state subcommand behaviors

## Implementation

commands_state.rs contains these functions in the range ~lines 1600-2280 that belong together:

**Failure classification and reporting:**
- `format_failure_event_value` (line 1606)
- `build_trace_report` (line 1617)
- `build_recent_failure_report` (line 1692)
- `classify_failure_event` (line 1739)
- `classify_failure_payload` (line 1804)
- `taxonomy`, `payload_has_source`, `payload_mentions` (line 1811+)
- `failure_payload_text`, `collect_payload_text` (line 1834+)

**Policy and fix reporting:**
- `build_policy_report` (line 1860)
- `build_failure_fix_report` (line 1929)

**Rollback reporting:**
- `build_rollback_rows` (line 2072)
- `build_rollback_report` (line 2165)
- `format_rollback_row` (line 2176)
- `build_rollback_payload` (line 2217)
- `rollback_row_payload` (line 2237)

**Shared utilities:**
- `eval_failed` (line 2282)
- `eval_status_label` (line 2301)
- `FailureTaxonomy` struct (defined near line 1739)
- `RollbackReportRow` struct (defined near line 2072)

### Extraction steps

1. Create `src/commands_state_reports.rs` with module doc comment "Failure/policy/rollback report builders — extracted from commands_state.rs."
2. Add appropriate imports at the top of the new file. Most functions use `serde_json::Value`, `crate::format::*`, and std types. Some may need `crate::commands_state::` imports for shared utilities like `read_events`, `event_string`, etc.
3. Move the struct definitions (`FailureTaxonomy`, `RollbackReportRow`) and all the functions listed above. Keep function bodies unchanged except for any necessary `crate::` qualifier adjustments on types that are now in a different module.
4. In `src/commands_state.rs`, remove the moved functions and structs.
5. In `src/commands_state.rs`, add `mod commands_state_reports;` near the other `mod` declarations at the top (after existing module declarations like `pub(crate) mod commands_state_crashes;`).
6. In `src/commands_state.rs`, re-export `handlers` from the new module by adding `pub(crate) use commands_state_reports::*;` or specific use statements so the handler functions (`handle_failures`, `handle_fixes`, `handle_rollbacks`, `handle_policies`) in commands_state.rs can still call `build_recent_failure_report`, `build_policy_report`, `build_rollback_report`, etc.
7. If any moved functions are `pub(crate)` and called from outside commands_state.rs (e.g., from commands_state_graph.rs), make sure the re-exports or direct `use` paths work.

### Guidelines

- Follow the pattern from `commands_state_crashes.rs`: the extracted module imports shared utilities from `crate::commands_state::{...}`.
- Do not change function behavior — this is a pure code-move refactor.
- If functions from the IO/formatting range (lines 975-1606) are called by the failure report builders, do NOT move those IO functions — this task only moves the failure/policy/rollback report builders. The IO functions will be extracted separately.
- If `build_recent_failure_report` calls `classify_failure_event` which calls `classify_failure_payload`, move all three together.
