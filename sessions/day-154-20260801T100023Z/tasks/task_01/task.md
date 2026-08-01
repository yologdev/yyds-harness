Title: Separate input-validation exits from real lifecycle gaps in state scripts
Files: scripts/append_terminal_state_events.py, scripts/log_feedback.py, scripts/summarize_state_gnomes.py
Issue: none
Origin: planner (refined from harness-seed)

Evidence:
- trajectory shows `deepseek_model_call_unmatched_completed_count=2` with `state_unmatched/open_after_FailureObserved=2`
- assessment confirms: "Model call lifecycle gaps — state events show runs where FailureObserved closes a run but ModelCallCompleted is missing"
- `is_input_validation_completion()` already exists in all three target files but isn't consistently applied to lifecycle classification
- Day 153's ghost call fix (summarize_state_gnomes.py) already skips synthetic ModelCallCompleted events — the remaining gap is input-validation exits being counted as real lifecycle problems
- log_feedback corrected lessons say "planner produced no usable task" — making lifecycle metrics precise helps the planner make better decisions

Edit Surface:
- scripts/append_terminal_state_events.py: ensure input-validation runs don't generate synthetic FailureObserved events, or classify them separately
- scripts/log_feedback.py: don't count input-validation lifecycle gaps toward session scoring
- scripts/summarize_state_gnomes.py: separate input-validation lifecycle metrics from real unmatched model calls in gnome output

Verifier:
- python3 -c "from scripts.append_terminal_state_events import collect_input_validation_run_ids; print('import ok')"
- python3 -c "from scripts.log_feedback import is_input_validation_completion; print('import ok')"
- python3 -c "from scripts.summarize_state_gnomes import is_input_validation_completion; print('import ok')"

Fallback:
- If all three files already correctly classify input-validation exits (verify by checking that `is_input_validation_completion` is called in the lifecycle-classification code paths), mark this task obsolete with evidence of the existing guards. Do not add redundant checks.

Objective:
Ensure input-validation pre-agent exits (fire-and-forget checks that never start an agent) are classified separately from real model-call lifecycle gaps, so state metrics and log feedback scores don't penalize sessions for expected non-agent runs.

Why this matters:
The trajectory's graph-derived pressure includes `deepseek_model_call_unmatched_completed_count=2` — model calls that appear unmatched. Some of these are from input-validation runs (pre-agent checks that exit before any agent starts). When these are counted as real lifecycle gaps, they inflate diagnostic gnomes and distort log-feedback scoring, which in turn affects task selection. Separating them makes the metrics honest.

Success Criteria:
- Input-validation runs are excluded from `deepseek_model_call_unmatched_completed_count` and similar lifecycle-gap gnomes
- Log feedback scoring does not penalize sessions for lifecycle gaps caused by input-validation exits
- `summarize_state_gnomes.py` gnome output separates "real lifecycle gaps" from "input-validation (expected)" counts

Verification:
- python3 -m py_compile scripts/append_terminal_state_events.py
- python3 -m py_compile scripts/log_feedback.py
- python3 -m py_compile scripts/summarize_state_gnomes.py

Expected Evidence:
- Future trajectory snapshots show lower `deepseek_model_call_unmatched_completed_count` (or a separate `input_validation_unmatched` counter)
- Log feedback score is not dragged down by lifecycle gaps from input-validation pre-agent runs

Implementation Notes:
- `is_input_validation_completion()` already exists in all three files — the task is to ensure it's consistently called in the lifecycle-classification code paths, not to reimplement it
- In `append_terminal_state_events.py`: the `collect_input_validation_run_ids` function (line 310) already collects these run IDs; check that it's used when deciding whether to flag a run as having a lifecycle gap
- In `log_feedback.py`: `is_input_validation_completion` is used at line 1332 and 1338 but may not cover all lifecycle-classification paths — audit all callers of lifecycle-gap metrics
- In `summarize_state_gnomes.py`: check that `is_input_validation_completion` (line 558 area, per repo map) is called when computing lifecycle gnomes
- Do NOT modify the Rust source or evolve.sh — this task is script-classification only
