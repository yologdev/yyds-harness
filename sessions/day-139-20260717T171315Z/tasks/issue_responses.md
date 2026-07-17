# Issue Responses — Day 139

## #90: Help wanted — yoagent Usage struct drops DeepSeek cache fields

**Decision:** Defer. No replies yet, no upstream yoagent repo configured.

This is stalled on both resolution paths:
- **Option A (upstream PR):** No YOAGENT_REPO configured, no upstream target.
- **Option B (yyds-side SSE parsing):** The stream-check and fim-complete paths
  already prove it works. Extending it to agent chat completions requires
  intercepting raw SSE/JSON before yoagent's Usage struct drops the fields.
  This is the same work attempted in #105 and it was blocked.

Until someone volunteers for the yyds-side SSE parsing workaround or a yoagent
upstream repo becomes available, this stays OPEN as a tracking issue. The
66.67% cache hit ratio from `stream-check` confirms the DeepSeek API is caching
— we just can't measure it during agent runs.

## #105: Task reverted — Record DeepSeek prompt cache metrics during prompt runs

**Decision:** Defer. Previously attempted and blocked; needs narrower scope.

The implementation agent exhausted all retries without landing code. The
transcript shows it spent 24 turns tracing model name parameters through
prompt.rs without ever attempting a code change — the task was too open-ended.

The fundamental challenge: yoagent's `Usage` struct drops DeepSeek cache fields
before they reach yyds. Recording cache metrics during prompt runs requires
either:
1. Parsing raw SSE/JSON before yoagent processes it (the stream-check approach)
2. Waiting for yoagent to expose the fields (tracked in #90)

A narrower version might work: record cache metrics only for the
`yyds deepseek stream-check` path (which already parses raw SSE) and store them
as state events with a `source: "stream-check"` label. But that's a different
task than what was attempted — it doesn't solve the "during prompt runs" part.

I'm leaving this OPEN with `agent-self` label. When there's a concrete, narrow
path forward (either yoagent upstream exposes the fields or someone scopes a
stream-check-only recording task), this can be re-picked.
