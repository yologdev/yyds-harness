# Issue Responses — Day 158

## Issue #105 (agent-self): Task reverted — Record DeepSeek prompt cache metrics
**Decision:** defer
**Reason:** Blocked by yoagent upstream. Issue #90 tracks the root cause:
yoagent's `Usage` struct drops `cache_read_input_tokens` and
`cache_creation_input_tokens` from DeepSeek API responses. Until those two
fields land in yoagent, the cache metrics pipeline on the yyds side has nowhere
to read from for agent chat completions. The diagnostic paths (stream-check,
fim-complete) already work because they parse raw JSON.

No response needed this session — nothing has changed.

## Issue #131 (help-wanted): Evaluator timeouts cause false task reverts
**Decision:** keep open, still waiting
**Reason:** The fix lives in `scripts/evolve.sh` (do-not-modify territory).
Three more tasks have been killed by evaluator timeouts since the last check.
Pattern remains: `cargo build && cargo test` passes, evaluator times out
without verdict, code gets reverted. Still needs a human to either increase
the timeout or implement early-verdict collection.

No response needed this session — the issue already has the latest status.

## Issue #90 (help-wanted): yoagent Usage struct drops DeepSeek cache fields
**Decision:** keep open, still waiting
**Reason:** Two fields. `cache_read_input_tokens` and `cache_creation_input_tokens`.
The diagnostic paths prove the data exists. Everything on the yyds side is ready.
Still waiting for a human with yoagent repo access. Blocking #105.

No response needed this session — nothing has changed.
