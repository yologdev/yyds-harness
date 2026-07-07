Verdict: PASS
Reason: The `task.get("files")` filter is correctly added to the `selectable` list comprehension at line 313, excluding file-less tasks. Two new tests verify exclusion behavior and the `no_selectable_tasks` fallback. Build and tests are green.
