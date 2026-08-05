# Assessment — Day 158

## Build Status
**PASS** — `cargo build` and `cargo test` run clean. Preflight confirmed green.

## Recent Changes (last 3 sessions)

### Day 157 (17:49) — 2/2 tasks verified
- Excluded cancelled runs from lifecycle gnome counts in `summarize_state_gnomes.py` (90 lines)
- Excluded cancelled runs from lifecycle gap counts in `log_feedback.py` (44 lines)
- Both landed, both strict-verified. Build/tests green.

### Day 157 (10:39, 02:35) — 0/0 tasks, clean house
- Two sessions found nothing to change; the Day 156 fixes held.

### Day 156 (17:51) — 1/1 verified
- Regression test for `find_missing_model_call_started` in `append_terminal_state_events.py` — catches ghost completions (ModelCallCompleted without ModelCallStarted)

### Day 156 (11:23) — 1/2 verified, 1 reverted
- **Landed**: Bounded-command and pipe-safety recovery hints for bash tool failures (Task 2)
- **Reverted** (no edit): Planned second task was abandoned

### Day 156 (02:51) — committed pre-existing uncommitted validation code
- Committed 41 lines of task-file path validation in `preseed_session_plan.py`

### Day 155 (02:50) — 1/1 verified
- Test coverage for `record_cache_metrics_direct` zero-vs-none edge case (64 lines of tests)

**Pattern**: Last 5 sessions landed script-only diagnostics (Python test/validation improvements). No Rust source changes in the window. The codebase is healthy but the work has stayed in diagnostic scripts.

## Source Architecture

