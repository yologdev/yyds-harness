Title: Audit lifecycle gap metrics and close issue #149 if confirmed stale
Files: none (audit-only, no code changes)
Issue: #149
Origin: planner

Evidence:
- The trajectory shows `deepseek_model_call_abnormal_completed_count=3` — 3 model calls completed without matching start events.
- Day 153 02:52 committed `1f417ee7` "Fix gnome lifecycle counting to filter retroactive harness-internal ModelCallCompleted events" — this already fixed the counting side by filtering synthetic retroactive events.
- The remaining 3 abnormal completions are likely from pre-fix historical data or input-validation runs (which don't call the model).
- Issue #149 was reverted by evaluator timeout — the implementation was correct but the evaluator never reached a verdict. The underlying data is likely already clean after the Day 153 gnome fix.
- If all remaining abnormal model completions are from pre-fix data or input-validation runs, the graph-pressure item is stale and #149 should be closed as obsolete so future sessions don't waste slots on ghost-chasing.

Edit Surface:
- No code changes. This is an audit task: run gnome summary, inspect lifecycle metrics, and close the issue via `gh issue close` if stale.

Verifier:
- python3 scripts/summarize_state_gnomes.py --test (baseline check)
- gh issue view 149 --repo yologdev/yyds-harness --json state,title (pre-condition)
- After audit: if closed, `gh issue view 149 --repo yologdev/yyds-harness --json state` shows "CLOSED"

Fallback:
- If `summarize_state_gnomes.py` shows zero lifecycle gaps (all clean), close #149 as obsolete with a note citing the Day 153 gnome fix.
- If gaps remain but are all from pre-fix historical data (runs before Day 153 02:52), close #149 with a note that the fix addressed the counting and remaining gaps are historical artifacts that will age out.
- If gaps remain AND include post-fix runs with real incomplete model calls, leave #149 open and file a narrower follow-up issue targeting the specific remaining gap type.
- If `gh issue close` fails (no token/perms), write the close reason to a comment instead.

Objective:
Determine whether the remaining `deepseek_model_call_abnormal_completed_count=3` is from pre-fix data or represents a live problem, and close issue #149 if the data confirms the Day 153 gnome fix resolved the underlying issue.

Why this matters:
Issue #149 has been open since Day 153 and was reverted by evaluator timeout (not because the code was wrong). The graph pressure item "Close yyds state and model lifecycle gaps" keeps appearing in trajectory output, but after the gnome counting fix, these gaps may be stale. Leaving #149 open wastes future task-selection slots on a problem that may already be fixed. Closing it (if confirmed stale) removes noise from the task picker and gives future sessions clearer signal.

Success Criteria:
- Run `summarize_state_gnomes.py` and inspect lifecycle-related metrics.
- Determine which of the remaining 3 abnormal completions (if any) are from post-fix runs vs pre-fix/input-validation.
- Close #149 with a clear explanation if all remaining gaps are benign, OR leave it open with updated evidence if a live gap remains.
- If #149 is closed, the next trajectory run shows one fewer graph-pressure row for lifecycle gaps.

Verification:
- python3 scripts/summarize_state_gnomes.py --test (no regression)
- gh issue view 149 --repo yologdev/yyds-harness --json state (CLOSED if stale, OPEN if live)
- If closing: gh issue close 149 --repo yologdev/yyds-harness --comment "Closing as obsolete: [brief reason with evidence]"

Expected Evidence:
- If closed: #149 state=CLOSED with a comment citing the Day 153 gnome fix as resolution.
- If left open: updated comment with specific evidence of which remaining gaps are live and actionable.
- Future trajectory output shows reduced lifecycle-gap pressure.

To audit the lifecycle gaps:
1. Run `python3 scripts/summarize_state_gnomes.py` and check the model-call lifecycle section.
2. Look at `deepseek_model_call_abnormal_completed_count` and related metrics.
3. Check whether the abnormal completions have timestamps before or after Day 153 02:52 (the gnome fix commit).
4. Use `is_input_validation_completion()` (already defined in the script) to classify any remaining gaps.
5. Based on findings: close #149 or update it with fresh evidence.
