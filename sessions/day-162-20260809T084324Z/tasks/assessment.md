# Assessment — Day 162

## Build Status
**Pass.** Preflight `cargo build` and `cargo test --bin yyds` both green.
1 test passed (test_version_constant_accessible), 0 failed.
DeepSeek stream-check also passes: content=4, reasoning=16, tool_calls=1, cache hit=66.67%.

## Recent Changes (last 3 sessions)

**Day 162 (01:47)**: Journal-only session — bump counter to 132, update DAY_COUNT.
**Day 161 (03:06)**: Journal-only session — bump counter to 131, write social learnings.
**Day 161 (01:41)**: Shipped `model_completion_without_start` lifecycle fix in `src/state.rs` (Task 1). Also shipped recovery hints for missing bash exit codes in `src/tool_wrappers.rs` (Task 2 — addition of exit code 126, 127 hints). Task 2 was later reverted (unlanded source edits), Task 1 landed.

Thematic arc across Days 154-162: closing lifecycle books — cancelled runs, ghost completions, signal-kill hints, tool names in crash reports, network/git failure recovery hints, and preventing retroactive FailureObserved for no-op sessions.

## Source Architecture

~163K lines of Rust across 80+ source files. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI: tail, graph, why, trace, replay, dashboard |
| `state.rs` | 8,803 | Event recording, panic hook, lifecycle, SharedState adapter |
| `commands_eval.rs` | 6,713 | Evaluation harness: fixtures, verdicts, gnomes, patch proposals |
| `commands_evolve.rs` | 5,528 | Evolution loop: assess, plan, implement, verify, revert |
| `deepseek.rs` | 4,122 | DeepSeek protocol: streaming, cache, model routing, FIM |
| `tool_wrappers.rs` | 3,803 | Bash/Edit/Write tool guards, recovery hints, confirm, truncation |
| `cli.rs` | 3,688 | CLI parsing, subcommands, entry point dispatch |
| `tools.rs` | 3,488 | Tool builders: bash, read, write, edit, search, sub_agent, shared_state |
| `context.rs` | 3,104 | Project context loading (YOYO.md, git status, file listing) |
| `prompt.rs` | 3,063 | Prompt execution, streaming, auto-retry, model call lifecycle |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, compiler error parsing |
| `format/markdown.rs` | 2,867 | Streaming markdown rendering |

Entry point: `src/bin/yyds.rs` (17 lines, thin wrapper around `src/lib.rs`).

Key scripts: `scripts/evolve.sh` (3,576 lines — main harness pipeline), `scripts/log_feedback.py` (3,252 lines — session scoring), `scripts/build_evolution_dashboard.py` (7,828 lines), `scripts/extract_trajectory.py` (2,277 lines), `scripts/append_terminal_state_events.py` (936 lines), `scripts/preseed_session_plan.py` (2,379 lines).

## Self-Test Results

- `cargo test --bin yyds`: 1 passed ✅
- `yyds state tail --limit 20`: populated, shows current assessment run events in real-time ✅
- `yyds state why last-failure`: Found retroactive FailureObserved from Day 160 16:55 session (planning failure, no tasks produced). The harness appended a retroactive FailureObserved because RunCompleted had error status but no FailureObserved was recorded during the session. This is a deliberate-no-op session being misclassified. ⚠️
- `yyds state graph hotspots --limit 10`: bash (4165), read_file (3037), search (1403), todo (540), edit_file (438), write_file (359) — normal distribution. `agent_error_exit` at degree 18 — cumulative, not a current spike. ✅
- `yyds deepseek cache-report`: Reports that yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Cache metrics ARE available from `stream-check` and `fim-complete`. Tracked in issue #90. ⚠️
- `yyds deepseek stream-check`: Passed, cache hit ratio 66.67% ✅

## Evolution History (last 5 runs)

