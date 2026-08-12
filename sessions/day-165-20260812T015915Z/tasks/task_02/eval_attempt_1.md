Verdict: PASS
Reason: Diff adds exactly 3 new recovery-hint match arms ("cannot create directory", "syntax error near unexpected token", "not a directory") to the targeted_recovery_hint function. None duplicate existing patterns. Build and tests pass. No existing hints modified or refactored. Meets all success criteria.
