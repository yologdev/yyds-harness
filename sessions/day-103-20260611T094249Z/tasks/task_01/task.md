Title: Wire crash diagnostics into agent-run exit paths in lib.rs
Files: src/lib.rs
Issue: none
Origin: planner

Objective:
Ensure every agent-run crash captured by `state crashes` carries a diagnostic message,
so future sessions can diagnose failures without guessing. Currently 10 of 10 crash events
show `no` diagnostic key because `exit_with_state(code)` is called without first calling
`stash_diagnostic_error()`.

Why this matters:
State evidence: `yyds state crashes` shows 10 crashes, all with `no` diagnostic key.
These are agent startup/run failures that exit via `exit_with_state()` without stashing
the error message. When the harness can't see why a session crashed, it can't learn from
the failure — the same crash repeats across sessions. Closing this gap makes every crash
diagnosable, which feeds directly into harness self-evolution quality.

Success Criteria:
- `state crashes` on the next failed run shows at least some crashes with non-`no` diagnostic keys
- Every `exit_with_state(1)` and `exit_with_state(2)` in the piped/agent execution paths is preceded by `stash_diagnostic_error` with the error context that was already being printed to stderr

Verification:
- cargo build && cargo test
- grep 'exit_with_state' src/lib.rs to confirm each call has diagnostic stashing nearby

Expected Evidence:
- After a failed agent run, `state crashes` should show diagnostic keys like "api_error", "model_error", "provider_error" instead of "no"
- Crash events in state JSONL gain payload.error fields

Description:

The function `exit_with_state(code)` at src/lib.rs:972 calls `state::mark_run_completed_with_error` but does NOT call `state::stash_diagnostic_error`. The exit paths (lines 459, 479, 589, 595, 643, 691, 721, 726, 738, 754, 858, 861) all print error messages to stderr via `eprintln!` but those messages are lost to the state recording layer.

Add `state::stash_diagnostic_error(...)` calls before each `exit_with_state` in the agent execution paths. The error message is already available — it's the string being printed to stderr on the line right above the exit call. Convert the printed error into a stashed diagnostic:

Before:
```rust
eprintln!("{RED}  error: {e}{RESET}");
exit_with_state(1);
```

After:
```rust
eprintln!("{RED}  error: {e}{RESET}");
state::stash_diagnostic_error(&format!("agent run: {e}"));
exit_with_state(1);
```

For API failure paths (lines 471-480, 586-595), stash the specific error kind:
```rust
state::stash_diagnostic_error(&format!("api_error: {error_detail}"));
```

For the empty-input exit (line 691), use "empty_input" as the key.

For CHECKPOINT_TRIGGERED exit (line 691), use "checkpoint_triggered".

This task only touches `src/lib.rs`. Keep changes minimal — just insert `stash_diagnostic_error` calls before existing `exit_with_state` calls, reusing the error text already being printed.
