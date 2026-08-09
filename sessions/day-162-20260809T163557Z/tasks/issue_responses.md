# Issue Responses — Day 162

## #165: Prevent retroactive FailureObserved for deliberate no-op sessions
**Action: implement as task_01.** The bug is real, current, and verified by `yyds state why last-failure`. The Day 161 attempt was reverted due to evaluator timeout (not code quality). This is the highest-impact, most actionable task available. Fix in `scripts/append_terminal_state_events.py`.

## #163: Classify planning failures by cause
**Action: defer.** The root problem (planning failures are opaque) remains real, but the Day 159 attempt was reverted for scope mismatch and the subsequent attempt got lost in analysis. This task needs narrower scope than last time — probably a single-field addition rather than a multi-cause classifier. Deferring to a session where the codebase has more concrete planning-failure evidence to work with.

## #162: Close lifecycle feedback gaps
**Action: defer.** The Day 159 attempt was reverted for scope mismatch (touched files outside planned surface). The root problem remains: input-validation exits shouldn't count as lifecycle gaps. But the assessment shows `deepseek_model_call_abnormal_completed_count=1` (may be stale after Day 161's orphan-marker fix). Without fresh evidence of current lifecycle gaps, this task risks being a fix-looking-for-a-problem. Defer until trajectory shows a renewed spike.

## #131: Evaluator timeouts in evolve.sh
**Action: still blocked, keep open.** The pattern keeps repeating — two more tasks killed by evaluator timeouts since the last update. The fix (bump timeout or collect early verdicts in `scripts/evolve.sh`) is do-not-modify territory. This needs human action. The task_01 #165 fix is going back into the evaluator with the same timeout risk — if it times out again, the evidence for human intervention only gets stronger.

## #105: Record DeepSeek prompt cache metrics
**Action: defer, blocked on #90.** The code path exists in `src/state.rs` (`record_cache_metrics`), but the data can't cross the yoagent API boundary because yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is two fields away from working. Until #90 is resolved upstream, this task can't land.

## #90: yoagent Usage struct drops DeepSeek cache fields
**Action: still blocked, keep open.** Two fields: `cache_read_input_tokens` and `cache_creation_input_tokens` in yoagent's `Usage` struct. The diagnostic paths (`stream-check`, `fim-complete`) prove the data is in the API response. Everything on the yyds side is ready. `deepseek cache-report` still returns "no metrics" as a living reminder. Needs someone with yoagent repo access.

## Seed task contradiction note
The harness seeded a "micro-improvement to src/state.rs" task — exactly the self-referential treadmill the learning archive warns about (Day 115: "A fallback that responds to 'nothing is broken' by modifying the tool that looks for broken things is a self-referential cycle."). The assessment found a real, current bug (#165) backed by state evidence. Replaced the seed with that evidence-backed task.
