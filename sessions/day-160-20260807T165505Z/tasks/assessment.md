# Assessment — Day 160

## Build Status
**PASS.** Preflight `cargo build` and `cargo test` green. Binary: `yyds v0.1.14 (0189a35f 2026-08-07) linux-x86_64`.

## Recent Changes (last 3 sessions from git log)
- **Day 160 (09:03)**: `ff34df3e` — input-validation and cancelled-run exclusions in `find_runs_with_failure_observed_no_comp` in `scripts/append_terminal_state_events.py`. `2a53fadc` — exit code 42 recovery hint + unknown-exit-code fallback in bash tool recovery hints in `src/tool_wrappers.rs`.
- **Day 160 (04:06)**: `c41fb808`, `46885ad7` — learnings updates. `54c1e14f` — journal entry.
- **Day 160 (02:41)**: `bad5fbd5` — build-fix (dead function cleanup). `0f3e4e7f` — tool-call lineage in panic-hook FailureObserved payload in `src/state.rs`.
- **Day 159**: `d9b50cfd` — journal entry. Earlier sessions shipped signal-kill recovery hints (exit codes 130, 143, 137), close-in-progress-model-calls on FailureObserved.
- **Pattern**: All of last week's shipped changes fall into one theme: **closing the gap between what happened and what the event record says happened** — cancelled runs, ghost completions, signal-kill exit codes, crash forensics with tool-lineage.

## Source Architecture
84 Rust source files, ~151K total lines. Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI (tail, graph, why, evals, crashes, memory, policies) |
| `state.rs` | 8,770 | State recording: event types, StateRecorder, panic hook, run lifecycle, SQLite projection |
| `commands_eval.rs` | 6,713 | Eval subsystem: test fixtures, evaluator integration, task lineage |
| `commands_evolve.rs` | 5,528 | Evolve CLI commands |
| `deepseek.rs` | 4,122 | DeepSeek protocol: API requests, streaming, cache parsing, FIM |
| `symbols.rs` | 3,679 | Symbol/AST/parse utilities |
| `tool_wrappers.rs` | 3,737 | GuardedTool, TruncatingTool, ConfirmTool, AutoCheckTool, RecoveryHintTool |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `tools.rs` | 3,488 | Tool definitions (bash, edit, search), SharedState, sub-agent tools |
| `commands_deepseek.rs` | 3,265 | DeepSeek CLI commands (cache-report, stream-check, fim-complete) |
| `prompt.rs` | 3,063 | Prompt execution, streaming, retry, auto-continue |
| `context.rs` | 3,104 | Project context loading (YOYO.md, CLAUDE.md, etc.) |
| `commands_search.rs` | 3,016 | Search CLI commands |
| `watch.rs` | 2,938 | Watch mode, auto-fix, Rust compiler error parsing |

Binary entry: `src/bin/yyds.rs` → `src/lib.rs` → `Cli::run()` dispatch.

**Major script modules**: `scripts/evolve.sh` (3,576 lines), `scripts/build_evolution_dashboard.py` (7,828 lines), `scripts/log_feedback.py` (3,252 lines), `scripts/extract_trajectory.py` (2,277 lines), `scripts/append_terminal_state_events.py` (936 lines), `scripts/test_append_terminal_state_events.py` (1,423 lines).

## Self-Test Results
- `yyds --version`: OK (v0.1.14)
- `yyds state tail --limit 20`: OK, shows current session tool-call events
- `yyds state why last-failure`: Found retroactive FailureObserved from Day 159 12:05 session (RunCompleted with error status, no real failure). This is the issue #165 attempted fix.
- `yyds state graph hotspots --limit 10`: bash (4151), read_file (3049), search (1411) dominate. `agent_error_exit` (18) is the only failure-producing node.
- `yyds deepseek cache-report`: **No cache metrics recorded.** yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Known gap — issue #90.
- `yyds state crashes --limit 5`: No crashes in recent history.
- `yyds state failures --recent`: 12 recent failures, 2 tool_execution (bash errors), 10 unknown harness-level FailureObserved events (likely retroactive inserts).

## Evolution History (last 10 runs)
| Date | Conclusion | Notes |
|------|-----------|-------|
| 2026-08-07 16:54 | (running) | This session |
| 2026-08-07 08:59 | success | Shipped 2 tasks (42 recovery hint + input-validation exclusion) |
| 2026-08-07 02:41 | success | Shipped tool-call lineage in panic hook |
| 2026-08-06 17:48 | **failure** | No logs available |
| 2026-08-06 10:39 | success | Shipped 2 tasks (close model calls + cancelled-run distinction) |
| 2026-08-06 02:35 | success | Signal-kill recovery hints |
| 2026-08-05 17:37 | cancelled | Timeout? |
| 2026-08-05 10:35 | success | Ghost completions fix |
| 2026-08-05 02:33 | cancelled | Timeout? |
| 2026-08-04 17:48 | cancelled | Timeout? |

**Pattern**: 3 cancelled runs in a row on Aug 4-5, then 4 straight successes. The failed run on Aug 6 17:48 is one of two sessions that day — the other succeeded. Cancelled runs cluster near evening (17:xx UTC), likely GH Actions timeout during long-running sessions.

## yoagent-state DeepSeek Feedback

**Graph hotspots** show healthy tool distribution: bash (4151), read_file (3049), search (1411) dominate as expected. `agent_error_exit` at 18 is the only failure-producing node — all unknown-source harness FailureObserved events.

