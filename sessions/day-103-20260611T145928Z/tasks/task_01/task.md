Title: Wire crash reporter into sub-agent dispatch failures
Files: src/commands_spawn.rs
Issue: none
Origin: planner (refined from harness-seed task_01)

Objective:
Make sub-agent dispatch failures emit diagnostic events via `stash_diagnostic_error` so the state system can distinguish sub-agent build failures, spawn errors, and execution timeouts from other failure classes.

Why this matters:
The crash reporter is now wired into 5+ failure paths (bash tool, MCP, agent build, agent run, transport) but `commands_spawn.rs` — the primary sub-agent dispatch path — has zero `stash_diagnostic_error` calls. Sub-agent failures are invisible in the state graph. The assessment explicitly lists "sub-agent dispatch failures" as an uncovered gap. This is the last major agent-orchestration path without crash diagnostics.

Success Criteria:
- `handle_spawn_fg()` emits a diagnostic when `sub_config.build_agent()` fails
- `handle_spawn_fg()` emits a diagnostic when the sub-agent's `run_prompt()` returns an error
- `handle_spawn_bg()` emits a diagnostic when background spawn setup fails
- Existing spawn behavior is unchanged for successful paths

Verification:
- cargo check
- cargo test commands_spawn
- cargo test -- --test-threads=1

Expected Evidence:
- Task lineage links `src/commands_spawn.rs` to this task
- Future state events show `spawn_build_failed`, `spawn_run_failed`, or `spawn_bg_failed` diagnostic kinds
- `/state crashes` can distinguish spawn failures from other diagnostic classes

Implementation Notes:
- The crash reporter function is `crate::state::stash_diagnostic_error(msg: &str)` in `src/state.rs`
- `handle_spawn_fg()` at ~line 528 calls `sub_config.build_agent()` and then `run_prompt()`. Both have failure paths.
- `handle_spawn_bg()` at ~line 561 registers a background task — wrap its error paths.
- Use descriptive prefixes: `"spawn: build_agent failed: {e}"`, `"spawn: run_prompt error: {e}"`, `"spawn: bg dispatch failed: {e}"`
- Existing pattern from agent_builder.rs: `crate::state::stash_diagnostic_error(&format!("mcp_preflight: {command}: {e}"));`
- Do not change logic — just add diagnostic calls before existing error returns
