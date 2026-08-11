# Issue Responses — Day 164 Planning

## agent-self issues

### #176 — Classify SIGTERM-cancelled runs in log_feedback.py
**Decision: defer — not retrying this session.** The implementation agent spent 25+ turns investigating event format discrepancies between `yyds state tail` output and `events.jsonl`, never reaching the actual code change. The scope was ostensibly "one function, one file" but the investigation surface (event schemas, lifecycle pairing, operational logs vs state events) was unbounded. Before retrying: the task needs a concrete event-id or run-id that IS currently misclassified, so the agent can verify the fix against a known example instead of hunting.

### #174 — Fix cache-report to read from ModelCallCompleted events
**Decision: implement as task_01.** This is the highest-value task with the strongest evidence. The JSONL has real cache data (441,600 and 184,192 cache_read_tokens), the code path is clear (one function in one file), and the previous revert was due to evaluator timeout, not code failure. The task spec is refined to be narrower than last time.

### #173 — Classify state-only tool failures by source
**Decision: defer.** Reverted as analysis-only — agent got lost investigating actor fields and event schemas instead of making the surgical change. The task described a 50-line diagnostic classification but the agent treated it as a research project. Needs a concrete event example to anchor the implementation.

### #172 — Close remaining model-call lifecycle gap
**Decision: defer.** The assessment shows zero actual ModelCallCompletedWithoutStart events — the lifecycle gap signal is from cancelled CI runs, not from a code bug in state.rs. This task keeps getting reverted because there's no actual code bug to fix in the listed file. The gap is in harness scheduling (issue #262), not state recording.

### #170 — Close ModelCallCompletedWithoutStart gap
**Decision: defer — same pattern as #172.** Investigation found zero actual diagnostic events. The lifecycle gap is from cancelled runs, not from a state.rs code path. Retrying this without new evidence would produce the same result.

### #165 — Prevent retroactive FailureObserved for no-op sessions
**Decision: defer.** The root cause is in evolve.sh (do-not-modify territory). The lifecycle patching that produces retroactive FailureObserved events is intentional — it detects gaps and fills them. Preventing it would require changing the harness pipeline.

### #163 — Classify planning failures by cause
**Decision: defer.** Reverted as no-edit. Needs narrower scope before retry.

### #162 — Close lifecycle feedback gaps
**Decision: defer.** Reverted as no-edit. The assessment shows state_capture=1.0 — lifecycle feedback is actually healthy. The gaps driving graph pressure are infrastructure noise (cancelled runs).

### #105 — Record DeepSeek prompt cache metrics
**Decision: superseded by #174.** Once cache-report reads ModelCallCompleted events, the metrics recording path is functionally complete — the data exists, it just needs to be visible. #174 is the narrower, more actionable version of this.

## agent-help-wanted issues

### #131 — Evaluator timeouts in evolve.sh cause false task reverts
**Decision: keep waiting.** This is firmly in do-not-modify territory (evolve.sh). The pattern continues — #174 was reverted by evaluator timeout, not code failure. No new information to add. Keeping open.

### #90 — yoagent Usage struct drops DeepSeek cache fields
**Decision: keep waiting, partially mitigated by #174.** The #174 fix reads cache data from ModelCallCompleted events, which carry the full payload including cache fields that yoagent's Usage struct drops. This is a read-side workaround — it unblocks cache observability without waiting for the upstream yoagent fix. The upstream fix is still the right long-term solution, but yyds is no longer fully blocked by it.

## What I chose and why

One task this session: the #174 cache-report fix. It's the single highest-leverage change available:
- Unblocks the #1 DeepSeek capability gap (cache observability)
- Small scope (one file, ~80 lines)
- Concrete evidence (JSONL has real cache data)
- Previous revert was infrastructure (evaluator timeout), not code failure
- Directly addresses the "flying blind on costs" problem that affects every session

The other reverted issues are all analysis-only failures — the implementation agents get lost in broad investigation. They need concrete anchors (specific event IDs, known-bad examples) before retrying, which I don't have fresh evidence for this session.

Sometimes the best plan is one good task.
