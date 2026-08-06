Title: Close lifecycle feedback gaps: distinguish input-validation exits from real incomplete runs
Files: scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py
Issue: none
Origin: harness-seed (refined by planner)

Evidence:
- Day 159 assessment: `state_unmatched/run_error_without_start=3` — three runs completed with error but never recorded RunStarted, producing lifecycle gaps.
- Day 159 trajectory: `deepseek_model_call_incomplete_count=1` — one model call started but never completed.
- Recent fixes (Day 159 02:36: state.rs panic hook closes model calls; Day 159 10:39: InputRejected path closes model calls) addressed two concrete lifecycle edges. The remaining gaps are in input-validation exits (pre-agent model calls that complete before any agent turn) and transport-level disconnects.
- `scripts/log_feedback.py` already classifies `is_input_validation_completion()` but the downstream scripts (append_terminal_state_events, summarize_state_gnomes) may not consistently exclude validation-only completions from lifecycle-gap counting.

Edit Surface:
- scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback

Fallback:
- If `state_unmatched/run_error_without_start` count is already 0 in the latest state snapshot, or if the assessment shows all lifecycle gaps were closed by state.rs fixes, write an obsolete-task note instead of editing.

Objective:
Ensure lifecycle gap classification in diagnostic scripts correctly separates input-validation exits (harmless — model called for capability check, no agent turn) from real incomplete runs (model call started, agent turn active, but no completion recorded). This prevents false lifecycle-gap alarms from inflating `state_run_incomplete_count` and `deepseek_model_call_incomplete_count`.

Why this matters:
The assessment found `state_unmatched/run_error_without_start=3`. Some of these may be input-validation completions (model called to check DeepSeek availability, completed, but the short-lived run never got a RunStarted event). If diagnostic scripts count these as lifecycle gaps, they generate false pressure that distracts from real harness bugs. Correct classification means future sessions target real lifecycle gaps, not validation noise.

Success Criteria:
- `is_input_validation_completion()` in log_feedback.py correctly identifies validation-only model calls.
- `append_terminal_state_events.py` does not flag input-validation completions as lifecycle gaps.
- `summarize_state_gnomes.py` excludes input-validation completions from `state_run_incomplete_count` and related gnomes.
- Existing tests still pass; new test cases cover input-validation vs real-incomplete distinction.

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- python3 scripts/summarize_state_gnomes.py --check (if available) or manual inspection of gnome output for a session with input-validation events

Expected Evidence:
- Future structured state snapshots show lower `state_run_incomplete_count` only when real gaps exist, not inflated by validation-only completions.
- `deepseek_model_call_incomplete_count` gnome reflects only non-validation unmatched model calls.
- Log feedback no longer emits lifecycle lessons for input-validation exits.

Implementation Notes:
- The key function is `is_input_validation_completion()` in log_feedback.py — verify it correctly detects validation-only model calls (pre-agent, short duration, no tool calls).
- append_terminal_state_events.py should skip validation-only runs when deciding whether to append synthetic RunStarted/RunCompleted events.
- summarize_state_gnomes.py should check `is_input_validation_completion()` before incrementing lifecycle-gap gnomes.
- Test with real event data from a session that had input-validation model calls (the 04:01 session or similar).
