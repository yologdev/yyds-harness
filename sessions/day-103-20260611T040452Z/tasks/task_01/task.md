# Task 01: Wire crash reporter into pre-agent bootstrap path

Title: Wire crash reporter into pre-agent bootstrap path
Files: src/lib.rs
Issue: none
Origin: planner

## Objective

Move `record_session_started()` to fire BEFORE `build_agent()` so that pre-agent crashes (API key missing, config errors, agent construction failures) leave a `SessionStarted` diagnostic breadcrumb. Add `stash_diagnostic_error()` calls to early-exit paths in the bootstrap that currently exit without diagnostic info, so the `RunCompleted(status=error)` event carries the actual failure reason instead of just "exit code 1".

## Why this matters

The state crash report shows 10+ crashes in the last 49 seconds, all `key? no` — meaning the crash reporter never fired. These are pre-agent crashes happening in the harness bootstrap layer before `state::record_session_started()` is called (currently at line ~1120, AFTER `build_agent()` at ~1094). The `stash_diagnostic_error()` mechanism is already wired into state init failure (line 1032) and transport failures (deepseek.rs:1022), but bootstrap paths between `parse_args()` and `build_agent()` exit silently.

This is the #1 state-observability gap from the Day 103 assessment.

## Success Criteria

- `state::record_session_started()` fires before `agent_config.build_agent()` (line ~1094)
- The early `return` at the `apply_config_flags` failure path (line ~991) calls `stash_diagnostic_error()` before returning
- Any other bootstrap `exit_with_state(1)` calls between state init and agent construction also call `stash_diagnostic_error()` first
- `cargo build` passes, `cargo test` passes
- After this change, crashed runs with missing API keys show `key? yes` in the session started event (because the event fires before the agent tries to connect) AND the `RunCompleted` error payload includes a diagnostic message

## Verification

```
cargo build
cargo test -- --test-threads=1
./target/debug/yyds state crashes --limit 10
```

## Expected Evidence

- Task lineage shows changed file: `src/lib.rs`
- State events after the change: `SessionStarted` events appear for sessions that later crash during agent build, with `api_key_present: true` (the key exists but the agent can't connect) or `api_key_present: false` (the key was never set)
- `RunCompleted(status=error)` events include `error_detail` field with the diagnostic message

## Implementation Details

### Part A: Move record_session_started() earlier

Currently in `src/lib.rs`, the order is:
1. `state::init_global()` — line 994
2. `state::RunCompletionGuard` — line 1035
3. `state::install_panic_hook()` — line 1037
4. config extraction and agent building — lines 1047-1094
5. `connect_external_servers()` — line 1101
6. `state::record_session_started()` — line 1120 ← TOO LATE

New order should be:
1. `state::init_global()` — keep here
2. `state::RunCompletionGuard` — keep here
3. `state::install_panic_hook()` — keep here
4. **`state::record_session_started()` — MOVE HERE** (before agent build)
5. config extraction and agent building
6. `connect_external_servers()`

Wait — `record_session_started()` currently uses `agent_config.model`, `agent_config.skills.len()`, and `effective_context_tokens()`. Check that all of these are available BEFORE `build_agent()`. They should be — `agent_config` is built at line 1061-1086, before `build_agent()` at line 1094.

The `DEEPSEEK_API_KEY` env var check (`std::env::var("DEEPSEEK_API_KEY").is_ok()`) is also available at any point.

So move `record_session_started()` to right after `install_panic_hook()` at line 1037, but after `agent_config` is constructed (which happens at lines 1061-1086). The correct insertion point is between line 1088 (after `run_setup_wizard_if_needed`) and line 1094 (`build_agent()`).

### Part B: Add stash_diagnostic_error() to apply_config_flags failure

At line 990-992:
```rust
if !apply_config_flags(&config) {
    return;
}
```
This exits without any diagnostic trace. Add:
```rust
if !apply_config_flags(&config) {
    state::stash_diagnostic_error("config: apply_config_flags returned false");
    return;
}
```

### Part C: Audit other early exits

After moving `record_session_started()` earlier, audit every `exit_with_state(1)` and early `return` between state init and agent construction. Each one should call `stash_diagnostic_error()` before exiting.

Check specifically:
- `run_setup_wizard_if_needed` returning false (line 1088-1090) — does it need diagnostic stashing? It prints its own error message and returns. Add `stash_diagnostic_error` for consistency.
- Any other error paths visible in the function.

### Part D: Verify the `connect_external_servers` failure path

After `build_agent()` succeeds, `connect_external_servers()` is called. If it fails, does it leave a diagnostic error? Check whether `connect_external_servers` panics or returns an error that's caught. If it can fail silently, add `stash_diagnostic_error` before the exit.
