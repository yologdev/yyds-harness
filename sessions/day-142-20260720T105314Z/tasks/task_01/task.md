Title: Add single-retry for timed-out bash commands in StreamingBashTool
Files: src/tools.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure row 4: `failed_tool_summary.bash_tool_error=14` —
  bash commands are the dominant tool-failure category across recent sessions.
  The RecoveryHintTool in tool_wrappers.rs already provides targeted recovery
  hints for bash failures (exit codes, timeouts, spawn failures), but the
  timeout path in StreamingBashTool::invoke (src/tools.rs:623-635) returns
  immediately with no retry.
- Session reverted-task streak: 4/5 recent sessions landed zero code.
  Implementation agents hit bash command failures (timeouts, exit codes)
  and the harness retry loop re-plans from scratch instead of giving the
  command a second chance.
- The bash timeout error message (line 632: "Command timed out after {}s")
  gives the agent a recovery hint but doesn't act on the timeout itself.
  A single automatic retry doubles the effective timeout without changing
  the default or requiring per-invocation timeout tuning.
- The `cache_metrics` evidence shows PromptCacheHit events are being
  recorded — a timeout retry at the tool layer leverages prompt caching
  (the retried command context is already cached).

Edit Surface:
- src/tools.rs: modify `StreamingBashTool::invoke` (lines ~486-642) to add
  a single timeout-retry loop around command spawning and waiting.

Verifier:
- cargo test tools -- --test-threads=1
- cargo build

Fallback:
- If the spawn+wait refactoring proves too invasive (touches >100 lines),
  abandon this approach and document why in an obsolete note. Do not attempt
  a broader refactor — the task is timeout retry only, not bash tool rewrite.
- If the existing tests (`test_streaming_bash_timeout`, `test_streaming_bash_progress_with_timeout`)
  would need substantial modification, add a focused new test and leave old
  tests intact.

Objective:
When a bash command times out, retry it once with doubled timeout (capped at
the 600s max) before reporting failure. This gives implementation agents one
more chance with a longer deadline, converting some `bash_tool_error` failures
into successes without changing the default timeout.

Why this matters:
`bash_tool_error=14` is the dominant tool-failure category in the trajectory.
Every failed bash command wastes the prompt context window that preceded it —
the agent's reasoning, tool calls, and accumulated state are all discarded
when a single command times out. A single retry amortizes the context cost
across two attempts: the second attempt benefits from prompt caching (the
prefix is identical) and gets 2x the time budget. This is the cheapest
possible intervention against the most frequent failure mode.

Success Criteria:
- A bash command that would time out at its configured timeout gets one
  automatic retry with doubled timeout (capped at 600s).
- The retry preserves `set -o pipefail` and RTK prefix behavior.
- `cargo test tools` passes — existing timeout tests continue to work.
- The retry is transparent: tool output comes from whichever attempt
  succeeded, and the diagnostic error distinguishes first-timeout from
  final-timeout.

Verification:
- cargo test tools -- --test-threads=1
- cargo build
- Manual: a command that takes 3s with timeout=2s should succeed on retry
  with effective timeout=4s and return the command's output.

Expected Evidence:
- Future trajectory snapshots show reduced `bash_tool_error` count in
  `failed_tool_summary`.
- State events show `stash_diagnostic_error` with "bash timeout (retrying)"
  pattern when retry is attempted.

Implementation Notes:
- Use `tokio::time::timeout()` instead of `tokio::time::sleep()` in the
  select so timeout extension is cleaner. The current code uses:
  ```rust
  _ = tokio::time::sleep(timeout) => { /* kill, return error */ }
  ```
  Replace with:
  ```rust
  result = tokio::time::timeout(current_timeout, child.wait()) => {
      match result {
          Ok(Ok(status)) => { /* success */ }
          Ok(Err(e)) => { /* wait error */ }
          Err(_elapsed) => { /* timeout — retry or fail */ }
      }
  }
  ```
  This lets the timeout branch check a retry flag and continue the loop
  instead of returning immediately.
- Wrap lines ~486-642 (from `cmd.spawn()` through `reader_handle` await)
  in a `loop` with a retry flag. The loop body:
  1. Build and spawn a fresh child process each iteration.
  2. Set up stdout/stderr reader (same as current code).
  3. Use `tokio::select!` with `cancel.cancelled()` and
     `tokio::time::timeout(current_timeout, child.wait())`.
  4. On timeout: if first attempt and `current_timeout < 600s`, kill child,
     double timeout, stash "bash timeout (retrying)" diagnostic, continue loop.
  5. On timeout (final): kill child, abort reader, stash "bash timeout"
     diagnostic, return error.
  6. On success: break with exit_status.
- Keep `accumulated` and `truncated` inside the loop (fresh Arc's per attempt)
  so retry output is clean.
- Keep the change under 60 lines net. Do not refactor unrelated code.
- The `cancel.cancelled()` branch still returns immediately — cancellation
  is not retried.
