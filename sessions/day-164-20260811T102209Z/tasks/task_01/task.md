Title: Classify SIGTERM-cancelled runs in log_feedback.py to reduce false lifecycle pressure
Files: scripts/log_feedback.py
Issue: none
Origin: planner (refined from harness-seed — narrowed 3-file scope to 1-file, 1-function change)

Evidence:
- Day 164 assessment structured state: `state_unmatched_non_validation=1` — one run lifecycle gap where a RunCompleted exists without RunStarted, and it's NOT an input-validation exit.
- The trajectory shows 3/5 recent CI runs were "cancelled" (GH Actions kill). Cancelled runs can leave RunCompleted events without RunStarted events.
- `scripts/log_feedback.py` function `is_cancelled_completion()` (line 1279) already detects two signals: explicit `status == "cancelled"` and SIGTERM panic (RunCompleted + error + empty error_detail). But `state_unmatched_non_validation=1` persists, meaning at least one cancelled run isn't being classified.
- `scripts/append_terminal_state_events.py` function `_is_run_externally_cancelled` may have a pattern not mirrored in `is_cancelled_completion()`. The two functions have drifted.
- The assessment says lifecycle gap tasks keep getting reverted because implementation agents get lost in broad analysis. This task is ONE function in ONE file — no room for analysis paralysis.

Edit Surface:
- scripts/log_feedback.py

Verifier:
- python3 -c "from scripts.log_feedback import is_cancelled_completion; print('import ok')"

Fallback:
- If `yyds state tail --limit 500 | grep -c '"kind":"RunCompleted"'` and `yyds state tail --limit 500 | grep -c '"kind":"RunStarted"'` are already equal (all recent runs are properly paired), mark this task obsolete — the `state_unmatched_non_validation=1` is a retroactive artifact that will age out of the window.
- If the unmatched run has a RunStarted but no RunCompleted (the opposite direction), this task doesn't apply — write an obsolete note and describe the actual direction of the gap.

Objective:
Make `is_cancelled_completion()` in `scripts/log_feedback.py` recognize ALL externally-cancelled runs, so `state_run_unmatched_non_validation_completed_count` no longer counts cancelled CI runs as real lifecycle gaps.

Why this matters:
`state_unmatched_non_validation=1` feeds into trajectory graph pressure every session, creating false urgency about lifecycle gaps that are just cancelled CI runs. Until cancelled runs are filtered out, every session's dashboard shows a phantom lifecycle gap that distracts from real harness issues and wastes a task slot on a non-problem.

Success Criteria:
- `is_cancelled_completion()` returns `True` for the currently-unmatched cancelled run (the one producing `state_unmatched_non_validation=1`).
- The function remains conservative: when uncertain, returns `False`. False negatives are less harmful than false positives.
- No regression: existing input-validation classification continues to work correctly.

Verification:
- python3 -c "from scripts.log_feedback import is_cancelled_completion; print('import ok')"
- python3 scripts/log_feedback.py 2>&1 | head -5  (confirm script parses)

Expected Evidence:
- Next trajectory shows `state_run_unmatched_non_validation_completed_count=0`.
- Lifecycle graph pressure stops listing "Close yyds state and model lifecycle gaps" as a top item.

Implementation Notes:
- Start by running `yyds state tail --limit 200 | grep '"kind":"RunCompleted"'` and `yyds state tail --limit 200 | grep '"kind":"RunStarted"'` to find unmatched RunCompleted events.
- For each unmatched RunCompleted, inspect the payload's `error_detail` and `status` fields.
- Compare `is_cancelled_completion()` in `scripts/log_feedback.py` (line 1279-1307) with `_is_run_externally_cancelled` in `scripts/append_terminal_state_events.py`.
- Look for patterns that `_is_run_externally_cancelled` catches but `is_cancelled_completion()` misses. Common drift: one function checks for a signal (like "SIGTERM" in error_detail) that the other doesn't.
- The fix should be 5-15 lines. Add one new detection pattern to the existing function. Do NOT refactor, extract helpers, or restructure.
- If `_is_run_externally_cancelled` has a pattern not in `is_cancelled_completion()`, mirror it.
- If no new pattern is found (both functions are already aligned), check whether the unmatched run has `status == "cancelled"` but the status check is case-sensitive and the actual value is "Cancelled" — a simple `.lower()` might fix it.
- Keep the change surgical. Do not modify any other function or file.
