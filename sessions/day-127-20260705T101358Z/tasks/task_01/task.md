Title: Add per-command timeout to eval infrastructure (fixture runner + gates)
Files: src/eval_fixtures.rs, src/commands_eval.rs
Issue: #67
Origin: planner

Evidence:
- Day 127 Task 2 was auto-reverted after the evaluator agent timed out waiting for the
  fixture command `yyds state lifecycle --limit 1000` to complete. The evaluator's bash
  tool (300s default) hit its wall, the agent produced no Verdict: PASS/FAIL, and the
  harness auto-reverted the task — wasting a full task slot.
- `run_fixture_command` (src/eval_fixtures.rs:625) and `run_gate` (src/commands_eval.rs:987)
  both call `Command::output()` which blocks indefinitely with no timeout. If a gate or
  fixture command hangs or takes too long, there is no escape — the calling process (eval
  agent) either times out externally or runs until killed.
- Trajectory: task_verification_rate=0.5 — the Day 127 revert is the direct cause. Fixing
  the timeout path converts "timed out without verdict → auto-revert" into "timed out →
  failure verdict → counted as a real attempt."
- Historical: every fixture that runs a state-scanning command (`yyds state lifecycle`,
  `yyds state tail --limit N`) is vulnerable to the same hang. The event log grows over
  time, so commands that were fine yesterday may time out tomorrow.

Edit Surface:
- src/eval_fixtures.rs — add timeout to run_fixture_command; add optional timeout_secs field to BenchmarkTask
- src/commands_eval.rs — add timeout to run_gate (same pattern, shared helper or duplicated)

Verifier:
- cargo build && cargo test
- python3 -c "
import subprocess, time
# Verify timeout produces failure, not hang
start = time.monotonic()
result = subprocess.run(['cargo', 'run', '--', 'eval', 'fixtures', 'run', '--task', 'nonexistent'],
                        capture_output=True, text=True, timeout=30)
# Should complete quickly (fixture not found), not hang
print('ok - eval runner responds within 30s')
"

Fallback:
- If thread-based timeout proves unreliable in CI (flaky kill-by-PID), fall back to
  adding a timeout_secs field to BenchmarkTask only (JSON schema) and documenting the
  constraint. The fixture author can set a shorter timeout, but the code won't enforce it
  at the process level. Mark the task done-with-findings.

Objective:
Make eval gates and fixture commands time-bounded: every command gets a default 120s
timeout (configurable per-fixture). A timed-out command produces a failure result
instead of hanging the eval agent.

Why this matters:
Every eval gate and fixture command that hangs wastes a task slot (20 min eval agent,
then auto-revert). As the state event log grows, commands like `yyds state lifecycle
--limit 1000` that were fine yesterday silently cross the timeout threshold and start
failing. Without per-command timeouts, the eval infrastructure can't detect or report
this — it just disappears into the evaluator agent's bash-tool timeout, producing no
verdict.

This directly raises task_verification_rate by converting "timed out → no verdict →
auto-revert" into "timed out → failure verdict → counted." It also prevents future
sessions from hitting the same wall when the event log grows.

Success Criteria:
- `run_fixture_command` returns within timeout_secs (default 120) even if the child
  process hangs
- `run_gate` returns within 120s even if the child process hangs
- Timed-out commands produce `FixtureCommandResult { passed: false, stderr_preview:
  "command timed out after Ns" }` or equivalent
- Existing tests pass; no regression in fixture or gate execution
- `cargo test --lib eval_fixtures` (or nearest test target) passes
- New unit test: spawn a `sleep 999` command with a 1s timeout, verify it returns
  failure within 3s wall time

Verification:
- cargo build && cargo test
- cargo test --test integration -- --test-threads=1

Expected Evidence:
- Task lineage shows eval_fixtures.rs changed with timeout logic
- A new unit test for timeout behavior exists and passes
- Future eval runs produce "timed out" failure results instead of hanging
- task_verification_rate improves (fewer tasks lost to timeout-then-revert)

Implementation Notes:
- Use only std library (no new dependencies). The pattern:
  1. `cmd.spawn()` to get a Child process
  2. Record the child PID before moving it into a thread
  3. `thread::spawn(move || child.wait_with_output())` with an `mpsc::channel` sender
  4. `rx.recv_timeout(Duration::from_secs(timeout))` on the parent
  5. On timeout: kill child by PID (`kill -9`), return failure result
  6. On success: unwrap the output and build the normal result

- Add `timeout_secs: Option<u64>` to `BenchmarkTask` in src/eval_fixtures.rs with
  serde(default). Default when None is 120.

- For `run_gate` in commands_eval.rs, apply the same pattern with a hard-coded 120s
  timeout (gates don't have per-gate config). Consider extracting a shared helper
  `run_command_with_timeout(command, timeout_secs, workdir) -> (bool, String, String)`
  to avoid duplication — both callers can use it.

- The FixtureCommandResult and GateResult structs should include the timeout in
  stderr_preview when the command times out, so the eval report is informative.

- Edge cases: if `cmd.spawn()` itself fails (command not found, permission denied),
  return the error immediately without spawning a thread. If `kill -9` fails (process
  already exited), ignore the error — the process is gone, which is the desired state.
