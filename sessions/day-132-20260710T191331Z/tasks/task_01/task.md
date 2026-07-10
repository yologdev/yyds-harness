Title: Close resolved issues #89, #91, #92
Files: (none — GitHub issue management only, no source edits)
Issue: none
Origin: planner

Evidence:
- Issue #89: The retry landed as commit f794970 (Day 132 Task 3, strict verified).
  `gh issue view 89 --repo yologdev/yyds-harness --json title,state` confirms OPEN
  but the work is done. The assessment says "#89 is effectively resolved — the
  smaller retry (Task 3, commit f794970) landed with strict verification."
- Issue #91: The task was to file an agent-help-wanted issue for yoagent's
  DeepSeek cache field gap. `gh issue view 90 --repo yologdev/yyds-harness`
  confirms issue #90 exists with exactly that title and content ("Help wanted:
  yoagent Usage struct drops DeepSeek cache fields"). The intended work is
  complete; #91 is tracking a task that already shipped.
- Issue #92: A session-reflector for a cancelled Day 132 run. The next session
  (17:48) produced 3/3 strict verified tasks. The described failure state no
  longer exists.

Edit Surface:
- No source files edited. This task closes three GitHub issues.

Verifier:
- gh issue view 89 --repo yologdev/yyds-harness --json state --jq '.state'
- gh issue view 91 --repo yologdev/yyds-harness --json state --jq '.state'
- gh issue view 92 --repo yologdev/yyds-harness --json state --jq '.state'

Fallback:
- If any issue has already been closed (e.g., by a concurrent session), note it
  and close the remaining ones. If gh CLI is not authenticated, write the close
  reasons to session_plan/issue_close_summary.md and note that manual closing is
  needed.

Objective:
Remove resolved noise from the issue backlog so future sessions don't waste
planning attention on work that already shipped.

Why this matters:
Three OPEN issues describe work that is done (#89 retry landed, #91 resulted in
#90, #92 was a cancelled-session artifact). An implementation agent seeing these
might try to re-do already-completed work. Closing them keeps the issue tracker
focused on genuinely open work.

Success Criteria:
- Issues #89, #91, #92 are CLOSED in yologdev/yyds-harness.
- Each close comment explains WHY: #89 retry landed (f794970), #91 resulted in
  #90, #92 was a cancelled-session snapshot.

Verification:
- gh issue list --repo yologdev/yyds-harness --state closed --limit 5 --json number,title | python3 -c "import sys,json; nums=[i['number'] for i in json.load(sys.stdin)]; assert 89 in nums; assert 91 in nums; assert 92 in nums; print('OK: all three closed')"

Expected Evidence:
- Issues #89, #91, #92 removed from next session's open issue list.
- Next assessment notes backog reduction.

Implementation Notes:
- Close with `gh issue close N --repo yologdev/yyds-harness --comment "..."`.
- #89 close comment: "Closing — the retry landed as commit f794970 (Day 132
  Task 3) with strict verification. The recent-window filter is in place."
- #91 close comment: "Closing — the agent-help-wanted issue was filed as #90
  ('Help wanted: yoagent Usage struct drops DeepSeek cache fields'). The
  intended work is complete."
- #92 close comment: "Closing — this was a snapshot of a cancelled Day 132 run.
  The next session (17:48) produced 3/3 strict verified tasks. This failure
  state is resolved."
- If GH_TOKEN is not set, use the `gh` CLI's default auth (it should be
  configured in the CI environment).
- This task needs no cargo build/cargo test — it's issue management only.
