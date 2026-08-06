Verdict: PASS
Reason: The panic hook at lines 64-82 now takes the active model call ID, records a ModelCallCompleted with "interrupted"/"rust_panic" status before recording FailureObserved, and the new test at line 8616 verifies correct event ordering and lifecycle pairing. The prompt.rs FailureObserved calls are for post-model-call tool errors and do not create the same gap. Build/tests pass.
