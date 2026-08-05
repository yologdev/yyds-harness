Verdict: PASS
Reason: Diff adds signal-specific match arms for exit codes 130/137/139/141/143 before the wildcard arm, matching the task's exact message suggestions. New test `test_targeted_recovery_hint_bash_signal_exit_codes` verifies each signal code returns a non-empty hint containing the signal name and key terms. Build and tests pass. No existing behavior changed.
