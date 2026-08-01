Verdict: PASS
Reason: All three parts implemented correctly — CURRENT_MODEL_CALL_ID thread-local with set/clear fns in state.rs, panic hook records ModelCallCompleted before FailureObserved, and prompt.rs sets/clears at all three ModelCallCompleted sites. Build and tests pass.
