Title: Close yyds state and model lifecycle gaps
Files: scripts/evolve.sh, scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py
Issue: none
Origin: harness-seed
validated_against_assessment: true

Objective:
Ensure every yyds DeepSeek invocation and model-call path records paired terminal lifecycle events, and keep feedback lessons precise when lifecycle imbalance is real.

Why this matters:
The assessment found incomplete run/model-call lifecycle gnomes. Those signals affect state feedback, assessment trust, and future task selection more directly than dashboard display.

Success Criteria:
- Normal, timeout, error, and completion-file early-stop paths record terminal RunCompleted and ModelCallCompleted events.
- Pre-agent input-validation exits stay classified separately from non-validation unmatched completions.
- Log feedback and state summaries emit lifecycle lessons only for real incomplete or non-validation unmatched paths.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- bash -n scripts/evolve.sh

Expected Evidence:
- Future structured state snapshots show lower `state_run_incomplete_count` and `deepseek_model_call_incomplete_count`.
- Lifecycle repair tasks are selected from current assessment evidence instead of stale dashboard-only symptoms.

Implementation Notes:
- This task was seeded by the harness before planner exploration because recent runs reached planning without durable task files.
- Treat it as a minimum viable task for Day 107 (21:14); refine it if the planner has stronger evidence, but do not leave the session with zero task files.
- Keep the change scoped to the listed files unless verification reveals a direct dependency.
