Title: Close ModelCall lifecycle gaps — write retroactive ModelCallCompleted for orphaned ModelCallStarted events
Files: scripts/append_terminal_state_events.py, scripts/test_append_terminal_state_events.py
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure #1: "Close yyds state and model lifecycle gaps (deepseek_model_call_incomplete_count=8): Lifecycle causes: model_incomplete/open_after_ModelCallStarted=8; stale incomplete events from historical runs."
- The janitor already handles RunStarted/RunCompleted and FailureObserved lifecycle gaps (find_missing_failure_observed, find_runs_with_failure_observed_no_completion). ModelCallStarted/ModelCallCompleted is a parallel lifecycle track with no retroactive closure logic.
- src/prompt.rs lines 777-781 emit ModelCallStarted, lines 939-954 emit ModelCallCompleted on AgentEnd, lines 1012-1027 on ctrl_c, and lines 1054-1064 on channel close. The 8 orphaned ModelCallStarted events are from historical runs that crashed before any of these completion paths fired — the same class of problem the janitor already fixes for RunStarted/RunCompleted.

Edit Surface:
- scripts/append_terminal_state_events.py
- scripts/test_append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --dry-run (no new errors or unexpected terminal events)

Fallback:
- If the 8 orphaned ModelCallStarted events all belong to runs that already have RunCompleted closed by the janitor (i.e., the runs are fully closed at the run level), add a simpler "closed-run model-call sweep" that writes ModelCallCompleted for any ModelCallStarted whose run_id maps to a completed run AND that has no existing ModelCallCompleted. Mark the task as done if the sweep logic works and the test passes.
- If orphaned ModelCallStarted events somehow reference run_ids that don't exist in the event stream (corrupted data), write a diagnostic note and emit a limited retroactive ModelCallCompleted only for the recoverable subset. Do not spend time repairing corrupted state.

Objective:
Extend the state janitor (`scripts/append_terminal_state_events.py`) to detect and close ModelCallStarted events that have no matching ModelCallCompleted, writing retroactive ModelCallCompleted events with `"retroactive": true` and a reason like `"retroactive: ModelCallStarted orphaned — no ModelCallCompleted found"`.

Why this matters:
The #1 graph-pressure item is model call lifecycle gaps. Unclosed model calls mean state feedback is incomplete — metrics that depend on model call pairing (latency, success rate, cache behavior) can't be computed accurately. The janitor already closes RunStarted/RunCompleted and FailureObserved gaps; ModelCall pairs are the last major lifecycle track without retroactive closure. Fixing this directly reduces `deepseek_model_call_incomplete_count` from 8 to 0 and gives the graph-based task picker one fewer recurring pressure signal to nag about.

Success Criteria:
- The janitor detects ModelCallStarted events with no corresponding ModelCallCompleted (matched by model_call_id).
- For each orphaned ModelCallStarted, a retroactive ModelCallCompleted is written with `"retroactive": true` and a descriptive reason.
- No duplicate ModelCallCompleted events are written for model calls that already have a completion.
- Existing janitor behavior (FailureObserved dedup, RunStarted/RunCompleted closure) is unchanged.
- The test file has a new test case covering the ModelCall lifecycle closure path.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --dry-run 2>&1 | head -20 (sanity check output)

Expected Evidence:
- After the janitor runs in a subsequent session, `yyds state graph hotspots` shows reduced model_incomplete count (trending toward 0).
- Future trajectory snapshots show `deepseek_model_call_incomplete_count` decreasing.
- Test output confirms the new lifecycle closure logic fires for orphaned ModelCallStarted events.

Implementation Notes:
- ModelCallStarted and ModelCallCompleted are paired by `model_call_id` in the payload. The janitor should scan for ModelCallStarted events, collect their model_call_ids, then check which ones lack a matching ModelCallCompleted.
- The retroactive ModelCallCompleted payload should include: `{"model_call_id": <id>, "retroactive": true, "completion_reason": "retroactive: ModelCallStarted orphaned — no ModelCallCompleted found"}`.
- For the `model` field in the retroactive completion, use the model name from the original ModelCallStarted payload.
- Follow the same pattern as the existing FailureObserved retroactive writing: use `_maybe_append_event()` for idempotent dry-run support, include `session_id` and `trace_id` from the current session, actor="harness".
- Add a new diagnostic key (e.g., `"model_call_lifecycle_diagnostics"`) to the diagnostics dict summarizing how many orphaned model calls were found and how many were closed.
- In the test file, add a test that: (1) writes events with ModelCallStarted but no ModelCallCompleted, (2) calls the janitor logic, (3) asserts ModelCallCompleted events were appended.
