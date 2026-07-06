Title: Clean up lifecycle gnome classification: separate input-validation exits from real unmatched completions (#73 retry)
Files: scripts/log_feedback.py, scripts/summarize_state_gnomes.py, scripts/append_terminal_state_events.py
Issue: #73
Origin: planner (refined from harness-seed)

Evidence:
- Graph pressure: `state_run_unmatched_non_validation_completed_count=22` — lifecycle gnome counts are inflated by pre-agent input-validation exits (empty_input, invalid_input:*) that never called the model.
- Day 127 03:30 session landed `append_terminal_state_events.py` (+77 lines) to retroactively append FailureObserved for error-completed runs. But `is_input_validation_completion()` (summarize_state_gnomes.py:464) is only used locally — not plumbed through to log_feedback.py's lifecycle lesson emission or the gnome aggregation pipeline.
- The trajectory reports `deepseek_model_call_incomplete_count` and `state_run_incomplete_count` as gnomes that feed into lifecycle repair task selection. Input-validation exits that inflate these counts cause the harness to select "fix lifecycle gaps" tasks when the actual gap is zero.
- `state_run_unstarted_input_validation_error_count` already exists as a separate gnome key in log_feedback.py:272 — the bucket exists but isn't consistently used to subtract from the incomplete-count gnomes.
- Day 127 Task 1 was reverted due to missing Files: entries in the task file — not a content failure. The implementation plan in #73 is complete and correct.

Edit Surface:
- scripts/log_feedback.py — add input-validation filtering to lifecycle lesson emission (before emitting "incomplete model call" or "incomplete run" lessons, filter out events where is_input_validation_completion() returns true)
- scripts/summarize_state_gnomes.py — ensure is_input_validation_completion() is used when computing incomplete model call / incomplete run gnomes; input-validation exits should increment state_run_unstarted_input_validation_error_count but NOT state_run_incomplete_count or deepseek_model_call_incomplete_count
- scripts/append_terminal_state_events.py — read-only context; may need no change if the fix is purely in the other two scripts

Verifier:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- python3 -c "
import scripts.summarize_state_gnomes as sg
assert sg.is_input_validation_completion({'kind': 'RunCompleted', 'status': 'error', 'error_detail': 'empty_input'}) == True
assert sg.is_input_validation_completion({'kind': 'RunCompleted', 'status': 'error', 'error_detail': 'not validation'}) == False
print('ok - input validation classification works')
"

Fallback:
- If the scripts already correctly subtract input-validation exits from incomplete counts (verify by reading the actual gnome computation paths in both scripts), mark the task done-with-findings and note the specific line numbers where filtering already happens.
- If the fix requires restructuring more than 50 lines across the scripts, narrow scope to the single highest-impact change: ensure is_input_validation_completion is called before emitting any "incomplete model call" lifecycle lesson in log_feedback.py.

Objective:
Ensure that pre-agent input-validation exits (empty_input, invalid_input:*) are classified separately from real unmatched model completions in lifecycle gnomes and feedback lessons, so the harness doesn't waste task slots on "fix lifecycle gaps" when all unmatched completions are pre-agent validation exits.

Why this matters:
When `deepseek_model_call_incomplete_count` includes input-validation exits, the harness sees "incomplete model calls → need lifecycle repair" and selects tasks to fix lifecycle recording. But if those incompletes are actually pre-agent validation exits (the model was never called), the repair task is chasing a phantom — the lifecycle recording is already correct. This wastes task slots and inflates reverted counts when repair tasks can't find anything to fix. The Day 127 03:30 session already added retroactive FailureObserved for error runs; the remaining gap is gnome counter accuracy.

Success Criteria:
- is_input_validation_completion() is called before incrementing incomplete model call counts or emitting "incomplete model calls" lessons
- Input-validation exits are counted in state_run_unstarted_input_validation_error_count (already exists) but NOT in state_run_incomplete_count or deepseek_model_call_incomplete_count
- Lifecycle feedback lessons (log_feedback.py) only recommend lifecycle repair when there are real (non-validation) incomplete paths
- Existing tests pass (no regression in script test suites)

Verification:
- python3 -m unittest scripts.test_append_terminal_state_events scripts.test_task_lineage_feedback
- Manual spot-check: run `python3 scripts/summarize_state_gnomes.py` and verify that state_run_unstarted_input_validation_error_count captures validation exits separately from state_run_incomplete_count

Expected Evidence:
- Future structured state snapshots show lower deepseek_model_call_incomplete_count when the only unmatched completions are pre-agent validation exits
- The trajectory stops recommending lifecycle-repair tasks when lifecycle recording is actually healthy
- Task lineage shows the script files changed with input-validation filtering logic

Implementation Notes:
- The is_input_validation_completion() function (summarize_state_gnomes.py:464) checks kind == "RunCompleted" AND status == "error" AND detail matches "empty_input" or "invalid_input:...". This is the canonical check — use it, don't duplicate it.
- In log_feedback.py, find where lifecycle lessons are emitted (search for "incomplete model call" or "incomplete run" in the lesson text). Before emitting those lessons, filter out events where is_input_validation_completion() returns true.
- In summarize_state_gnomes.py, find where state_run_incomplete_count and deepseek_model_call_incomplete_count are computed. Ensure input-validation completions are excluded from those counts.
- If is_input_validation_completion needs to be imported from summarize_state_gnomes into log_feedback.py, that's fine — both are in the scripts/ directory.
- Keep the change minimal: add filtering, don't restructure the gnome computation pipeline.
- If state_run_unstarted_input_validation_error_count is already being correctly populated, just verify that it's not also inflating the incomplete counts.
