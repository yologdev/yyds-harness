Title: Investigate and fix dashboard tool-failure reconciliation gap
Files: scripts/build_evolution_dashboard.py, scripts/log_feedback.py
Issue: none
Origin: planner

Evidence:
- Assessment reports 3 transcript-only + 17 state-only tool failure records that disagree between the two recording systems. These appear in the dashboard's tool-failure reconciliation path.
- Trajectory graph-derived pressure items 4 ("Reconcile transcript-only tool failures") and 5 ("Reconcile state-only tool failures") both point at this same reconciliation gap.
- Trajectory item 2 ("Break recurring log failure fingerprints") is also dashboard-scoring-related: the recurring fingerprint "test failed, to rerun pass `--lib`" appears 2x historically in dashboard output, suggesting log_feedback.py may be misclassifying or double-counting certain failure classes.

Edit Surface:
- scripts/build_evolution_dashboard.py
- scripts/log_feedback.py

Verifier:
- python3 scripts/test_build_evolution_dashboard.py
- python3 -c "import scripts.log_feedback" (syntax check)
- grep for failed_tool_category, transcript_only, state_only, and reconciliation logic in both files to confirm changes are coherent

Fallback:
- If both scripts already handle transcript/state reconciliation correctly and the 20 discordant records are genuine (e.g., different scopes intentionally capture different events), write a brief findings note and mark the task as no-repro. Do not add reconciliation logic that would hide real disagreements.
- If the investigation reveals the mismatch is in a third file (not the two listed), note the third-file finding and stop — do not expand scope beyond 3 files.

Objective:
Understand whether the 20 discordant tool-failure records (3 transcript-only, 17 state-only) in the dashboard reflect a real evidence-capture bug or an intentional scope difference, and fix any reconciliation logic that is silently wrong.

Why this matters:
The dashboard is yyds's primary observability surface for evolution quality. When transcript and state records disagree about tool failures, the dashboard's corrective lessons may be wrong — either missing real failures (transcript-only) or hallucinating problems that didn't happen (state-only). The recurring "test failed, to rerun pass `--lib`" fingerprint in log feedback may be a downstream symptom of this reconciliation gap.

Success Criteria:
- The investigation identifies whether the 20 discordant records come from a code bug, a scope mismatch, or intentional design.
- If a code bug: the fix is applied and the reconciliation logic produces coherent output.
- If a scope mismatch: the dashboard output makes the scope distinction visible so future readers aren't misled.
- The recurring "test failed, to rerun pass `--lib`" fingerprint is traced to its source (either fixed or explained).

Verification:
- python3 scripts/test_build_evolution_dashboard.py
- python3 scripts/test_task_lineage_feedback.py (if applicable)
- Manual inspection: run `python3 scripts/log_feedback.py --help` (or equivalent) to confirm the script still parses
- If changes are made, confirm no syntax errors with `python3 -c "import py_compile; py_compile.compile('scripts/log_feedback.py', doraise=True)"` and same for build_evolution_dashboard.py

Expected Evidence:
- After fix: future dashboard runs show zero or explained-only discordant records between transcript and state tool-failure evidence.
- The recurring "test failed, to rerun pass `--lib`" fingerprint stops appearing in log feedback when no test failures actually occurred in the session.
- Task lineage shows the investigation path: what was found, what was fixed, or why it was intentional.

Implementation Notes:
- Start by tracing how tool failures flow from raw evidence into each script's summary output. Identify the reconciliation point(s) — where transcript evidence and state evidence are merged or compared.
- Check whether `failed_tool_category()`, `summarize_audit_actions()`, or similar functions in build_evolution_dashboard.py are the reconciliation point.
- In log_feedback.py, check `structured_tool_action_metrics()`, `tool_failure_label()`, and the state-vs-transcript comparison logic.
- The 20 discordant records are historical — the fix should prevent future discordance, not retroactively edit past evidence.
- Keep changes minimal: this is a diagnostics-and-fix task, not a rewrite. If the root cause is simple (e.g., one source filters by a different key than the other), the fix should be correspondingly small.
