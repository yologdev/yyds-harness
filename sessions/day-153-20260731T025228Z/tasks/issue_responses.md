# Issue Responses — Day 153

## Agent-Self Issues

### #135: Task reverted: Break self-referential planning fallback
**Action: implement as task_01**

The previous attempt was reverted due to evaluator timeout, not wrong code. The fix is re-scoped to the exact same ~5-10 line change in `scripts/preseed_session_plan.py`: wire `_healthy_codebase_fallback()` into the no-candidates path when analysis-only pressure is active. If evaluator timeout kills it again, the code is still correct — the problem is infrastructure, not the fix.

### #134: Task reverted: Close harness-internal model lifecycle gap
**Action: implement as task_02**

The previous attempt was blocked because the implementation agent got lost in broad analysis across multiple modules (state.rs, append_terminal_state_events.py, summarize_state_gnomes.py, extract_trajectory.py). Re-scoped to a single-file, ~3-line change: filter retroactive harness-internal ModelCallCompleted events from the gnome counter in `summarize_state_gnomes.py`. No Rust changes needed.

### #105: Task reverted: Record DeepSeek prompt cache metrics
**Action: defer — blocked on #90 (yoagent upstream)**

This remains blocked until yoagent's `Usage` struct exposes `cache_read_input_tokens` and `cache_creation_input_tokens`. The diagnostic paths (`stream-check`, `fim-complete`) already prove the data is available in DeepSeek API responses — the bottleneck is the yoagent API boundary. See #90 for status.

## Help-Wanted Issues

### #131: Evaluator timeouts in evolve.sh cause false task reverts
**Action: still waiting on human**

Two more tasks (#144, #135) have been killed by evaluator timeout since my last update. The pattern is consistent: code passes `cargo build && cargo test`, evaluator times out at 240s before writing a verdict, code gets reverted. I can improve the diagnostic detection in `log_feedback.py` (which I did on Day 143 with `evaluator_timeout_with_passing_impl_count`), but I cannot fix the root cause in `evolve.sh` — it's in the do-not-modify list.

The 240s evaluator timeout needs to be increased, or the evaluator needs early-verdict collection so a "code looks correct" signal survives even if full analysis times out.

### #90: yoagent Usage struct drops DeepSeek cache fields
**Action: still waiting on upstream**

No change since Day 148. The fix is two fields in yoagent's `Usage` struct: `cache_read_input_tokens: Option<u32>` and `cache_creation_input_tokens: Option<u32>`. Once those land in a yoagent release, #105 unblocks and yyds gets real prompt-cache observability for agent chat completions.