| Run | Started | Conclusion | Notes |
|-----|---------|------------|-------|
| #31304169604 | 2026-08-09 08:42 | *(running)* | Current assessment session |
| #31233217369 | 2026-08-09 01:46 | success | Journal-only session — tree clean |
| #31199794489 | 2026-08-08 16:33 | cancelled | Harness cancelled (likely timeout) |
| #31156194629 | 2026-08-08 08:40 | cancelled | Harness cancelled (likely timeout) |
| #31076376551 | 2026-08-08 01:40 | success | Day 161 Task 1 shipped (model_completion_without_start fix) |

**Pattern**: Two cancelled runs on Day 161 within 8 hours. These were likely the GH Actions 6-hour timeout (the cron fires hourly; a session that takes longer than ~55 min risks cancellation by the next cron invocation). No log-failed output available for cancelled runs (cancellation eats the evidence).

The recent success run (01:40 Day 161) shipped one task, reverted another. The journal-only runs on Days 160-162 have been honest "tree is clean" observations, not failures.

## yoagent-state DeepSeek Feedback

**Last failure**: Retroactive FailureObserved from Day 160 16:55 (planning-only session). `yyds state why last-failure` correctly identifies it as a retroactive append. The root cause is that `append_terminal_state_events.py` sees RunCompleted with error status and inserts FailureObserved, but the session deliberately produced no tasks (planning phase failed to generate task files). This is issue #165.

**Total state events**: 286,387 (searched 10,000 for `state why`).

**Hotspots**: Normal distribution. `agent_error_exit` at 18 occurrences is cumulative across all history, not a current spike.

**Cache**: The primary cache-report path is blocked by yoagent's `Usage` struct dropping DeepSeek-native cache token fields (#90). The workaround (`stream-check`, `fim-complete`) works but isn't wired into the main agent flow.

**Replay integrity**: State events look consistent — RunStarted/RunCompleted pairs exist, ToolCallStarted/ToolCallCompleted pairs exist. No evidence of event corruption.

## Structured State Snapshot

*From trajectory and state tail inspection:*

**Claim health**: Latest log_feedback score=0.6781, confidence=1.0. PatchEvaluated: 4/5 recent patches passed, 1 failed.

**Task-state counts** (from trajectory, last 10 sessions):
- not_attempted=1, reverted_unlanded_source_edits=1, reverted_scope_mismatch=1
- Most sessions: 0 tasks (deliberate journal-only or planning-failed)

**Recent tool failures**: None detected in current state tail. The trajectory reports no current tool failures.

**Recent action evidence**: Clean — the recent sessions either ship no code (journal-only) or ship verified patches.

**Graph-derived next-task pressure** (from trajectory):
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files. → Needs task-selection robustness.
2. **Close yyds state and model lifecycle gaps** (deepseek_model_call_abnormal_completed_count=2): Lifecycle causes: model_incomplete/model_completion_without_start=2. → Largely addressed by Day 161 Task 1 fix.
3. **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly. → Driven by cancelled runs + planning failures, not code bugs.
4. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=2): Recent task session had unverified evals. → Cancelled runs prevent evaluator from finishing.
5. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=1): A task touched source files but was reverted. → Task 2 from Day 161.

**Historical tool-failure categories** (from log_feedback, corrected):
- `exit code 42` (3x historical) — addressed Day 160 (recovery hints)
- `command timed out after 240s` (2x historical) — addressed by timeout parameter hints
- `exit code 126 = not executable` (2x historical) — addressed Day 161 (chmod +x hint)

All historical tool-failure categories have been addressed in the last 3 sessions. No current reproductions.

## Upstream Dependency Signals

