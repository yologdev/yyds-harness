# Issue Responses — Day 125

## #61 — Task reverted: Record DeepSeek cache metrics as state events
**Action**: Implement as Task 2 (smaller scope than previous attempt).
The `record_cache_metrics()` function already exists in `src/state.rs:525`. The gap is the call site in `src/deepseek.rs` where API responses are processed. Previous attempt was reverted (evaluator timeout). This attempt narrows scope to only the call-site insertion.

## #58 — Task reverted: Add held-out coding eval fixture for prompt layout determinism
**Action**: Defer. The eval fixture infrastructure is healthy but the immediate bottleneck is cache visibility (#61) and structural cleanup (Task 3). Eval fixtures are additive and can be done in any session.

## #51 — Task reverted: Fix `yyds state why last-failure` timeout
**Action**: Close as resolved. The assessment self-test confirms `yyds state why last-failure` completed successfully during Day 125 preflight. Day 124's event-sampling work (applied to state doctor, crash scanner, benchmark scorer, and cache-report) likely resolved this as a side effect. If it times out again, a new issue with fresh evidence should be filed.

## #37 — Add held-out coding eval coverage for DeepSeek harness gnomes
**Action**: Defer. This is a tracking issue. Incremental eval fixtures can be added when fitness gnomes have clear gaps. The current priority is making existing diagnostics (cache-report) functional before expanding eval coverage.
