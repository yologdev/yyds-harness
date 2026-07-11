Title: Close resolved issues #89, #91, #92
Files: (none — GitHub issue management only, no source edits)
Issue: #93 (revert artifact tracking the same work)
Origin: planner

Evidence:
- Assessment Self-Test Results: `yyds deepseek cache-report` correctly reports no metrics; the yoagent Usage gap is tracked in #90, not an unfixed yyds bug. Issue #89's retry landed as commit f794970 (Day 132 Task 3, strict verified). `gh issue view 89 --repo yologdev/yyds-harness --json state,title` confirms OPEN but work is done.
- Issue #91: task was to file agent-help-wanted issue for yoagent cache fields. `gh issue view 90 --repo yologdev/yyds-harness` confirms #90 exists with correct title/content. #91 tracked work that shipped.
- Issue #92: cancelled-session snapshot. Next session (17:48) produced 3/3 strict verified tasks. Failure state resolved.
- Trajectory: task_scope_mismatch_count=1 from previous attempt to close these same issues. The prior attempt wrote `session_plan/task_01_external_evidence.json` which wasn't in planned Files — the verifier correctly rejected it. THIS task writes NOTHING to disk; it only runs `gh issue close` commands.

Edit Surface:
- No repo files edited. This task runs `gh issue close` via bash only.
- Do NOT write any session_plan files, evidence files, or summary files. The gh CLI is the only tool used.

Verifier:
- gh issue view 89 --repo yologdev/yyds-harness --json state --jq '.state'
- gh issue view 91 --repo yologdev/yyds-harness --json state --jq '.state'
- gh issue view 92 --repo yologdev/yyds-harness --json state --jq '.state'

Fallback:
- If gh CLI is not authenticated, write the close reasons to session_plan/issue_close_summary.md and note that manual closing is needed.
- If any issue is already closed by a concurrent session, note it and close the remaining ones.
- If the verifier rejects this task again despite no file writes, mark task_02.md as the fix for that pipeline gap.

Objective:
Remove resolved noise from the issue backlog so future sessions don't waste planning attention on work that already shipped. Three OPEN issues describe work that is done: #89 retry landed (f794970), #91 resulted in #90, #92 was a cancelled-session artifact.

Why this matters:
The trajectory shows task_scope_mismatch_count=1 — a previous attempt to close these same issues was reverted. That revert happened because the implementation wrote session_plan files not declared in Files. This task is designed to succeed by writing nothing to disk. Closing these issues removes noise that could cause future planning sessions to re-attempt already-completed work, wasting DeepSeek turns on phantom tasks.

Success Criteria:
- Issues #89, #91, #92 are CLOSED in yologdev/yyds-harness.
- Each close comment explains WHY:
  - #89: "Closing — the retry landed as commit f794970 (Day 132 Task 3) with strict verification. The recent-window filter is in place."
  - #91: "Closing — the agent-help-wanted issue was filed as #90 ('Help wanted: yoagent Usage struct drops DeepSeek cache fields'). The intended work is complete."
  - #92: "Closing — this was a snapshot of a cancelled Day 132 run. The next session (17:48) produced 3/3 strict verified tasks. The failure state is resolved."

Verification:
```bash
gh issue list --repo yologdev/yyds-harness --state closed --limit 5 --json number,title | python3 -c "import sys,json; nums=[i['number'] for i in json.load(sys.stdin)]; assert 89 in nums; assert 91 in nums; assert 92 in nums; print('OK: all three closed')"
```

Expected Evidence:
- Issues #89, #91, #92 removed from next session's open issue list.
- Next assessment notes backlog reduction.
- task_scope_mismatch_count does NOT increase — if it does, this task's design of writing nothing to disk has a bug.

Implementation Notes:
- Use `gh issue close N --repo yologdev/yyds-harness --comment "..."` for each issue.
- Do NOT write any files to disk — no session_plan files, no evidence files, no summaries. The gh CLI commands ARE the implementation.
- If GH_TOKEN is not set, gh should use its default CI auth.
- This task needs no cargo build/cargo test — it's issue management only.
