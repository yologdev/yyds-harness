Verdict: PASS
Reason: The diff adds the exact `render_task` guard specified in the task description, raising `ValueError` for empty or whitespace-only `files`. Two new tests cover both edge cases, and `python3 scripts/preseed_session_plan.py --test` passes all tests.
