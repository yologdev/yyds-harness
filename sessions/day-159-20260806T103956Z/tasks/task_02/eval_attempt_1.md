Verdict: PASS
Reason: The diff adds exactly the one-line `if attempted:` guard before `gnomes["session_success_rate"] = 0.0` in the `planning_failed` block, matching the task specification. Syntax is valid, build/tests pass, and the logic correctly distinguishes no-op sessions (attempted=0) from failed planning (attempted>0).
