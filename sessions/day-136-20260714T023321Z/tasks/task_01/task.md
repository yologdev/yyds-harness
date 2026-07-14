Title: Close yyds state and model lifecycle gaps
Files: scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py
Issue: none
Origin: harness-seed (refined by planner)

Evidence:
- Trajectory graph pressure #2: `state_run_incomplete_count=1` with lifecycle causes `state_unmatched/open_after_FailureObserved=3` — runs that emitted FailureObserved but never emitted RunCompleted, leaving their lifecycle book permanently open.
- Assessment: structured state snapshot confirms `open_after_FailureObserved=3` from the trajectory extractor (aggregated from audit-log sessions, not stale dashboard decoration).
- Lesson from Day 115 (memory): "Crash boundaries are where evidence goes to die — every crash path has two responsibilities: record what went wrong AND close the book so evidence is readable later."
- The three target scripts already track lifecycle start/completion counts and open runs. The gap is that terminal event recording must always close the lifecycle, not just count the gap.

Edit Surface:
- scripts/append_terminal_state_events.py — ensures terminal RunCompleted/FailureObserved events are always emitted for interrupted agent invocations
- scripts/log_feedback.py — ensures lifecycle lessons and feedback scoring account for unmatched/open runs
- scripts/summarize_state_gnomes.py — ensures state_run_incomplete_count reflects only genuinely incomplete runs, not non-validation input-validation exits

Verifier:
- python3 -m pytest scripts/test_append_terminal_state_events.py scripts/test_log_feedback.py scripts/test_summarize_state_gnomes.py 2>/dev/null || python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback 2>/dev/null || echo "no existing tests found — implementation must add focused assertions"

Fallback:
- If current assessment shows lifecycle gaps are already fixed (no `open_after_FailureObserved` in fresh trajectory) or the scripts already handle all edge cases, write task_01_obsolete.md explaining the contradiction with exact evidence from assessment/trajectory.

Objective:
Reduce `state_run_incomplete_count` by ensuring terminal RunCompleted/FailureObserved events are always recorded, and that lifecycle gnome computation classifies input-validation exits separately from genuinely incomplete runs.

Why this matters:
Open lifecycle events corrupt state feedback. When a run emits FailureObserved but never RunCompleted, the state graph shows an unbalanced lifecycle that:
1. Makes the dashboard report ghost incomplete runs that mask real problems
2. Prevents `state why last-failure` from finding genuine failure evidence
3. Blocks accurate session success rate measurement (graph pressure #3)

Success Criteria:
- The three scripts correctly detect and close open runs after agent invocation
- Input-validation exits (pre-agent, e.g. "no API key") are classified separately from genuine incomplete runs
- A future trajectory snapshot shows `state_run_incomplete_count` at 0 for sessions where no real lifecycle leak exists

Verification:
- python3 -m pytest scripts/test_append_terminal_state_events.py scripts/test_log_feedback.py scripts/test_summarize_state_gnomes.py 2>&1
- If no existing tests, add focused unit test assertions for: (a) a run with FailureObserved but no RunCompleted → RunCompleted retroactively emitted; (b) input-validation exit → NOT counted as incomplete

Expected Evidence:
- Future structured state snapshots show lower `state_run_incomplete_count` and `deepseek_model_call_incomplete_count`.
- `state why last-failure` correctly surfaces failure evidence from runs that previously had open lifecycles.

Implementation Notes:
- This task was seeded by the harness because recent runs reached planning without durable task files.
- Focus on the smallest change that closes the lifecycle: when detecting an open run (FailureObserved without RunCompleted), emit the missing terminal event rather than just reporting the gap.
- Do NOT touch `scripts/evolve.sh` — it is a protected file. If the fix requires changes there, file a help-wanted issue instead.
- The `open_after_FailureObserved=3` count comes from the audit-log trajectory, meaning these are real production gaps — not just a fresh CI artifact.
