Verdict: PASS
Reason: The fix adds a ModelCallCompleted event with status "input_rejected" in the InputRejected handler, using the same guard pattern as the ctrl+c handler. It preserves the existing FailureObserved recording and diagnostic print. All four terminal paths (Completion, InputRejected, ctrl+c, channel close) now close the model call lifecycle. Build and tests pass.
