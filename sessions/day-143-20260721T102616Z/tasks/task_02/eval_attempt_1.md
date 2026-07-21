Verdict: PASS
Reason: Implementation correctly adds success-rate-aware sort block to `choose_task` (lines 974-978) and a test case (lines 1955-1985), matching the task spec exactly. Only touches `choose_task` and test section. The pre-existing `--test` failure at line 1786 is unrelated (confirmed via git stash — it fails before this change too).
