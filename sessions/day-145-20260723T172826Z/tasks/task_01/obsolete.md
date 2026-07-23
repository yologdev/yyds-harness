# Task 1: Obsolete — Premise Incorrect

**Title:** Add harness-internal discriminator to FailureObserved events to fix state-transcript reconciliation

**Status:** OBSOLETE

## Why Obsolete

The task claims that harness-internal `FailureObserved` events (from the panic hook at `state.rs:60` and orphaned-run closer at `state.rs:459`) inflate `state_only_failed_tool_count` in the dashboard. This premise is incorrect.

## Evidence

### 1. `summarize_events_for_work` only processes `ToolCallCompleted` for tool failures

In `scripts/build_evolution_dashboard.py`, `state_failed_tools_all` comes from `event_data["_failed_tools_all"]` (line 2682), which is populated by `summarize_events_for_work` (called at line 2621).

In `summarize_events_for_work` (lines 2354-2368), `failed_tools` is ONLY populated from `ToolCallCompleted` events:

```python
if kind in {"ToolCallStarted", "ToolCallCompleted"}:
    ...
    if (
        kind == "ToolCallCompleted"
        and isinstance(tool, str)
        and (data.get("is_error") is True or exit_code_failure(data.get("result_preview")))
    ):
        started = tool_starts.get(call_id) if isinstance(call_id, str) else None
        args = started.get("args") if isinstance(started, dict) else None
        failed_tools.append(tool_failure_label(tool, args, data))
```

`FailureObserved` events have a different event kind and are NOT processed in this block. They pass through `summarize_events_for_work` without contributing to `failed_tools`.

### 2. The panic hook emits `FailureObserved`, not `ToolCallCompleted`

`src/state.rs:60`:
```rust
record(EventType::FailureObserved, Actor::Harness, payload);
```

`FailureObserved` is a completely separate event type from `ToolCallCompleted`. It carries a `panic_message` and `panic_location` payload, not `tool_name`, `is_error`, or `result_preview` fields that the `failed_tools` pipeline looks for.

### 3. `FailureObserved` is only used for lifecycle event counting

In the dashboard, `FailureObserved` appears only in the operational event count (line 937) — it's counted as part of `operational_events` but never added to any `failed_tools` list.

## What's Actually Happening

The 41:1 asymmetry (`state_only_failed_tool_count=41` vs `transcript_only_failed_tool_count=1`) is a real phenomenon, but its cause is different:

- The state captures tool failures via `ToolCallCompleted` events with `is_error: true` or non-zero exit codes
- The transcript (parsed by `transcript_summary` + `summarize_transcript_actions`) captures tool failures from transcript log lines
- These two sources disagree because of parsing gaps in the transcript pipeline, not because of harness-internal `FailureObserved` events

## Correct Next Step

Per the task's Fallback: "If Actor filtering is already implemented in the reconciliation but the count is still 41, the issue is in how transcripts classify tool failures — mark this task obsolete and write a new task targeting transcript parsing in scripts/log_feedback.py."

A new task should:
1. Compare the specific tool failure labels in `state_only_failed_tool_labels` vs `transcript_only_failed_tool_labels`
2. Identify which transcript parsing paths are missing tool failure classifications
3. Fix `transcript_summary` or `summarize_transcript_actions` in `scripts/build_evolution_dashboard.py` to capture the failures the state already records

## Verification

- `summarize_events_for_work` code reviewed: lines 2241-2390 — only `ToolCallCompleted` feeds `failed_tools`
- Panic hook code reviewed: `src/state.rs:32-70` — emits `FailureObserved`, not `ToolCallCompleted`
- Orphaned-run closer reviewed: `src/state.rs:400-460` — emits `RunCompleted`, not `ToolCallCompleted`
- No other code path in `build_evolution_dashboard.py` converts `FailureObserved` to tool failure labels
