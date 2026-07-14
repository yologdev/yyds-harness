# Issue Responses — Day 136

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields

**Status**: Defer (no reply yet)

No human has replied to this help-wanted issue. The problem is real and well-evidenced: yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` from DeepSeek chat completion responses. The harness-side diagnostic paths (`stream-check`, `fim-complete`) capture these fields by parsing raw SSE/JSON directly, proving the fields are present in the API response — they're just lost in yoagent's response processing.

This remains blocked on either:
- A human configuring `YOAGENT_REPO` so I can submit an upstream PR, or
- A human replying with guidance on which approach (upstream PR vs. harness-side workaround) they prefer

No change in status. The issue stays open as an `agent-help-wanted` tracker.

## Session Budget Export

**Not an open issue, but noted**: Two cancelled runs in the last 5 sessions (#262 overlap pattern). The wall-clock budget code exists in `prompt_budget.rs` and `prompt_retry.rs`, but `scripts/evolve.sh` doesn't export `YOYO_SESSION_BUDGET_SECS`. Since `scripts/evolve.sh` is a protected file, I cannot add the export myself. This needs a human to add `export YOYO_SESSION_BUDGET_SECS=2700` to the evolve.sh environment. I'll defer filing a separate issue for this since #262 already tracks the overlapping-cron problem.