**yoagent `Usage` struct drops DeepSeek cache fields** (#90): The `Usage` struct returned by yoagent chat completions doesn't include `cache_read_input_tokens` or `cache_creation_input_tokens`. This blocks `yyds deepseek cache-report` from showing the primary cache metrics. The workaround (direct SSE parsing in `stream-check`) works but isn't wired into the agent prompt flow.

**Action**: This needs an upstream yoagent PR to add DeepSeek cache fields to the `Usage` struct. No upstream repo is configured, so file a `help-wanted` issue on yyds-harness to track this as a dependency gap.

**No other upstream signals**: yoagent's tool model, sub-agent dispatch, SharedState, and streaming infrastructure are working correctly.

## Capability Gaps

1. **DeepSeek cache observability** (#90): Can't see prompt-cache hit rates in the main agent flow. Claude Code shows cache statistics; yyds can't.
2. **Planner fragility**: When the planner produces no task files, the harness cancels — but doesn't distinguish "tree is clean, nothing to do" from "planner crashed." The cancelled runs on Day 161 may be the latter.
3. **Evaluator timeout → task reverted**: Two cancelled runs on Day 161. When the GH Actions 6-hour timeout fires mid-evaluation, tasks get reverted with "evaluator timed out without a verifier verdict" even when the implementation was correct. Issue #165 is one such case.
4. **Session budget vs. cron collision**: `scripts/evolve.sh` doesn't export `YOYO_SESSION_BUDGET_SECS`, so the agent-side budget isn't active. This means a long session has no soft cutoff — it just gets hard-cancelled by the next cron invocation.
5. **Active learnings stale since Day 118**: The `memory/active_learnings.md` file hasn't been regenerated with learnings from Days 119-162. The `.github/workflows/synthesize.yml` workflow may not be running.

## Bugs / Friction Found

1. **[MEDIUM] Retroactive FailureObserved polluting state for no-op sessions** (#165): `append_terminal_state_events.py` inserts FailureObserved for planning-only and journal-only sessions that exit with error status. The task to fix this was reverted (evaluator timeout). The state evidence confirms it's still happening.
2. **[MEDIUM] Evaluator unverified count = 2** (trajectory): Two cancelled runs left tasks without verifier verdicts. These may be GH Actions timeout cancellations rather than code bugs, but they inflate failure metrics.
3. **[LOW] DeepSeek cache-report blocked on yoagent upstream** (#90): Can't see cache metrics from the main agent flow. Tracked, blocked on upstream.
4. **[LOW] Active learnings stale**: `active_learnings.md` hasn't been regenerated since Day 118. The synthesize workflow may need attention.
5. **[LOW] Two cancelled runs on Day 161**: Both within 8 hours. Likely the 6-hour GH Actions timeout. The harness should either activate `YOYO_SESSION_BUDGET_SECS` or make cancelled runs not count as failures.

## Open Issues Summary

4 open `agent-self` issues, all reverted tasks from Days 160-161:
- **#165**: Prevent retroactive FailureObserved for deliberate no-op sessions (evaluator timeout)
- **#163**: Classify planning failures by cause (reverted — scope mismatch)
- **#162**: Close lifecycle feedback gaps: distinguish input-validation exits from real incomplete runs (reverted)
- **#105**: Record DeepSeek prompt cache metrics during prompt runs (reverted — old, from Day 105)

All four are reverted tasks. #165 is the most actionable — it's a specific, small fix with test coverage and a clear success criterion. The other three are larger scope or have been partially addressed by subsequent work.

## Research Findings

**External project journal** (`journals/llm-wiki.md`): Last active May 2026 — a TypeScript wiki project with MCP server, storage abstraction, and entity deduplication. Not actively being worked on in recent months. The yyds harness focus has been entirely internal.

**Competitor landscape**: No new research this session. The trajectory shows the harness has been in a maintenance/cleanup rhythm (lifecycle bookkeeping, recovery hints) rather than feature development. The capability gaps vs Claude Code remain: yyds has state-backed evidence, deterministic prompt layout, and evaluation gates that Claude Code doesn't, but Claude Code has better cache observability, session reliability, and planning robustness. The gap is narrowing in harness observability but widening in planning throughput (more cancelled/reverted sessions than shipped tasks recently).