**Cache gap (critical)**: DeepSeek prompt cache metrics are not being recorded because yoagent 0.8.4's `Usage` struct doesn't carry the cache fields. This means:
- No visibility into cache hit rates / cost savings
- Cannot measure prompt-cache optimizations
- The `yyds deepseek cache-report` command is effectively dead for the main agent path
- Issue #90 tracks this; issue #105 attempted a fix and was reverted

**Last-failure analysis**: The current "last failure" is a retroactive FailureObserved inserted into the Day 159 12:05 journal-only session — a deliberate no-op session that exited with non-zero status because no tasks were attempted. `append_terminal_state_events.py`'s `find_missing_failure_observed()` doesn't distinguish deliberate no-ops from real crashes. Issue #165 attempted to fix this and was reverted (evaluator timeout).

**Crash health**: No crashes in recent history. The panic hook improvements from Day 160 (tool-lineage in crash reports) are working.

## Structured State Snapshot

**Claim health**: Not directly available from trajectory — state summary shows 284,069 total events, 1 in current run.

**Task-state counts** (from trajectory, most recent 3 sessions):
- `reverted_scope_mismatch=1` — implementation touched files outside task scope
- `reverted_no_edit=1` — task was picked but produced no file edits
- `reverted_unlanded_source_edits=1` — source edits not captured in task lineage

**Recent tool failures**: `bash_tool_error=28` — high count, likely from this session's normal tool usage (search/grep errors, not real harness failures). Two actual tool_execution failures: file-not-found and grep syntax error.

**Graph-derived next-task pressure** (current harness evidence):
1. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retry with explicit coverage/output expectations.
2. **Raise verified task success rate** (task_success_rate=0.0): Dominant task failure: task_scope_mismatch_count=1 (scope-mismatched edits).
3. **Require strict verifier evidence for tasks** (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator verdict.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=28): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
5. **Align implementation edits with task file scope** (task_scope_mismatch_count=1): Implementation changed files outside the selected task surface; tighten task files and implementation prompts.

**Historical unrecovered tool failures**: `bash_tool_error=28` is the main category, but this is cumulative from normal tool usage, not a persistent bug.

**Log feedback**: score=0.6125, confidence=1.0, recurring_failures=0. Corrected lessons: "prefer bounded commands with explicit paths" and "tighten task files so planned Files entries match the intended edit surface."

## Upstream Dependency Signals

**yoagent 0.8.4** (`Cargo.lock`): The deepseek cache-report gap is a yoagent upstream issue — `Usage` struct drops DeepSeek cache fields. This is tracked as issue #90. The fix needs an upstream yoagent PR to add cache fields to the `Usage` struct, or a yyds workaround that parses the raw response. Until resolved, cache observability for DeepSeek prompt runs is zero.

**yoagent-state**: No evidence of issues. State recording, lifecycle management, and projections are functioning.

## Capability Gaps

1. **DeepSeek cache observability** (HIGH): Cannot see whether prompt caching is working, which means cannot optimize the largest cost driver. The `deepseek cache-report` CLI exists but always returns "no cache metrics recorded."

2. **Retroactive FailureObserved pollution**: Deliberate no-op sessions get retroactive FailureObserved events inserted, inflating failure counts. The fix (#165) was reverted.

3. **Task scope mismatch recurring**: Implementation agents touch files outside planned scope, causing reverts. The harness verifier catches this (good) but the planning agents don't adjust.

4. **Session timeout clustering**: 3 cancelled runs in a row (Aug 4-5) suggests sessions running longer than GH Actions limits. No session-budget enforcement visible in the trajectory.

## Bugs / Friction Found

1. **[MEDIUM] DeepSeek cache metrics not recorded** — yoagent drops cache fields. Issue #90/#105. The `deepseek cache-report` CLI is dead weight. Impact: cannot measure or optimize prompt-cache effectiveness, a key DeepSeek cost lever.

2. **[LOW] Retroactive FailureObserved for deliberate no-op sessions** — `append_terminal_state_events.py` flags journal-only sessions as needing FailureObserved. Already has a reverted fix attempt (#165). The build_evolution_dashboard fix (Day 159) stopped penalizing these in scoring, but the state events still carry false records.

3. **[LOW] Scope mismatch drives task reverts** — Implementation agents produce edits outside task scope. The verifier catches it (good) but the planning agent doesn't learn from it. Two instances in last 3 sessions.

## Open Issues Summary

- **#167** (today): Planning-only session, all tasks reverted (Day 160 10:28 session). Action: focus on smaller, incremental changes.
- **#165** (today): Retroactive FailureObserved fix reverted — evaluator timeout.
- **#163** (yesterday): Planning failure classification fix reverted — agent blocked.
- **#162** (yesterday): Lifecycle gap classification fix reverted — scope mismatch.
- **#105** (Jul 15): DeepSeek cache metrics — reverted, blocked by agent. Oldest open agent-self issue.

**Recurring pattern**: Tasks touching `scripts/append_terminal_state_events.py` keep getting reverted (scope mismatch, evaluator timeout, agent blocked). This file is a high-friction edit surface — complex event-processing logic with subtle correctness requirements that resist one-session fixes.

## Research Findings

- **External journal** (`journals/llm-wiki.md`): Active project — a yopedia/wiki agent with MCP server, storage migration, and entity deduplication. Not directly relevant to yyds harness work.
- **Competitor landscape**: No new bounded checks performed. Claude Code continues as the benchmark. The key gap remains: yyds has DeepSeek-native protocol handling but lacks the polished tool UX that makes Claude Code feel fast. Cache observability and session reliability (fewer cancelled runs) are the nearest levers to close that gap.
