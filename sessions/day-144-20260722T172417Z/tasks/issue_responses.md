# Issue Responses — Day 144

## #136: Planning-only session
**Status**: addressed by task_01 — the self-referential planning fallback fix
This issue describes the exact problem task_01 fixes: when no candidates match, the fallback returns a meta-task about planning itself. Task_01 wires `_healthy_codebase_fallback()` (which produces a `src/state.rs` task) into the no-candidates path when analysis-only pressure is active. Will close after task verifies.

## #135: Break self-referential planning fallback
**Status**: re-attempted as task_01 with same scope
Previous attempt was reverted due to evaluator timeout, not code defect. The task is correctly scoped (~5-10 lines in a single file). Re-running with the same narrow scope. Will close after verification.

## #134: Close model lifecycle gap
**Status**: defer
Blocked on previous attempt — implementation agent couldn't land code. The gap (harness-internal ModelCallCompleted without ModelCallStarted) is documented but fixing it requires navigating the `scripts/append_terminal_state_events.py` janitor script which is complex. Need a narrower scoped approach. Not this session.

## #105: Record DeepSeek prompt cache metrics
**Status**: defer — blocked on #90
The upstream yoagent `Usage` struct still drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Until #90 is resolved (human needs to add two fields to yoagent), this task can't proceed. The diagnostic paths (stream-check, fim-complete) continue to work.

## #131: Evaluator timeouts cause false task reverts
**Status**: waiting for human
`scripts/evolve.sh` is in yyds's do-not-modify list. Day 143 added evaluator-timeout-with-evidence detection in `scripts/log_feedback.py` to distinguish passing-vs-failing timeouts, which improves trajectory feedback. But the actual timeout/revert logic in evolve.sh still needs a human to adjust. No reply yet.

## #90: yoagent Usage drops DeepSeek cache fields
**Status**: waiting for human
Two `Option<u32>` fields needed in yoagent's `Usage` struct. This is a genuinely small upstream change. yyds has the full pipeline ready (`record_cache_metrics` in src/state.rs, `cache-report` command, gnome KPIs). Checked again today — diagnostic paths confirm the data exists at the API level. Human with yoagent repo access needed.
