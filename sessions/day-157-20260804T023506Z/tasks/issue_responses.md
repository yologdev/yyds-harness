# Issue Responses — Day 157

## #152 — Distinguish cancelled runs from error exits
**Decision: defer**

This was reverted Day 154 (evaluator timeout) and again hasn't landed. The root blocker is #131 — the evaluator timeout in evolve.sh kills verification before a verdict arrives. Until #131 is fixed, any script-level task touching append_terminal_state_events.py risks the same timeout-revert cycle.

The underlying problem (cancelled runs polluting failure signal) is real and the trajectory keeps surfacing it, but re-attempting with the same evaluator-timeout risk isn't productive. Keeping open until #131 moves.

## #105 — Record DeepSeek prompt cache metrics during prompt runs
**Decision: defer**

Still blocked by upstream #90 (yoagent Usage struct drops cache fields). The diagnostic paths (stream-check, fim-complete) prove the data exists in DeepSeek responses, but the primary agent chat-completion path can't access it until yoagent exposes `cache_read_input_tokens` and `cache_creation_input_tokens` in its `Usage` struct.

The entire yyds-side pipeline is ready (record_cache_metrics, cache-report command, gnome KPIs, zero-vs-None test coverage). This stays open until the two-field upstream change lands.

## #131 — Evaluator timeouts in evolve.sh cause false task reverts
**Decision: keep open (needs human)**

Nothing new this session. The pattern is now well-documented: evaluator times out before writing a verdict, correct code that passes cargo build && cargo test gets reverted. Three tasks killed by timeouts in the last week. The fix lives in scripts/evolve.sh (do-not-modify territory): either bump the evaluator timeout or collect early verdicts.

## #90 — yoagent Usage struct drops DeepSeek cache fields
**Decision: keep open (needs upstream)**

No change. Two fields. That's the whole ask. Blocking #105 and all DeepSeek cost observability work. The diagnostic paths prove the data is there — we just can't get it through the yoagent API boundary into state events.
