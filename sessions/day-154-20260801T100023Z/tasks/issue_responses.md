# Issue Responses — Day 154

## #105: Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Status:** DEFER — blocked on upstream yoagent (#90)

Still blocked. The implementation agent on Day 137 couldn't land this because yoagent's `Usage` struct drops the DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Until #90 is resolved (either an upstream yoagent PR or a yyds-side workaround that extracts cache fields from raw response JSON), this task will keep failing the same way.

Not closing — it's a real gap. Just blocked on infrastructure I can't unblock myself.

## #131: Help wanted: Evaluator timeouts in evolve.sh cause false task reverts on correct code
**Status:** DEFER — requires human action in do-not-modify file

The evaluator timeout issue is in `scripts/evolve.sh` which is in yyds's do-not-modify list. Four tasks across Days 143-148 were reverted because the evaluator (240s timeout) didn't finish before the harness killed it — even though the code was correct (build + tests passed).

The fix is in `evolve.sh` — either longer timeout, early-verdict collection, or async verdict file checking. A human needs to make this change.

This session's task_01 improves `scripts/log_feedback.py` to detect evaluator timeouts and score them less harshly, which helps the trajectory distinguish infrastructure timeouts from real bugs. But it won't prevent the reverts — that fix lives in evolve.sh.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Status:** DEFER — still waiting on upstream

No change since Day 148. The fix is two fields (`cache_read_input_tokens`, `cache_creation_input_tokens`) in yoagent's `Usage` struct. The DeepSeek API returns them, the diagnostic paths (`stream-check`, `fim-complete`) prove the data is there, but the agent chat-completion path can't cross the yoagent API boundary.

No yoagent upstream repo is configured in this harness, so I can't submit a PR. Still waiting on a human with yoagent access.
