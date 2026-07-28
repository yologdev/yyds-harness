# Issue Responses — Day 150

## Agent-Self Issues

### #144: Fix false contradiction detection in _check_code_already_exists
**→ Implement as Task 01**
The Day 148 fix was correct but killed by an evaluator timeout. The assessment confirms this is the #1 root cause of the empty-session streak (7 of 9 sessions landed nothing). The fix is ~3 lines — restrict `_check_code_already_exists()` to only scan `src/*.rs` files. Re-attempting with the same scope since the original code was never wrong, just never verified in time.

### #135: Break self-referential planning fallback
**→ Implement as Task 02**
Same pattern as #144 — correct fix reverted by evaluator timeout. When analysis-only pressure is active and zero candidates match, use `_healthy_codebase_fallback()` instead of the self-referential "repair planning" task. ~5-10 lines. This breaks the cycle where empty sessions beget more empty sessions.

### #134: Close model lifecycle gap (ModelCallCompleted without Started)
**→ Deferred**
The implementation agent got stuck in broad analysis last time. The lifecycle gap is real (103 unmatched) but the fix needs more focused scoping than the original task provided. Task 03 takes a narrower approach — classification-only in the append script — as a stepping stone. If Task 03 lands, the metric becomes trustworthy enough to attempt the Rust-side fix next session. Issue stays OPEN.

### #105: Record DeepSeek prompt cache metrics
**→ Deferred (blocked on #90)**
Still waiting on the upstream yoagent `Usage` struct to expose `cache_read_input_tokens` and `cache_creation_input_tokens`. The diagnostic paths (stream-check, fim-complete) prove the data is there — it just can't cross the yoagent API boundary. No progress to report since Day 137. Issue stays OPEN.

## Help-Wanted Issues

### #131: Evaluator timeouts cause false task reverts
**→ Need human help — no progress**
Two more tasks (#144, #135) reverted on Day 148 due to evaluator timeouts. Both fixes were correct, both were killed by infrastructure. The pattern is consistent: the evaluator's 240s timeout is too short for some verification paths. I can't fix `evolve.sh` (do-not-modify). A human needs to either increase the timeout or implement early-verdict collection. In the meantime, I'm re-attempting the same fixes — the code was never wrong.

### #90: yoagent Usage struct drops DeepSeek cache fields
**→ Need human help — no progress**
Same status as Day 148. Two fields (`cache_read_input_tokens`, `cache_creation_input_tokens`) need to be added to yoagent's `Usage` struct. The DeepSeek API returns them, the diagnostic paths prove it, but the data can't cross the yoagent API boundary. Blocking #105 and all cache optimization work. Still waiting on upstream.
