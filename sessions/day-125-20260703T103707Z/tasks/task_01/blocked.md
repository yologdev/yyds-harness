# Task blocked by no-progress implementation attempts

The implementation agent used the allowed attempts without landing file progress
or emitting TASK_TERMINAL_EVIDENCE.

This task should be replanned with narrower scope, clearer owning files, or
stronger pre-confirmed evidence before another implementation attempt.

Recent transcript tail:
```
So:
- If `prompt_cache_miss_tokens` is present: `usage.input = prompt_cache_miss_tokens`
- If `prompt_cache_miss_tokens` is absent: `usage.input = prompt_tokens - cache_read`

For the FIRST call with no cache: `cache_read = 0`, so `usage.input = prompt_tokens`. This is non-zero. The filter at line 566 (both are Some(0)) wouldn't trigger.

For subsequent calls with cache: `cache_read > 0`, `usage.input = prompt_tokens - cache_read` (or the explicit miss_tokens value). Either way, at least one is non-zero.

So the filter shouldn't be an issue...

Hmm, but what if yoagent's `Usage.input` starts at 0 and the `accumulate_usage` function adds it? Let me check:
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 21 ────────────────────────────────────────────────────────────╮
  ▶ $ grep -n 'fn accumulate_usage' src/prompt.rs ✓ (12ms)

── Thinking ────────────────────────────────────────────────────────────────

────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 22 ────────────────────────────────────────────────────────────╮
  ▶ read src/prompt.rs:24..34 ✓ (7ms)

── Thinking ────────────────────────────────────────────────────────────────
OK so `accumulate_usage` adds up usage correctly. If `msg_usage.input` has the miss tokens and `msg_usage.cache_read` has the hit tokens, they accumulate correctly.

Now, I'm going to take a different approach. Instead of continuing to diagnose, let me write a test that tests `record_cache_metrics` end-to-end by:

1. Setting up the global recorder
2. Calling `record_cache_metrics` with realistic data
3. Checking that the event was recorded

Actually wait, let me check how the recorder is set up in tests:
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 23 ────────────────────────────────────────────────────────────╮
  ▶ $ grep -n 'GLOBAL_RECORDER\|init_recorder\|test_recorder' src/state.rs | head -30 ✓ (13ms)

── Thinking ────────────────────────────────────────────────────────────────
Let me look at line 3323 for the test cleanup and how tests set up the recorder:
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 24 ────────────────────────────────────────────────────────────╮
  ▶ read src/state.rs:3315..3345 ✓ (7ms)

── Thinking ────────────────────────────────────────────────────────────────
OK let me take a completely different approach. Instead of continuing to analyze, let me write a concrete regression test now and then look at the overall picture one more time.

Actually, I just realized something. Let me look at what happens AFTER the cache_metrics_payload function. The filter drops events where BOTH cache_hit AND cache_miss are 0. But what about the case where:

- First call with no cache: cache_hit = 0, cache_miss = usage.input (which should be prompt_tokens from yoagent's fallback)
- But if `prompt_tokens` is also 0 in the API response, then BOTH are 0 and the event is dropped

Wait, `prompt_tokens` cannot be 0 for a real API response. OK.

Let me just approach this differently. Let me write a test that validates `record_cache_metrics` works end-to-end with the global recorder, and then add the missing `record_cache_metrics` call to the fallback path.

The concrete fix I'll make:
1. Write a regression test for `record_cache_metrics` through the global recorder
2. Add `record_cache_metrics` to the fallback path at line 1054

Wait, but the task says the issue might be in the `model` check or filter. Let me re-read the task's Evidence more carefully:

> Despite the function existing and being called, zero events land in the state store. The gap is either: (a) yoagent::Usage.cache_read is always 0 for DeepSeek (upstream mapping issue), (b) the ModelCompleted code path is never reached in the actual agent loop, or (c) the payload filter at line 566 silently drops all events.

Additional evidence:
> `cost_per_successful_task_usd` gnome has no cache data — cache optimization is flying blind.

Hmm, actually let me look at this from another angle. The `cost_per_successful_task_usd` gnome - let me check if that's reading cache metrics:
────────────────────────────────────────────────────────────────────────────────


  ╭─ Turn 25 ────────────────────────────────────────────────────────────╮
  ▶ $ grep -n 'cache\|CacheMetrics' scripts/log_feedback.py scripts/summarize_state_gn… ✓ (12ms)


```
