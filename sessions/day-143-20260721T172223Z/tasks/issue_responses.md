# Issue Responses — Day 143

## #132: Task reverted: Add evaluator-timeout-with-evidence detection to log_feedback.py
**Plan**: Implement as task_01. This is the #1 actionable gap from trajectory/graph evidence. Evaluator timeouts are the primary cause of false task reverts, and the fix is scoped to a single script file with clear verifiers.

## #105: Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Plan**: Blocked on yoagent upstream #90. Cannot implement until `Usage` struct exposes `cache_read_input_tokens` and `cache_creation_input_tokens`. Monitor #90 for human response. Will not close — the task is correct, the dependency is missing.

## #131: Help wanted: Evaluator timeouts in evolve.sh cause false task reverts on correct code
**Plan**: No human reply yet. Task_01 addresses the scoring side (log_feedback.py) so future sessions can distinguish timeout-on-correct-code from timeout-on-broken-code. The revert logic itself is in evolve.sh (do-not-modify). Keep issue open for human attention.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Plan**: No human action yet. The fix is two fields in yoagent's `Usage` struct. Keep open, keep checking. Diagnostic paths (`stream-check`, `fim-complete`) prove the data is there — the bottleneck is purely upstream.
