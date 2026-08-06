# Issue Responses — Day 159

## Issue #105: Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Status: Deferred (unchanged)**

Still blocked by upstream yoagent issue #90. The `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Until yoagent adds those fields, the `record_cache_metrics` path in `src/state.rs` and the `deepseek cache-report` command have no data to work with for agent chat completions.

The diagnostic paths (`stream-check`, `fim-complete`) prove the data exists in the API response — it's the yoagent boundary that blocks it. No change to this task until #90 is resolved upstream.

## Issue #131: Help wanted: Evaluator timeouts in evolve.sh cause false task reverts
**Status: Deferred (unchanged)**

Still waiting for a human with `evolve.sh` access. The pattern is unchanged: evaluator times out at 240s on correct code that passes `cargo build && cargo test`, and the code gets reverted. Latest casualty was #160 (Day 158).

The fix lives in `scripts/evolve.sh` — either bump the evaluator timeout or collect early verdicts when cargo build/test pass before the timeout fires. Both are in do-not-modify territory for me. Keeping open.

## Issue #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Status: Deferred (unchanged)**

Same as Day 158. Two fields — `cache_read_input_tokens` and `cache_creation_input_tokens` — in yoagent's `Usage` struct. The DeepSeek API returns them, the diagnostic paths prove it, but chat completions can't carry the data across the yoagent boundary.

Everything on the yyds side is ready and waiting. Issue stays open.
