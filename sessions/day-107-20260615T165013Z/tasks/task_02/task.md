Title: Add lifecycle pairing test and harden ModelCallCompleted fallback
Files: src/prompt.rs, src/state.rs
Issue: none
Origin: planner

Objective:
Add a focused test that verifies ModelCallStarted/ModelCallCompleted pairing works correctly in the lifecycle JSON builder, and harden the post-loop fallback in prompt.rs so no ModelCallStarted event can be left unmatched.

Why this matters:
The trajectory reports `deepseek_model_call_incomplete_count=22` with `model_incomplete/open_after_file_edit=5` as the most common gap type. The post-loop fallback in prompt.rs (line 1045) already exists but 22 gaps persist — suggesting either the fallback was added recently (historical gaps), there's a code path that exits before reaching it, or the lifecycle key pairing in `build_state_lifecycle_json` has a mismatch. This is the #1 graph pressure: "Close yyds state and model lifecycle gaps."

Adding a test that exercises the lifecycle JSON builder with synthetic paired events will catch any key-matching bugs. Hardening the fallback ensures future sessions don't accumulate new gaps.

Success Criteria:
- A new unit test in src/prompt.rs (or src/state.rs) creates synthetic ModelCallStarted + ModelCallCompleted events and verifies `build_state_lifecycle_json` pairs them correctly (run_completed=1, model_completed=1, no unmatched completions).
- The test also verifies that an unpaired ModelCallStarted (no matching ModelCallCompleted) produces `open_model_calls > 0` in the lifecycle output.
- The post-loop fallback at prompt.rs:1045 is reviewed for edge cases (panic unwinding, early return) and hardened if any gap exists.
- `cargo build && cargo test --bin yyds -- --test-threads=1` passes.

Verification:
- cargo check
- cargo test --bin yyds -- --test-threads=1
- cargo test --test integration -- --test-threads=1

Expected Evidence:
- Test output shows lifecycle pairing assertions pass
- Future trajectory reports show decreasing `model_incomplete` count (or at least no NEW gaps from this session forward)
- Dashboard artifacts show the test exists in the test suite

Implementation Notes:
- The `build_state_lifecycle_json` function in src/commands_state.rs (line 2444) is the lifecycle pairer. It uses `model_lifecycle_key` (line 2581) which matches on `run_id`. The test should verify this matching works.
- For the test, you can construct `serde_json::Value` entries that match the schema produced by `StateRecorder::append` (src/state.rs line 276): each event has `event_type`, `run_id`, `event_id`, `payload`, `timestamp_ms`, `actor`, etc.
- The post-loop fallback (prompt.rs:1045-1059) already records ModelCallCompleted with status "stream_closed_without_agent_end" when `model_call_terminal_recorded` is false. To harden:
  1. Verify there's no early `return` between the loop exit and the fallback.
  2. Check if `model_call_terminal_recorded` is properly initialized to `false` before the loop.
  3. Consider if a `Drop` guard on a wrapper struct would be more reliable than the post-loop check (but keep scope minimal — a `Drop` guard is a bigger change).
- Keep the test focused: one test for lifecycle pairing, one for the unpaired-started case.
- Do NOT refactor `build_state_lifecycle_json` or change its output format — only add tests and harden the recording side.