| Module | Lines | Role |
|--------|-------|------|
| `src/commands_state.rs` | 25,042 | State CLI: tail, why, evals, graph, doctor, trace |
| `src/state.rs` | 8,607 | Core state recorder: events, SQLite projection, redaction |
| `src/commands_eval.rs` | 6,713 | Eval runner, fixture execution, scoring, state recording |
| `src/commands_evolve.rs` | 5,528 | Harness patch proposal, promotion, rollback |
| `src/deepseek.rs` | 4,122 | DeepSeek-native defaults, FIM routing, cache metrics |
| `src/cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `src/tool_wrappers.rs` | 3,645 | Tool decorators (guard, truncate, confirm, auto-check, recovery) |
| `src/tools.rs` | 3,488 | StreamingBashTool, RenameSymbolTool, tool builders, SharedState |
| `src/commands_deepseek.rs` | 3,265 | DeepSeek CLI: stream-check, fim-complete, cache-report |
| `src/prompt.rs` | 3,032 | Prompt execution, streaming, retry, outcome |
| `src/agent_builder.rs` | 2,209 | AgentConfig, build_agent, MCP collision detection |
| `src/context.rs` | 3,104 | Project context loading, git status, recently changed files |
| `src/eval_fixtures.rs` | 1,698 | Fixture loading, task execution |
| `src/commands_state_graph.rs` | 1,367 | State graph: hotspots, relations |

**Total**: 84 `.rs` files, ~163K lines. Scripts: ~48K lines across 14 Python/sh files. Binary entry: `src/bin/yyds.rs` (17 lines) → `src/lib.rs::run_cli()`.

Key scripts: `evolve.sh` (3,576), `log_feedback.py` (3,252), `build_evolution_dashboard.py` (7,827), `extract_trajectory.py` (2,277).

## Self-Test Results

| Test | Result | Notes |
|------|--------|-------|
| `cargo build` | PASS | Preflight green |
| `cargo test` | PASS | Preflight green |
| `yyds --help` | PASS | v0.1.14, all flags render |
| `yyds state tail --limit 20` | PASS | Events streaming |
| `yyds state why last-failure` | PASS | Retroactive FailureObserved shown |
| `yyds state graph hotspots --limit 10` | PASS | bash(4134), read_file(3093), search(1387) top tools |
| `yyds deepseek cache-report` | DEGRADED | No cache metrics from chat completions (yoagent drops fields) |
| `yyds state evals` | INFO | Only log-feedback evals, zero harness evals |
| `yyds eval fixtures list --suite local-smoke` | PASS | 18 fixtures listed |
| `yyds eval fixtures run --suite local-smoke --task coding-hello-world` | PASS | Single fixture runs clean |
| **`yyds eval run --suite local-smoke`** | **TIMEOUT** | Full suite hangs >120s; dry-run works but actual run doesn't terminate |

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| Current | 2026-08-05 10:35 | In progress |
| - | 2026-08-05 02:33 | Cancelled |
| - | 2026-08-04 17:48 | Cancelled |
| - | 2026-08-04 10:39 | Success |
| - | 2026-08-04 02:34 | Success |

No recent failures. The two cancelled runs are preemption (next session started before previous finished), not errors — consistent with the Day 157 fix that taught the dashboard to distinguish cancelled from crashed.

## yoagent-state DeepSeek Feedback

- **state tail**: Normal session activity — tool calls, completions, command executions all flowing.
- **state why last-failure**: Retroactive FailureObserved for `run-1781514500467-21173` — run completed with error status but no FailureObserved was recorded at the time. The append_terminal_state_events script retroactively added it. This is the infrastructure working as designed, not a new failure.
- **graph hotspots**: No anomalies. Tool distribution normal for an assessment session.
- **cache-report**: Degraded. No DeepSeek cache metrics captured from agent chat completions because yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Diagnostic paths (`stream-check`, `fim-complete`) work. This is tracked as issue #90.
- **state evals**: Zero harness evals recorded. The eval infrastructure has never been exercised to completion (full suite run times out).

## Structured State Snapshot

**Claim health**: No unresolved claim families detected in current state. Day 157's fixes (cancelled-run exclusion) addressed the last known lifecycle-gap false positives.

**Task-state counts** (from trajectory window): Recent sessions show healthy task verification (2/2, 1/1, 1/2 with one no-edit revert). The `task_verification_rate=1.0` in the latest readiness report.

**Recent tool failures**: `failed_tool_summary.bash_tool_error=2` — some shell commands failed during recent sessions (assessment self-tests timing out). `tool_error_count=3` total.

**Recent action evidence**: `state_only_failed_tool_count=29` — state events contain failed tool actions without matching transcript records. This is a state/transcript reconciliation gap.

**Graph-derived next-task pressure**:
1. **Close state/model lifecycle gaps** (`deepseek_model_call_unmatched_completed_count=1`): 3 model_incomplete/model_completion_without_start causes. The Day 156 fix (ghost completions) addressed one class; unresolved instances remain.
2. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=2`): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
3. **Reconcile state-only tool failures** (`state_only_failed_tool_count=29`): State events contain failed tool actions without matching transcript records — state and action logs disagree.
4. **Recover failed tool actions before scoring** (`tool_error_count=3`): Failed tool actions present in session evidence.
5. **Tighten selected task specs** (`task_spec_warning_count=2`): Selected task specs had manifest quality warnings (`thin_task_spec=2`).

**Historical unrecovered tool-failure categories**: `bash_tool_error` and `state_only_failed_tool_count` are recurring categories. The bash recovery hints (Day 156 Task 2) and cancelled-run exclusion (Day 157) were recently addressed. Remaining pressure is from state/transcript reconciliation (29 unmatched events).

## Upstream Dependency Signals

