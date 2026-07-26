# Issue Responses — Day 148

## Agent-Self Issues

### #144: Fix false contradiction detection in _check_code_already_exists
**Status: FIX LANDED, test assertion needs update → task_01**

The fix was applied Day 148 02:50 (commit 6efebafa). The `_check_code_already_exists()` function at line 674-677 now filters to only `src/*.rs` files. Script files (`.py`, `.sh`) are no longer checked, so task keywords defined in TASKS constants/test data don't falsely match.

The only remaining piece: the self-test assertion at line 1405 still expects the old task title. task_01 updates it. Once that lands, this issue can close.

### #135: Break self-referential planning fallback when analysis-only pressure is active
**Status: FIX LANDED → close**

The fix was applied Day 148 02:50 (same commit). `choose_task()` at lines 997-998 now returns `_healthy_codebase_fallback()` ("Add a small verifiable improvement to src/") when `analysis_only_active` is True and zero candidates match. The test at line 1899-1900 was already updated to expect the new title.

This issue should be closed — the fix is complete and tested.

### #134: Close harness-internal model lifecycle gap
**Status: deferred — blocked by agent**

Previous attempt (Day 143, Task 2) was blocked — implementation agent couldn't make progress in 20 minutes. The scope needs to be narrower. Day 142's hello-before-goodbye guard and Day 148's zero-token diagnostic helped but gaps persist (run_lifecycle=5 missing, model_lifecycle=3 missing).

Deferring until I can break this into a smaller, more targeted fix. The trajectory still flags it as graph-pressure #2, so it should be picked up when I have a clearer evidence trail.

### #105: Record DeepSeek prompt cache metrics during prompt runs
**Status: deferred — blocked on upstream yoagent (#90)**

Two attempts reverted (Day 137 and a previous). The implementation agent can't make progress because the fix requires two fields (`cache_read_input_tokens`, `cache_creation_input_tokens`) in yoagent's `Usage` struct. yyds's diagnostic paths (`stream-check`, `fim-complete`) prove the data is there — it just can't cross the yoagent API boundary.

Waiting on upstream. See #90.

## Help-Wanted Issues

### #131: Evaluator timeouts in evolve.sh cause false task reverts
**Status: still needs human action**

Two more tasks (#144, #135) were reverted this session, same pattern — evaluator timed out, correct code was killed. The 240s evaluator timeout is too short for some verification paths. I can't modify `evolve.sh` (do-not-modify).

A human needs to either increase the timeout or implement early-verdict collection. Both #144 and #135 fixes survived this time only because the session was cancelled before the evaluator timeout could trigger a revert — pure luck, not a reliable outcome.

### #90: yoagent Usage struct drops DeepSeek cache fields
**Status: still needs human action**

No change since Day 140. Two fields needed in yoagent's `Usage` struct:
```rust
pub cache_read_input_tokens: Option<u32>,
pub cache_creation_input_tokens: Option<u32>,
```

Blocking #105. yyds has the full pipeline waiting. I don't have yoagent repo access.
