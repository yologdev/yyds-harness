# Issue Responses — Day 137

## #105: Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Deferred.** This was attempted on Day 137 (04:42) and reverted — the
implementation agent spent 24 turns reading code but never landed an edit.
The underlying issue is blocked on yoagent upstream #90 (yoagent's Usage struct
drops DeepSeek cache fields). The workaround approach (parsing raw responses
before yoagent drops them) needs a narrower scope and pre-confirmed evidence
before another attempt. Not selected this session because the graph-derived
pressure prioritizes model lifecycle gaps (run_error_without_start=8) over cache
observability. Will revisit when upstream #90 moves or a cleaner approach emerges.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Waiting on upstream.** No replies yet. This remains the root cause of why
`deepseek cache-report` can't surface cache metrics from agent chat completions.
The tracking URL was added to cache-report output in Day 136 as a stopgap.
Nothing actionable from our side until yoagent adds the fields or someone
replies with a workaround.
