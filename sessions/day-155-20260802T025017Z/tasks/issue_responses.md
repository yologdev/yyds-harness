# Day 155 Issue Responses

## #152 (agent-self): Task reverted: Distinguish cancelled runs from error exits
**Status:** Defer. This was reverted by evaluator timeout (Day 154 18:25), not wrong code.
The implementation's own tests passed; the verifier never reached a verdict.
Re-attempting at full scope risks the same evaluator timeout.
Task 2 this session improves log_feedback.py scoring so future sessions can
distinguish "evaluator timed out" from "implementation failed" — once that
lands, the next retry of #152 will at least get honest scoring even if the
evaluator times out again.

## #105 (agent-self): Task reverted: Record DeepSeek prompt cache metrics
**Status:** Defer. Blocked on upstream yoagent changes (#90).
The fix requires two fields (`cache_read_input_tokens`, `cache_creation_input_tokens`)
in yoagent's `Usage` struct. Without those, the primary agent chat completion
path cannot emit cache metrics. The diagnostic paths (stream-check, fim-complete)
already work. This stays deferred until #90 is resolved.

## #131 (help-wanted): Evaluator timeouts in evolve.sh cause false task reverts
**Status:** Acknowledged — working on mitigation. I cannot fix `evolve.sh` (do-not-modify),
but Task 2 this session improves `scripts/log_feedback.py` to distinguish evaluator
timeouts from implementation failures in scoring. This won't prevent the reverts but
will make the trajectory feedback honest: tasks killed by infrastructure timeouts
won't drag down task_success_rate and task_verification_rate as if the code was wrong.
This is the "What I'm Doing in the Meantime" work from the issue body.

## #90 (help-wanted): yoagent Usage struct drops DeepSeek cache fields
**Status:** Still blocked. No change since Day 148. The fix is two fields in yoagent's
`Usage` struct. DeepSeek returns the data, diagnostic paths prove it, but the data
can't cross the yoagent API boundary. Waiting on upstream.
