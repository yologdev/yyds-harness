Title: Make `state why last-failure` find RunCompleted(error) events

Files: src/commands_state.rs

Issue: none

Origin: planner

Objective:
  `state why last-failure` currently returns "no state event found for 'last-failure'"
  because `is_failure_event_type` only matches `FailureObserved`, `JsonOutputFailure`,
  and `ToolSchemaFailure` — but NOT `RunCompleted` events with `"error"` status.
  The most common failure evidence (RunCompleted errors) is invisible to the
  diagnostic query tooling. Fix this so the query tool actually works.

Why this matters:
  - 10 crashes this session, all recorded as RunCompleted(error) events.
  - `state why last-failure` searches for events matching `is_failure_event_type`
    which excludes RunCompleted entirely.
  - The query tool was built for `FailureObserved` events (from panic hooks)
    but those may not persist to disk when the panic crashes before state flush.
  - RunCompleted(error) events ALWAYS persist because they're written by
    `mark_run_completed_with_error` during `exit_with_state`.
  - After Task 01 (panic hook → stash_diagnostic_error), RunCompleted events
    will also carry `error_detail` with the actual panic message.
  - Making `last-failure` recognize RunCompleted(error) closes the loop:
    data captured (Task 01) + data findable (Task 02).

Success Criteria:
  - `state why last-failure` finds the most recent RunCompleted event with
    `"error"` status, even when no FailureObserved/JsonOutputFailure/ToolSchemaFailure
    events exist
  - When both a FailureObserved and a RunCompleted(error) exist, the most
    recent failure-type event is returned (whichever came last)
  - Existing behavior for explicit event IDs is unchanged
  - New unit test verifies the behavior

Verification:
  - cargo build
  - cargo test --lib -- commands_state (or the specific test)
  - Manual: `yyds state why last-failure` should show a RunCompleted(error) event
    when one exists in the log

Expected Evidence:
  - After a crash session, `state why last-failure` shows the RunCompleted event
    with `status: "error"`, the error message, and any `error_detail`
  - `state crashes` count should align with `state why last-failure` output
  - Task lineage should show this as a state query improvement

---

## Implementation Plan

### Step 1: Modify `is_failure_event_type` to include RunCompleted

In `src/commands_state.rs` line 13930-13935, `is_failure_event_type` currently:

```rust
fn is_failure_event_type(kind: &str) -> bool {
    matches!(
        kind,
        "FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure"
    )
}
```

Add `"RunCompleted"`:

```rust
fn is_failure_event_type(kind: &str) -> bool {
    matches!(
        kind,
        "FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure" | "RunCompleted"
    )
}
```

BUT: This alone would match ALL RunCompleted events, including successful ones.
We need to also check that the payload has `status: "error"`.

### Step 2: Refine the `last-failure` lookup

The `find_target_event` function (line 13916-13928) currently does:

```rust
if id == "last-failure" {
    return events.iter().rev().find(|event| {
        event
            .get("event_type")
            .and_then(|v| v.as_str())
            .map(is_failure_event_type)
            .unwrap_or(false)
    });
}
```

Modify the `"last-failure"` branch to also check RunCompleted payloads:

```rust
if id == "last-failure" {
    return events.iter().rev().find(|event| {
        let event_type = event.get("event_type").and_then(|v| v.as_str()).unwrap_or("");
        if matches!(event_type, "FailureObserved" | "JsonOutputFailure" | "ToolSchemaFailure") {
            return true;
        }
        if event_type == "RunCompleted" {
            // Only match RunCompleted events with error status
            return event
                .get("payload")
                .and_then(|p| p.get("status"))
                .and_then(|v| v.as_str())
                .map(|s| s == "error")
                .unwrap_or(false);
        }
        false
    });
}
```

This preserves the existing behavior for FailureObserved/JsonOutputFailure/ToolSchemaFailure
while also finding RunCompleted events with error status.

Note: we should NOT add `"RunCompleted"` to `is_failure_event_type` because
that function is used elsewhere (e.g., in `build_why_report` line 2584 for
dispatching to `append_failure_explanation`). Adding RunCompleted there would
change behavior for successful RunCompleted events in the why report. Instead,
keep `is_failure_event_type` unchanged and handle the RunCompleted case
specially only in the `last-failure` lookup.

### Step 3: Add a test

Add a test in `src/commands_state.rs` near the existing tests for `find_target_event`:

```rust
#[test]
fn test_find_last_failure_finds_run_completed_error() {
    // Create events where the most recent "failure" is a RunCompleted(error)
    let events = vec![
        json!({"event_type": "RunStarted", "event_id": "run-1"}),
        json!({"event_type": "RunCompleted", "event_id": "rc-1", "payload": {"status": "completed"}}),
        json!({"event_type": "RunStarted", "event_id": "run-2"}),
        json!({"event_type": "RunCompleted", "event_id": "rc-2", "payload": {"status": "error", "error": "exit code 1"}}),
    ];

    let found = find_target_event(&events, "last-failure");
    assert!(found.is_some(), "should find RunCompleted(error) as last failure");
    assert_eq!(
        found.and_then(|e| e.get("event_id").and_then(|v| v.as_str())),
        Some("rc-2")
    );
}

#[test]
fn test_find_last_failure_prefers_explicit_failure_over_run_completed_error() {
    // FailureObserved should be found even if RunCompleted(error) is more recent
    let events = vec![
        json!({"event_type": "FailureObserved", "event_id": "fo-1", "payload": {"failure_class": "rust_panic"}}),
        json!({"event_type": "RunCompleted", "event_id": "rc-1", "payload": {"status": "error", "error": "exit code 1"}}),
    ];

    let found = find_target_event(&events, "last-failure");
    assert!(found.is_some());
    // The most recent should be RunCompleted since we iterate in reverse
    assert_eq!(
        found.and_then(|e| e.get("event_id").and_then(|v| v.as_str())),
        Some("rc-1")
    );
}

#[test]
fn test_find_last_failure_skips_run_completed_success() {
    let events = vec![
        json!({"event_type": "RunCompleted", "event_id": "rc-1", "payload": {"status": "completed"}}),
    ];

    let found = find_target_event(&events, "last-failure");
    assert!(found.is_none(), "should not find RunCompleted(completed) as failure");
}
```

### Step 4: Verify

```bash
cargo build
cargo test --lib -- commands_state
cargo test --bin yyds -- --test-threads=1
cargo clippy --all-targets -- -D warnings
```

### Risks
- `is_failure_event_type` is used in `build_why_report` at line 2584 for
  dispatching to `append_failure_explanation`. Do NOT add RunCompleted there
  because `append_failure_explanation` likely assumes FailureObserved-specific
  payload structure. Keep the change scoped to `find_target_event` only.
- The test uses `json!()` macro from `serde_json`. Verify this is available
  in the test module (it's used extensively in `commands_state.rs` tests already).
