Title: Capture DeepSeek transport failures as diagnostic errors
Files: src/deepseek.rs
Issue: none
Origin: planner

Objective:
Wire the crash diagnostic reporter into DeepSeek transport failure paths so
that protocol-level errors (context length, timeouts, network failures,
malformed responses) are captured and visible via /state crashes.

Why this matters:
The DeepSeek transport layer in deepseek.rs already has detailed error
classification (DeepSeekTransportErrorClass) and state event recording
(record_deepseek_transport_failure). But it doesn't stash diagnostic errors
via stash_diagnostic_error(). When a transport failure causes a session to
crash — as many did during Days 100-102 — there's no crash-level diagnostic
to inspect afterward.

Adding a stash_diagnostic_error() call in record_deepseek_transport_failure
closes this gap. After this change, every DeepSeek transport failure is both
logged as a state event AND available as a diagnostic error for crash analysis.
This is a 3-line change with outsized diagnostic impact.

Success Criteria:
- record_deepseek_transport_failure() calls stash_diagnostic_error() with
  a descriptive message including source, model, and error classification
- The stashed error is visible via /state crashes
- cargo build && cargo test pass with no regressions
- Existing DeepSeek transport failure tests still pass

Verification:
- cargo build --lib
- cargo test --lib -- deepseek
- cargo test --lib -- state (crash reporter tests)
- cargo test (full suite)

Expected Evidence:
- State events: CrashDiagnosticStashed events appear for transport failures
- /state crashes output includes DeepSeek transport errors
- No behavior change for successful API calls

---

## What to do

### 1. Add stash_diagnostic_error to record_deepseek_transport_failure

In `src/deepseek.rs`, in the function `record_deepseek_transport_failure` (around
line 1003), add a call to `crate::state::stash_diagnostic_error()` with a
descriptive message that includes:
- The source (what operation triggered it)
- The model being used
- A truncated version of the error text (first 200 chars max)
- The classified error type if available

Make the message consistent with the pattern used in src/lib.rs:
"yyds deepseek: transport failure ({source} on {model}): {error_summary}"

### 2. Keep it minimal

- Add exactly ONE stash_diagnostic_error call in record_deepseek_transport_failure
- Don't add stash calls to other error paths in deepseek.rs
- Don't refactor error handling — just add the diagnostic stash
- The stash should happen AFTER the existing state::record call, not instead of it
- Use the classify_transport_error result to add the error class to the message

### 3. Example

```rust
pub fn record_deepseek_transport_failure(source: &str, model: &str, error_text: &str) {
    let policy = active_harness_genome().transport_policy;
    let status = extract_deepseek_transport_status(error_text);
    let class = classify_transport_error(status, error_text);
    
    crate::state::record(
        crate::state::EventType::FailureObserved,
        crate::state::Actor::Harness,
        deepseek_transport_failure_state_payload(
            source, model, error_text,
            policy.max_retries, &policy,
        ),
    );
    
    // NEW: also stash as diagnostic error for crash analysis
    let summary = if error_text.len() > 200 {
        &error_text[..error_text.ceil_char_boundary(200)]
    } else {
        error_text
    };
    crate::state::stash_diagnostic_error(
        &format!("yyds deepseek: transport failure ({source} on {model}, {class:?}): {summary}")
    );
}
```

Note: use `error_text.char_indices().take_while(|(i, _)| *i < 200).map(|(_, c)| c).collect::<String>()` or similar safe truncation to avoid byte-index panics on multi-byte characters.

### 4. Import check

`stash_diagnostic_error` is in `crate::state`. Verify the import path works —
deepseek.rs is at the crate root level so `crate::state::stash_diagnostic_error`
should resolve. If not, add `use crate::state;` at the top.
