# Task 01: Broaden state failure recording to capture DeepSeek transport and protocol failures

Title: Broaden state failure recording for DeepSeek transport/protocol errors
Files: src/deepseek.rs, src/state.rs
Issue: none

## Problem

The state subsystem records only 3 FailureObserved events across 124 runs. The assessment identifies this as a gap: either the system is remarkably robust, or more likely, failure recording is too narrow. DeepSeek transport errors (API failures, connection errors, status code errors), protocol check failures, and schema validation failures all exist in `deepseek.rs` but may not be flowing through to the state recorder as FailureObserved events.

The current harness has `record_failure_event` and `should_record_failure` fields on transport/protocol decision structs, plus a `record_failure_schema()` for the strict tool schema system — but these may not be consistently wired to emit `EventType::FailureObserved` via the state recorder at runtime.

## What To Do

1. **Audit `deepseek.rs`**: Find every code path where a DeepSeek transport error, protocol check failure, or schema validation failure is detected but NOT recorded to state. Key areas:
   - `classify_deepseek_transport_failure` / `extract_deepseek_transport_status` — transport-level errors
   - `validate_strict_tool_schema_suite` / `validate_strict_tool_arguments` — schema validation failures
   - `parse_json_output_attempts` — JSON output parsing failures
   - `validate_deepseek_thinking_tool_call_messages` — thinking-mode tool call mismatches
   - Any `DeepSeekTransportDecision` or `DeepSeekProtocolCheck` that returns a failure without recording it

2. **Wire recordings**: For each failure path you find that doesn't already call the state recorder, add a `record_failure` call (or use the existing state recording API) that emits a `FailureObserved` event with:
   - `source`: the function/module name where the failure was detected (e.g., "deepseek::transport", "deepseek::schema_validation")
   - `message`: the specific error message
   - `class`: the failure taxonomy (transport, protocol, schema, json_parse, thinking_mismatch)
   - `retryable`: whether the failure is retryable

3. **Add a test** that verifies at least one of these new failure paths actually records a FailureObserved event. Use the existing state test infrastructure in `state.rs` — write a test that simulates the failure condition and asserts a FailureObserved event appears.

## Verification

- `cargo build` must pass
- `cargo test` must pass
- Run `yyds state why last-failure` after a session that exercises a DeepSeek API call — verify that transport/protocol failures appear in the state log
- The new test should fail before the fix and pass after

## Sizing Note

This is a focused change to ~2 files. The audit step is the bulk of the work; the actual wiring should be small. If you find more than 3-4 unwired failure paths, pick the 2 most impactful and add a comment noting the others for a future task.
