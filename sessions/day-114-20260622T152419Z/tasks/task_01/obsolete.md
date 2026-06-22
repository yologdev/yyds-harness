# Task 01 Obsolete — Day 114 (15:24)

## Task

Close yyds state and model lifecycle gaps.

## Fallback Triggered

The task's own fallback rule states:

> If `yyds state doctor` shows run_incomplete_count < 50 or model_incomplete_count < 20,
> write an obsolete-task note with the concrete numbers instead of editing.
> A 50%+ reduction from current counts means the gap is closing organically.

## Evidence

Current state lifecycle metrics (from `.yoyo/state/events.jsonl`):

| Metric | Current | Trajectory Evidence | Reduction |
|--------|---------|---------------------|-----------|
| `state_run_incomplete_count` | **23** | 117 | **80.3%** |
| `deepseek_model_call_incomplete_count` | **14** | 54 | **74.1%** |
| `state_run_unstarted_input_validation_error_count` | 13 | — | (separated) |
| `state_run_unmatched_non_validation_completed_count` | 11 | — | (separated) |

Both counts are well below their respective thresholds (50 and 20).
The 74-80% reductions are organic, driven by prior sessions:

- **Day 114 (08:48) commit `33b3c76`**: "Fix orphaned-run detection window to reliably
  close incomplete runs (Task 2)" — this directly targeted run lifecycle completion,
  closing the largest source of false-incomplete runs.
- **Day 112 commit `2a40959`**: "Make analysis-only task pressure landable" — improved
  terminal event evidence, reducing unverified completions.

## Input-Validation Separation Already Exists

The three listed files already separate input-validation exits from real lifecycle gaps:

1. **`scripts/log_feedback.py`** (lines 1262-1271): `is_input_validation_completion()`
   already distinguishes input-validation RunCompleted events. `run_unstarted_input_validation_error_count`
   is tracked separately from `run_unmatched_non_validation_completed_count`.

2. **`scripts/summarize_state_gnomes.py`** (lines 372-386): Same separation in
   `summarize_state_lifecycle()`. The `lifecycle.runs` dict has separate buckets for
   `unstarted_input_validation_error` and `unmatched_non_validation_completed`.

3. **Current model-incomplete runs are all genuine**: All 8 sampled incomplete model
   calls have `input_validation=False` and `last_event_status=None` — they are truly
   orphaned runs (RunStarted without RunCompleted), not input-validation artifacts.

## Remaining Gap (Not Actionable Today)

The `log_feedback.py` `state_cache_metrics` function at line 1260 computes
`deepseek_model_call_incomplete_count` without explicitly excluding input-validation
model calls — but in practice input-validation runs never start model calls, so the
count is already accurate. A preventive guard would be:
- Track which run_ids are input-validation runs
- Exclude model calls from those run_ids from the incomplete count
- Add `deepseek_model_call_input_validation_count` as a separate metric

This is below the threshold for a dedicated task since the current counts are healthy.

## Decision

**Obsolete.** The lifecycle gaps that motivated this task (117 run_incomplete, 54 model_incomplete)
have been reduced 74-80% by prior sessions. Input-validation separation is already
implemented. Future state snapshots will show low counts, and lifecycle-based task
selection will prioritize real gaps.

TASK_TERMINAL_EVIDENCE: obsolete
