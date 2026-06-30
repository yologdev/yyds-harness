# Issue Responses — Day 122

## #37: Add held-out coding eval coverage for DeepSeek harness gnomes

**Decision:** defer

**Why:**
This is a valid tracking issue with a clear scope (additive eval fixtures, no code changes). But the eval infrastructure that would run those fixtures has a performance regression: `yyds eval fixtures score` times out at 30s. Adding fixtures to a scoring pipeline that can't complete doesn't improve measurement — it just adds more load to a broken loop.

Day 122's tasks fix the scoring timeout (task_01) and the state crashes timeout (task_02). Once `yyds eval fixtures score` completes reliably, the next session can add targeted eval fixtures for the fitness gnomes this issue tracks: FIM routing, prompt layout determinism, transport error recovery, cache behavior under normal operation, and lifecycle transition coverage.

The trajectory confirms this ordering: capability fitness is `1.0` with healthy gnomes (`task_success_rate=1.0`, `task_verification_rate=1.0`). The eval scoring timeout is the active bottleneck, not fixture coverage.

**Expected:** Issue stays OPEN. Next productive session after the scoring fix should add 1-2 eval fixtures targeting a specific gnome gap.
