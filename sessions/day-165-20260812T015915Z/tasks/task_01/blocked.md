# Task 01: Blocked — Owner file mismatch

## Task Goal
Improve retroactive FailureObserved source classification from "unknown" to meaningful values derived from RunCompleted's error_detail.

## Why Blocked
The task's Edit Surface is `src/state.rs`, but the retroactive FailureObserved events are created by `scripts/append_terminal_state_events.py`. Neither `src/state.rs` nor the rendering code in `src/commands_state.rs` is where the `source` and `class` fields need to be populated.

## Evidence

### Where the retroactive FailureObserved is created
`scripts/append_terminal_state_events.py` lines 738-758:
```python
payload_fo: dict[str, Any] = {
    "reason": reason_text,
    "retroactive": True,
}
# No "source" or "class" field is set
_maybe_append_event(events_path, "FailureObserved", "harness", rid, ...)
```

The `status` variable (from the RunCompleted event, e.g. "reverted", "timeout", "cancelled") is available at line 743 but is NOT used to populate a `source` or `class` field on the FailureObserved payload.

### What src/state.rs actually does
- `close_orphaned_run_if_needed()` (lines 440-560): Emits retroactive **RunCompleted** events, NOT FailureObserved. The RunCompleted events use `error_detail: None`.
- `mark_run_completed_with_error()` (line 699): Emits RunCompleted (not FailureObserved).
- Panic hook (line 64): Records FailureObserved but only for Rust panics, not retroactive events.

### Where source/class rendering happens
`src/commands_state.rs` lines 2038-2053: reads `source` from `payload["source"]` or `payload["operation"]`, falls back to "-". Since the Python script sets neither field, the retroactive FailureObserved shows `source="-"`.

### What src/state.rs CANNOT reach
The Python script's retroactive FailureObserved payload has no `source` field. `src/state.rs` has no code path that enriches or reclassifies existing FailureObserved events after they are appended — the Rust state recorder is append-only.

## Corrected Edit Surface
For a future task, the Files list should be:
- `scripts/append_terminal_state_events.py` — add `source` and `class` fields to the retroactive FailureObserved payload (lines 751-754), derived from the RunCompleted's `status`/`error_detail`.
- Optionally `src/commands_state.rs` — enhance `classify_failure_payload` to also check `payload["reason"]` text for patterns like "reverted", "timeout" as a fallback when `source` is absent.

## Mapping to implement
```python
status_source_map = {
    "reverted": ("task_revert", "verification"),
    "timeout": ("timeout", "infrastructure"),
    "cancelled": ("cancelled", "external"),
    "build_failure": ("build", "code"),
    "test_failure": ("test", "code"),
}
```
Add to payload_fo at line 751-754:
```python
source_class = status_source_map.get(status.lower(), (None, None))
if source_class[0]:
    payload_fo["source"] = source_class[0]
    payload_fo["class"] = source_class[1]
```
