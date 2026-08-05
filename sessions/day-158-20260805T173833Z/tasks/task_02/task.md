Title: Add signal-kill exit code hints to bash targeted_recovery_hint
Files: src/tool_wrappers.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure: failed_tool_summary.bash_tool_error=4
  "prefer bounded commands with explicit paths and inspect exit output before
  retrying broader checks"
- The targeted_recovery_hint function in src/tool_wrappers.rs (line 1018)
  already gives exit-code-specific advice for codes 1, 2, 126, and 127.
  Codes killed by signals (130=SIGINT, 137=SIGKILL, 139=SIGSEGV, 141=SIGPIPE,
  143=SIGTERM) get the catch-all empty-string advice, which is less helpful.
- In this planning session alone, a grep | head pipeline produced exit code 141
  (SIGPIPE) — a common failure mode when chaining commands.
- Adding signal-kill hints makes bash failures more self-diagnosable, reducing
  retry cycles and wasted turns.

Edit Surface:
- src/tool_wrappers.rs

Verifier:
- cargo test tool_wrappers

Fallback:
- If the exit_code match already covers these codes or if the bash_tool_error
  count is from non-exit-code failures (timeouts, spawn failures), narrow the
  task to the actual failure pattern or mark it obsolete.

Objective:
Extend the exit_code match in targeted_recovery_hint (line 1026) to include
specific advice for signal-killed exit codes: 130 (SIGINT/Ctrl+C), 137
(SIGKILL/OOM), 139 (SIGSEGV), 141 (SIGPIPE), and 143 (SIGTERM). Each hint
should name the signal and suggest a concrete next step.

Why this matters:
bash_tool_error=4 in the trajectory means implementation agents are spending
turns on failed bash commands. When the failure is a signal kill, the current
recovery hint just says "exit code N" with no signal context. The agent then
wastes another turn retrying the same command. A signal-aware hint tells the
agent whether the failure is retryable (SIGPIPE → use a bounded command),
fatal (SIGSEGV → the tool is broken, try something else), or environmental
(SIGKILL → OOM, use less memory).

Success Criteria:
- exit_code match arms for 130, 137, 139, 141, 143 return signal-specific advice.
- Existing tests pass (cargo test tool_wrappers).
- New tests verify each new exit code produces a non-empty hint string.

Verification:
- cargo test tool_wrappers
- cargo check

Expected Evidence:
- Tests for targeted_recovery_hint show signal-kill codes return meaningful hints.
- No change to existing hint behavior for codes 1, 2, 126, 127, or unknown codes.

Implementation Notes:
- The match is at src/tool_wrappers.rs line 1026 inside targeted_recovery_hint.
- Add arms BEFORE the wildcard `_ => ""` arm.
- Suggested messages:
  - 130: " (exit code 130 = SIGINT; command was interrupted, possibly by timeout
    or Ctrl+C — try with a longer timeout or a smaller scope)"
  - 137: " (exit code 137 = SIGKILL; likely out-of-memory — reduce input size
    or use a more memory-efficient command)"
  - 139: " (exit code 139 = SIGSEGV; the command itself crashed — try a
    different tool or check for known bugs in this version)"
  - 141: " (exit code 141 = SIGPIPE; output pipe closed early — use a bounded
    command (head -n, tail -n) or avoid piping to a consumer that exits first)"
  - 143: " (exit code 143 = SIGTERM; command was terminated — may be a
    resource limit or external signal; retry once, then escalate)"
- Add test functions in the existing targeted_recovery_hint test block (around
  line 2928) following the pattern of test_targeted_recovery_hint_bash_exit_code.
