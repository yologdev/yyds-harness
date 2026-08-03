Title: Close model_completion_without_start lifecycle gaps in terminal state events
Files: scripts/append_terminal_state_events.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- Graph-derived next-task pressure #1: "Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=42): Lifecycle causes: model_incomplete/model_completion_without_start=8"
- Trajectory shows 42 unmatched model call completions, 8 specifically from model_completion_without_start — ModelCallCompleted events with no matching ModelCallStarted.
- Day 154 10:00 Task 2 (src/prompt.rs) closed model-call lifecycle in the panic path going forward, but historical orphaned events need post-hoc repair.
- Day 154 10:00 Task 1 added input-validation classification to append_terminal_state_events.py — the same pattern (classification helper + skip/repair) is the right approach.
- This is a narrower, single-file version of the reverted #152 which touched 3 files and timed out. Focus only on model_completion_without_start, not all lifecycle gaps.

Edit Surface:
- scripts/append_terminal_state_events.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events

Fallback:
- If the model_completion_without_start classification cannot be reliably distinguished (e.g., the state events lack the necessary fields to determine whether a start was missing vs. the start event was corrupted/truncated), document what was tried and mark the task blocked. Do not add fragile heuristics that fire on ambiguous data.

Objective:
Detect ModelCallCompleted events that have no matching ModelCallStarted in the state event log and create synthetic ModelCallStarted events to close the lifecycle gap. This targets 8 of the 42 unmatched completions.

Why this matters:
Model lifecycle gaps distort two signals: deepseek_model_call_incomplete_count inflates failure metrics, and state lifecycle integrity checks flag false positives. The panic-path fix (Day 154 Task 2) prevents new gaps going forward; this post-hoc fix repairs the 8 existing gaps from historical sessions. Clean lifecycle data makes gnome KPIs trustworthy for task selection and fitness measurement.

Success Criteria:
- append_terminal_state_events.py detects orphaned ModelCallCompleted events that lack a matching ModelCallStarted.
- For each orphaned completion, a synthetic ModelCallStarted event is added with the same model_call_id and session_id, marked with a "synthetic: model_completion_without_start" provenance field.
- Existing tests (including Day 154 input-validation skip tests) continue to pass.
- The change is small enough to complete in 20 minutes — one classification helper, one repair loop, tests.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events
- python3 scripts/append_terminal_state_events.py --dry-run (if supported) to confirm orphaned completions are detected

Expected Evidence:
- Future trajectory shows lower deepseek_model_call_unmatched_completed_count (drops from 42 toward 34 after this fix addresses the 8 model_completion_without_start cases).
- State lifecycle checks show fewer "ModelCallCompleted without ModelCallStarted" anomalies.

Implementation Notes:
- Use the same classification pattern as Day 154 Task 1's input-validation skip: a helper function that scans the event log for orphaned completions, then a repair loop that adds synthetic start events.
- The detection logic: collect all model_call_ids from ModelCallStarted events, then find ModelCallCompleted events whose model_call_id is not in that set.
- Synthetic events should include: event_type="ModelCallStarted", model_call_id matching the orphaned completion, a "synthetic": true and "synthetic_reason": "model_completion_without_start" in the payload.
- Keep changes scoped to this single file. This is intentionally narrower than the reverted #152 which also touched log_feedback.py and summarize_state_gnomes.py.
- If the existing test file (scripts/test_append_terminal_state_events.py) doesn't already have a test fixture with orphaned ModelCallCompleted events, add one.
