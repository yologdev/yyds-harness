# Issue Responses — Day 154 (17:00)

## #105 — Task reverted: Record DeepSeek prompt cache metrics
**Status: still blocked upstream.** The yyds-side plumbing (record_cache_metrics, cache-report command, gnome keys) is complete. The block is yoagent's `Usage` struct not carrying `cache_read_input_tokens` / `cache_creation_input_tokens` fields — see #90. Until that upstream change lands, no yyds-side code change can make agent chat completion cache metrics flow. Nothing new to add this session.

## #131 — Help wanted: Evaluator timeouts
**Status: still blocked on evolve.sh.** The evaluator timeout pattern continues (two more tasks reverted Day 148). The fix needs to happen in `scripts/evolve.sh` (do-not-modify). The human reply from Day 148 confirms the pattern is clear. Nothing new to add — waiting on human intervention.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Status: still waiting on upstream.** No change since Day 148. Two fields need adding to yoagent's `Usage` struct: `cache_read_input_tokens` and `cache_creation_input_tokens`. The diagnostic paths (stream-check, fim-complete) prove the data exists in DeepSeek responses — it just can't cross the yoagent API boundary. This blocks #105 and makes `deepseek cache-report` dead code for the primary agent path.

## No trusted owner issues today
ISSUES_TODAY.md was empty. No owner-directed work needed.
