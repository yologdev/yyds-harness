Title: Close state lifecycle gaps: stash diagnostic error on DeepSeek transport failures
Files: src/deepseek.rs
Issue: none
Origin: planner

Evidence:
- YOUR TRAJECTORY graph pressure: "Close yyds state and model lifecycle gaps (state_run_unmatched_non_validation_completed_count=3): Lifecycle causes: state_unmatched/open_after_FailureObserved=3"
- `record_deepseek_transport_failure` (src/deepseek.rs:1011-1033) emits FailureObserved but does NOT call `stash_diagnostic_error` — the error context is lost and when the process later exits via `exit_with_state`, the RunCompleted payload won't carry the transport failure reason.
- The panic hook (src/state.rs:59-65) already correctly emits FailureObserved followed by RunCompleted. The transport failure path is the only FailureObserved emitter that doesn't wire into the RunCompleted diagnostic-error pipeline.

Edit Surface:
- src/deepseek.rs

Verifier:
- cargo test deepseek state -- --test-threads=1
- cargo check

Fallback:
- If `stash_diagnostic_error` is already called before `record_deepseek_transport_failure` at every call site, or if the function signature changed, write an obsolete-task note. Verify by searching: `rg -n 'record_deepseek_transport_failure\|stash_diagnostic_error' src/`

Objective:
Ensure that when `record_deepseek_transport_failure` records a FailureObserved event, the error text is also stashed via `stash_diagnostic_error` so that the eventual RunCompleted (emitted by `exit_with_state` or `mark_run_completed_with_error`) carries the transport failure context. This closes the lifecycle gap where runs end with FailureObserved as the last event.

Why this matters:
The trajectory shows 3 runs with `open_after_FailureObserved` — runs where FailureObserved was the last lifecycle event recorded. These are likely transport-failure runs where the error was recorded but the RunCompleted payload had no diagnostic context because `stash_diagnostic_error` was never called. Closing this gap means `state doctor` and the dashboard can correctly classify these as error-completed runs instead of orphaned runs. It also means `state crashes` can surface transport failure reasons.

Success Criteria:
- `record_deepseek_transport_failure` calls `crate::state::stash_diagnostic_error` with a summary of the transport error before or after emitting FailureObserved.
- Existing tests in `src/state.rs` that verify RunCompleted carries diagnostic errors still pass.
- No new clippy warnings.

Verification:
- cargo test deepseek::tests -- --test-threads=1
- cargo test state -- --test-threads=1
- cargo clippy -- -D warnings 2>&1 | grep -v "warning: function\|warning: struct\|warning: variant" | head -5

Expected Evidence:
- After this fix lands, the next trajectory should show `state_run_unmatched_non_validation_completed_count` decreasing (fewer open-after-FailureObserved runs).
- Transport-failure runs in `state crashes` should now show a diagnostic error message in the RunCompleted payload.

Implementation Notes:
- The fix is a one-line addition to `record_deepseek_transport_failure`: call `crate::state::stash_diagnostic_error(...)` with a summary that includes `source`, `model`, and a truncated `error_text`.
- `stash_diagnostic_error` is already public (src/state.rs:85). It stores a string that `mark_run_completed_with_error` picks up as `error_detail`.
- The `error_text` parameter may be long (full API error response). Truncate to 300 chars for the stashed diagnostic, since `stash_diagnostic_error` is meant for crash analysis summaries, not full error bodies. The full error is already in the FailureObserved payload.
- Do not change the function signature. Do not add new dependencies.
