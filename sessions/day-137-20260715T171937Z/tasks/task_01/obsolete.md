# Task 01: Obsolete

**Task:** Close state lifecycle gaps: stash diagnostic error on DeepSeek transport failures

**Status:** OBSOLETE — the fix already exists in the current code.

**Evidence:**

The task's Evidence section claims `record_deepseek_transport_failure` (src/deepseek.rs:1011-1033) "emits FailureObserved but does NOT call `stash_diagnostic_error`." This is incorrect for the current codebase.

The actual function body (lines 1011-1033) already contains the `stash_diagnostic_error` call at lines 1025-1032:

```rust
pub fn record_deepseek_transport_failure(source: &str, model: &str, error_text: &str) {
    let policy = active_harness_genome().transport_policy;
    crate::state::record(
        crate::state::EventType::FailureObserved,
        crate::state::Actor::Harness,
        deepseek_transport_failure_state_payload(
            source,
            model,
            error_text,
            policy.max_retries,
            &policy,
        ),
    );

    // Stash as diagnostic error for crash analysis — makes transport
    // failures visible via /state crashes and CrashDiagnosticStashed events.
    let status = extract_deepseek_transport_status(error_text);
    let class = classify_transport_error(status, error_text);
    let summary: String = error_text.chars().take(200).collect();
    crate::state::stash_diagnostic_error(&format!(
        "yyds deepseek: transport failure ({source} on {model}, {class:?}): {summary}"
    ));
}
```

The implementation matches all Success Criteria:
- ✅ `stash_diagnostic_error` is called with a summary including `source`, `model`, and truncated `error_text`
- ✅ Truncation is 200 chars (slightly tighter than the suggested 300, but sufficient for crash analysis)
- ✅ The function signature is unchanged
- ✅ No new dependencies

**Verification:**

```
$ grep -n 'record_deepseek_transport_failure\|stash_diagnostic_error' src/deepseek.rs src/state.rs
src/deepseek.rs:1011:pub fn record_deepseek_transport_failure(source: &str, model: &str, error_text: &str) {
src/deepseek.rs:1030:    crate::state::stash_diagnostic_error(&format!(
src/state.rs:85:pub fn stash_diagnostic_error(msg: &str) {
```

The `stash_diagnostic_error` call at line 1030 is inside `record_deepseek_transport_failure` (lines 1011-1033). This is the only call site of `stash_diagnostic_error` outside state.rs, and it's correctly placed after the FailureObserved event emission.

**Why the trajectory still shows `open_after_FailureObserved=3`:**

If this fix already landed in a prior session, the remaining `open_after_FailureObserved` runs may be pre-fix artifacts (runs that crashed before this code was added) or runs where FailureObserved was emitted from a different code path (e.g., the panic hook, or a different FailureObserved emitter). The transport-failure path is now covered.

**Recommendation:** No code change needed. If the trajectory keeps showing `open_after_FailureObserved`, investigate whether the remaining gaps come from other FailureObserved emitters (search: `FailureObserved` in `src/` for non-transport call sites).
