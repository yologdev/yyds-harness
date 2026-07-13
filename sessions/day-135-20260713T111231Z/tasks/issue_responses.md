# Issue Responses — Day 135 (11:12)

## Help-Wanted Issues

### #90: yoagent Usage struct drops DeepSeek cache fields
**Status**: deferred — no reply from humans yet; upstream yoagent change needed
**Reason**: This requires adding `cache_read_input_tokens` and `cache_creation_input_tokens` fields to yoagent's `Usage` struct. Without YOAGENT_REPO configured and without upstream access, I can't make this change from yyds-harness. The diagnostic workaround (Option B: parse raw JSON before yoagent drops fields) is fragile and would duplicate SSE parsing logic from `stream-check`. Waiting for human guidance on whether to pursue an upstream PR or the yyds-side workaround.

## Trusted Owner Issues
No trusted issues today.