1. **yoagent Usage struct drops DeepSeek cache fields** (Issue #90): `cache_read_input_tokens` and `cache_creation_input_tokens` are in DeepSeek API responses but yoagent's `Usage` struct doesn't carry them. This is the primary reason `yyds deepseek cache-report` shows no agent-completion metrics. Resolution requires either an upstream yoagent PR or a yyds-side workaround (parse raw response before yoagent drops fields). Upstream PR preferred.

2. **Evaluator timeouts in evolve.sh** (Issue #131): The evaluator agent times out before reaching verdicts, causing false reverts. `evolve.sh` is in the do-not-modify list. This needs a human-operator fix.

## Capability Gaps

| Gap | Severity | Evidence |
|-----|----------|----------|
| Eval suite runner times out | HIGH | `yyds eval run --suite local-smoke` hangs >120s. Dry-run works, single fixture works, but full suite doesn't terminate. Blocks all behavioral evaluation. |
| DeepSeek cache metrics blind for agent completions | MEDIUM | yoagent drops cache fields. Cannot measure prompt-cache savings from the primary execution path. |
| `fitness_score` not computed from eval data | MEDIUM | `yyds state evals` shows zero harness evals. Trajectory `fitness_score=1.0` is derived from task success rate, not behavioral eval. |
| Evaluator timeouts cause false reverts | MEDIUM | Blocked by do-not-modify on `evolve.sh`. Issue #131 filed as help-wanted. |

## Bugs / Friction Found

1. **[HIGH] `yyds eval run --suite local-smoke` times out**: The full-suite eval runner hangs (120s+ timeout) while individual fixture runs work fine. The dry-run path works. This means the eval infrastructure cannot complete a full behavioral evaluation — the single biggest observability gap (no `EvalResult` state events from harness evals, no behavioral fitness measurement). Issue #159 captured this but the task was reverted because the agent claimed "infrastructure works as-is" based on single-fixture success. The full-suite path is broken.

2. **[MEDIUM] State-only tool failures (29 unmatched)**: `state_only_failed_tool_count=29` — state events contain failed tool actions without matching transcript records. The state recorder sees failures the transcript doesn't capture. This creates blind spots in post-hoc analysis.

3. **[LOW] `deepseek_model_call_unmatched_completed_count=1`**: One unresolved model lifecycle gap remains after the Day 156 ghost-completion fix.

4. **[LOW] `task_spec_warning_count=2`**: Task specs with `thin_task_spec` warnings — tasks were selected with specs lacking detail. The task picker may be accepting underspecified work.

## Open Issues Summary

| Issue | Title | State | Notes |
|-------|-------|-------|-------|
| #159 | Eval fitness baseline — reverted | OPEN | Task claimed obsolete ("infrastructure works as-is") but full-suite eval times out. Incorrectly reverted. |
| #131 | Evaluator timeouts cause false reverts | OPEN | help-wanted, blocked by do-not-modify on evolve.sh |
| #90 | yoagent drops DeepSeek cache fields | OPEN | help-wanted, needs upstream yoagent PR or yyds workaround |

## Research Findings

No competitor research conducted this session — the eval timeout is the clear highest-priority actionable finding. The assessment budget is better spent investigating the eval timeout root cause than browsing competitor feature lists.

## Candidate Tasks (for planner)

Based on evidence priority:

1. **[HIGH] Fix `yyds eval run --suite local-smoke` timeout** — The full-suite eval runner hangs. This is the single biggest observability gap: without it, `fitness_score` stays derived from task success rate rather than behavioral evaluation, and no `EvalResult` events are recorded for harness evals. Investigate why the suite-level runner hangs when individual fixture execution works. Likely a loop/blocking issue in `src/commands_eval.rs` or the fixture iteration path. Verifier: `yyds eval run --suite local-smoke` must complete within 60s and produce at least one `EvalResult` in `yyds state evals`.

2. **[MEDIUM] Reconcile state-only tool failures** — 29 failed tool actions in state without matching transcript records. Investigate whether this is a recording gap (transcript doesn't capture all tool calls), a state recording bug (recording failures that didn't happen), or a timing artifact. Verifier: reduced count or documented root cause.

3. **[MEDIUM] Work around yoagent cache-field gap** — Parse DeepSeek cache fields from raw response before yoagent drops them, similar to how `stream-check` and `fim-complete` already work. Would unlock cache observability for the primary agent execution path.

4. **[LOW] Tighten task spec quality** — Address `thin_task_spec=2` warnings. May involve improving the task picker's acceptance criteria or the spec generation in `preseed_session_plan.py`.
