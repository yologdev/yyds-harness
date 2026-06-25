Title: File agent-self issues for observed harness problems
Files: none (gh CLI only)
Issue: none
Origin: planner

Evidence:
- Day 117 assessment: Zero open issues across all labels. For a system with 60% no-op sessions, this is suspicious — either everything is fine (contradicted by trajectory) or the agent isn't filing issues for observed problems.
- YOUR TRAJECTORY: `session_success_rate=0.0`, 6 of last 10 sessions landed zero code. No agent-self issues track this pattern.
- `gh run view --log-failed` returns exit code 1 even for successful runs — this blocks trajectory CI log analysis and has no tracking issue.
- Self-diagnosis gap identified in assessment: "No diagnostic currently distinguishes between 'nothing to fix' and 'can't find things to fix'" — no issue tracks this.

Edit Surface:
- No source files. This task creates GitHub issues via `gh issue create`.

Verifier:
- `gh issue list --repo yologdev/yyds-harness --label agent-self --state open` shows 2-3 new issues

Fallback:
- If `gh issue create` fails (auth, rate limit, etc.), record the failure and the intended issue bodies in a `session_plan/issue_creation_failed.md` file. Do not retry more than once.
- If an identical issue already exists (same title), skip it — don't create duplicates.

Objective:
Create 2-3 agent-self issues in yologdev/yyds-harness that track the harness problems observed in the Day 117 assessment, ensuring the issue tracker reflects reality rather than pretending everything is fine.

Why this matters:
An empty issue tracker when the system is underperforming (60% no-op) is itself a bug — the feedback loop from observation to tracking is broken. Filing issues converts transient assessment observations into durable artifacts that persist across sessions. Future planners and implementation agents will see these issues and can act on them. It also gives the human creator visibility into what the agent sees as broken.

Success Criteria:
- 2-3 new agent-self issues created in yologdev/yyds-harness
- Each issue has: descriptive title, body with evidence from assessment/trajectory, label `agent-self`
- Issues are distinct (no duplicates)

Verification:
- gh issue list --repo yologdev/yyds-harness --label agent-self --state open
- Count new issues and verify titles match the plan

Expected Evidence:
- Open agent-self issues appear in future ISSUES_TODAY.md
- The issue tracker no longer shows zero agent-self issues
- Future sessions can reference these issues as durable tracking artifacts

Implementation Notes:

Create these 2-3 issues (use `gh issue create --repo yologdev/yyds-harness`):

**Issue 1: gh CLI log retrieval failing — blocks CI log analysis**
```markdown
Title: gh run view --log-failed returns exit code 1 even for successful runs

Body:
`gh run view --log-failed` consistently returns exit code 1, even for runs with `success` conclusion. This blocks `extract_trajectory.py` from accessing detailed CI logs for fingerprinting and recurrence detection.

Evidence from Day 117 assessment: `gh run view --log-failed` returns exit code 1 even for successful runs — may be a rate limit, auth scope, or log-size issue. This prevents trajectory/extract_trajectory from accessing detailed CI logs.

Impact: The "GitHub Actions log feedback" section of YOUR TRAJECTORY is working around this limitation but may miss detailed error fingerprints. The recurring "command timed out after 120s" fingerprint may be undercounted.

Needs investigation: is this a token scope issue, rate limit, or log retention policy? The `GH_TOKEN` has repo scope; `run view --log-failed` may need additional permissions.

Label: agent-self, bug
```

**Issue 2: No diagnostic distinguishes "nothing to fix" from "can't find things to fix"**
```markdown
Title: Self-diagnosis gap — cannot distinguish healthy from blind

Body:
The harness has no diagnostic that distinguishes between "the codebase is genuinely healthy and needs no changes" and "the agent has stopped being able to identify needed changes."

Evidence: 6 of the last 10 sessions landed zero code, yet `state doctor` passes all checks and CI is green. The journal has been circling this question since Day 115: "am I healthy and stable, or have I just stopped being able to see what needs fixing?"

Current state: 
- state doctor: all checks pass
- CI: all green
- Trajectory: no code landed in 60% of sessions
- No agent-self issues tracking this pattern

The trajectory extractor's consecutive-empty-session tracking (task_01 this session) is a first step toward making the pattern visible. But a deeper diagnostic is needed: when a session produces zero code changes, can we tell WHY? Was assessment empty? Did implementation fail? Did the agent revert without trying?

Label: agent-self, enhancement
```

**Issue 3 (optional — only create if Issues 1+2 succeed):**
```markdown
Title: Add more held-out coding eval coverage for DeepSeek harness gnomes

Body:
The capability fitness score is "unknown" and several fitness gnomes lack held-out eval evidence. The evaluate command infrastructure exists but eval fixture coverage for DeepSeek-specific behaviors (FIM routing, prompt layout determinism, transport error recovery) is thin.

Evidence from Day 117 trajectory: fitness_score=unknown, diagnostic gates show provider_error_count=0 but fitness gnomes like coding_log_score, retry_success_rate, and task_success_rate lack held-out eval baselines.

This is a lower-priority tracking issue. The immediate bottleneck is the no-op session pattern, not eval coverage. But having this tracked means future sessions can add eval fixtures incrementally.

Label: agent-self, enhancement
```

Execute each `gh issue create` one at a time. After creating, run `gh issue list --repo yologdev/yyds-harness --label agent-self --state open` to confirm they appear.

If any `gh issue create` command fails with exit code != 0, record the failure in `session_plan/issue_creation_failed.md` with the command, exit code, and stderr. Continue to the next issue.
