Verdict: PASS
Reason: The diff adds an exit code 42 recovery hint, replaces the silent `_ => ""` wildcard with a generic fallback hint, updates the existing test assertion, and adds a new test for unknown exit code 99 — all matching the task spec. Build and tests pass.
