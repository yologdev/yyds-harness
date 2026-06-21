# Task 01: OBSOLETE — Add run-ID and timestamp detail to cold-start state diagnostics

**Date**: 2026-06-21 (Day 113, 04:19)
**Status**: Obsolete — current code already satisfies the task

## What the Task Asked For

Add run IDs and timestamps to `yyds state why last-failure` output when no
completed failures exist, so users can immediately run `yyds state trace <run_id>`.

## Proof the Task is Already Satisfied

Two independent code paths already include run IDs and timestamps:

### Path 1: `build_why_report` error message (`src/commands_state.rs` lines 3139–3170)

When `run_completed_count == 0 && run_started` and `id == "last-failure"`, the
error message includes:

```
Incomplete run(s):
  run=<run_id>  started=<timestamp>
```

The code at lines 3160–3168 explicitly gathers the most recent 5 incomplete
RunStarted events, extracts their `run_id` and `timestamp_ms`, and formats them
into the error string.

### Path 2: `handle_why` fallback (`src/commands_state.rs` lines 1156–1181)

When `build_why_report` returns `Err` for `last-failure`, `handle_why` calls
`find_incomplete_runs()` (line 1157) and prints:

```
  <run_id> — started <duration> ago, no RunCompleted event
```

With follow-up: `yyds state trace <run-id> for details` (line 1180).

### Existing Tests Confirm This

The test `why_report_suggests_alternatives_when_no_failure_found` at line 17478
verifies all three cases:

1. **Empty state** (line 17480): error message with "no state event found"
2. **All-green sessions** (line 17488): error message with "successful" + diagnostic suggestions
3. **Active incomplete run** (line 17512): error message contains:
   - `"in progress"` (line 17519) — session-state label
   - `"run-1"` (line 17523) — **run ID**
   - `"1970-01-01"` (line 17527) — **start timestamp**
4. **Multiple incomplete runs** (line 17531): shows at most 5 runs

Additional tests:
- `why_report_finds_last_failure` (line 15658): success path with failure details
- `last_failure_includes_json_output_failures` (line 17456): JSON output failures

## Why the Seed Task's Premise Was Stale

The seed task claimed output "lacks specific run IDs and timestamps" and that a
user seeing "1 incomplete run" can't immediately run `state trace <run_id>`.

In reality, the run ID is displayed **twice** in the incomplete-run output:
once by `build_why_report` (`run=<id>`) and once by `handle_why`'s fallback
(`<id> — started X ago`). The user can copy-paste either representation
directly into `yyds state trace <run_id>`.

The assessment (line 55) correctly observed that `state why last-failure`
already notes "1 incomplete run (current session)" and suggests state trace —
the assessment just didn't check whether the specific run ID was already
included. It is.

## Conclusion

No code changes needed. The task is obsolete — run IDs and timestamps are
already present in `yyds state why last-failure` output for incomplete runs.
