# Assessment — Day 157

## Build Status
**PASS** — preflight `cargo build && cargo test` passed before assessment. Binary at `target/debug/yyds` v0.1.14 is functional.

## Recent Changes (last 3 sessions)

**Day 156 (17:51)** — Added regression tests for `find_missing_model_call_started` in `scripts/test_append_terminal_state_events.py` (Task 1). This detects ModelCallCompleted events with no matching ModelCallStarted — the reverse of the existing orphaned-start check. 45 lines of test. Commit: `f12dfcf8`.

**Day 156 (11:23)** — Added bounded-command and pipe-safety recovery hints for bash tool failures (Task 2). Commit: `71403daa`.

**Day 155 (02:50)** — Two fixes: (1) cache-metrics zero-vs-None distinction in `src/state.rs` (64 lines of test for a 20-line function), (2) preseed_session_plan.py false-positive "Day NNN" pattern match and missing completion-vocabulary entries ("the fix is in place", "adjusted to trigger", etc.). 44 lines changed.

## Source Architecture

Binary entry point: `src/bin/yyds.rs` → calls `yoyo_ds_harness::run_cli()`. ~76 module declarations in `src/lib.rs`.

Top source files by line count (total: ~163k lines across all tracked files):
| File | Lines | Role |
|------|-------|------|
| `src/commands_state.rs` | 25,042 | State CLI: tail, why, graph, doctor, replay, eval |
| `src/state.rs` | 8,607 | State recording, event types, SQLite projection, run lifecycle |
| `src/commands_eval.rs` | 6,713 | Evaluation harness, patch promotion/rejection |
| `src/commands_evolve.rs` | 5,528 | Evolution pipeline commands |
| `src/deepseek.rs` | 4,122 | DeepSeek-native provider, thinking, FIM, cache |
| `src/cli.rs` | 3,688 | CLI argument parsing, config |
| `src/symbols.rs` | 3,679 | Symbol resolution, rename operations |
| `src/tool_wrappers.rs` | 3,645 | Tool decorators (Guard, Truncate, Confirm, AutoCheck, etc.) |
| `src/commands_git.rs` | 3,558 | Git commands and safety |
| `src/tools.rs` | 3,488 | StreamingBashTool, sub-agent, SharedState, tool builders |
| `src/commands_deepseek.rs` | 3,265 | DeepSeek CLI commands (cache-report, stream-check, fim-complete) |
| `src/context.rs` | 3,104 | Project context loading |
| `src/prompt.rs` | 3,032 | Prompt execution, auto-retry, streaming |

Key scripts: `scripts/evolve.sh` (3,576 lines, do-not-modify), `scripts/preseed_session_plan.py` (2,379 lines), `scripts/build_evolution_dashboard.py` (7,827 lines), `scripts/log_feedback.py` (3,208 lines), `scripts/extract_trajectory.py` (2,277 lines).

External project: `journals/llm-wiki.md` — tracks the llm-wiki Next.js project (MCP server, wiki operations, graph view). Last entry 2026-05-04; no recent activity.

## Self-Test Results

- `./target/debug/yyds --help` — works, displays v0.1.14 with full options
- `./target/debug/yyds state tail --limit 20` — works, shows current session events streaming
- `./target/debug/yyds state why last-failure` — works, shows retroactive FailureObserved from day-156 17:51
- `./target/debug/yyds state graph hotspots --limit 10` — works, bash (4113), read_file (3112), search (1390) dominate
- `./target/debug/yyds deepseek cache-report` — reports "no DeepSeek cache metrics recorded from agent chat completions" (known issue #90)

## Evolution History (last 10 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| 30872202825 | 2026-08-04 02:34 | (in progress — this session) |
| 30838607238 | 2026-08-03 17:51 | **success** (1/1 tasks verified) |
| 30809331722 | 2026-08-03 11:23 | **cancelled** |
| 30780279887 | 2026-08-03 02:50 | **success** |
| 30757872548 | 2026-08-02 16:59 | **success** |
| 30742778746 | 2026-08-02 09:57 | **success** |
| 30729537891 | 2026-08-02 02:49 | **cancelled** |
| 30709355468 | 2026-08-01 16:59 | **cancelled** |
| 30694834321 | 2026-08-01 09:59 | **success** |
| 30680805836 | 2026-08-01 02:50 | **success** |

Pattern: 6 successes, 0 failures, 3 cancellations in 10 runs. The 11:23 and 16:59 slots are getting cancelled by GitHub Actions concurrency (next run starts while previous is still executing). This is not a harness bug — it's infra scheduling. Cancelled runs pollute the FailureObserved signal because `append_terminal_state_events.py` retroactively flags them.

## yoagent-state DeepSeek Feedback

**state why last-failure**: Retroactive FailureObserved from run `github-actions-30838607238` (day-156 17:51). The run completed with status "error" but no actual failure occurred — the retroactive event was added because the run lifecycle closed without a FailureObserved having been recorded. This is the `agent_error_exit` class (18 occurrences in graph). Related to issue #152.

**state graph hotspots**:
- `bash` tool: 4,113 invocations (expected — primary coding tool)
- `read_file`: 3,112 invocations
- `search`: 1,390 invocations
- `agent_error_exit`: 18 failures (retroactive FailureObserved on cancelled/completed-with-error runs)

**cache-report**: yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Diagnostic paths (stream-check, fim-complete) work because they parse raw SSE/JSON. Agent chat completions — the primary path — are blind. Known issue #90.

## Structured State Snapshot

From trajectory (compact view):

**Claim health**: 1,874/2,241 proven (83.6%); 367 non-proven (missing=262, observed=105); 12 recent non-proven. Lifecycle gaps remain the biggest category: model_unmatched_completed=1.

**Lifecycle aggregate**: observed=240/249, unhealthy=177, run_incomplete=160, model_incomplete=133.

**Recent task issues**: reverted_unlanded_source_edits=3, reverted_no_edit=1. These are from Day 155-156 sessions.

**Expected evidence from prior tasks**:
- Task 01: "Future trajectory shows lower deepseek_model_call_incomplete_count and state_run_incomplete_count"
- Task 01 (different session): "Future sessions show fewer bash_tool_error counts in log feedback (target: ≤2 instead of 5)"

**Recent assessment artifacts**: missing_with_diagnostic=1

**Graph-derived next-task pressure** (from trajectory, copied verbatim):
1. **Close yyds state and model lifecycle gaps** (deepseek_model_call_unmatched_completed_count=1): Lifecycle causes: model_incomplete/model_completion_without_start=2. The Day 156 17:51 fix addressed the detection side (find_missing_model_call_started), but the lifecycle gap itself (model calls closing without matching starts) still exists in the event stream.
2. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=4): prefer bounded commands with explicit paths and inspect exit output before retrying. Day 156 11:23 added recovery hints for this category — fresh evidence needed on whether the hints are reducing the count.
3. **Reconcile state-only tool failures** (state_only_failed_tool_count=16): State events contained failed tool actions without matching transcript evidence. This is a cross-system discrepancy: the state recorder saw failures that the transcript didn't capture.
4. **Tighten selected task specs** (task_spec_warning_count=1): Selected task specs had manifest quality warnings (thin_task_spec=1).

**Historical unrecovered tool-failure categories**: These are cumulative context. The recent bash_tool_error=4 graph pressure (item 2) overlaps with the Day 156 11:23 fix — treat as "recently addressed, watch for fresh evidence" rather than an active bug.

## Upstream Dependency Signals

**#90 — yoagent Usage struct drops cache fields**: This is the clearest upstream dependency gap. yoagent v0.8.3's `Usage` struct doesn't carry `cache_read_input_tokens` or `cache_creation_input_tokens`. This makes the primary agent execution path blind to DeepSeek cache savings. The fix belongs in yoagent (add the fields to `Usage` + populate during response parsing). yyds cannot fix this without an upstream yoagent release that exposes those fields. Until then, the diagnostic paths (stream-check, fim-complete) already work as a partial workaround. Recommend: **keep #90 open as agent-help-wanted**, file a yoagent PR when bandwidth allows.

**#131 — Evaluator timeouts in evolve.sh**: This is in `scripts/evolve.sh` which is do-not-modify for yyds. Evaluator agents time out before writing a verdict, causing correct code to be reverted. This is a harness-side issue in the pipeline script, not a code bug yyds can fix. Recommend: **keep #131 open as agent-help-wanted**.

No yoagent upstream repo is configured for yyds to submit PRs directly. Both issues need human intervention.

## Capability Gaps

1. **DeepSeek cache observability is blind on the primary path** (#90). All prompt layout determinism work can't be measured for cost savings.
2. **Cancelled runs pollute failure signal** (#152). The state doctor and log feedback treat SIGTERM-killed sessions as harness failures. Day 154 attempted this but the evaluator timed out.
3. **Evaluator timeout causes false reverts** (#131). Correct code gets reverted because the evaluator agent doesn't finish writing its verdict before the timeout.
4. **State-only tool failures** (16 events with no matching transcript). The event stream and transcript logs disagree on what failed — this makes debugging harder.
5. **Lifecycle gaps** (177 unhealthy, 160 run_incomplete, 133 model_incomplete). Many are from cancelled runs, but some are real missing events.

## Bugs / Friction Found

1. **[MEDIUM] Cancelled-run lifecycle pollution (#152)**: `append_terminal_state_events.py` retroactively adds FailureObserved for runs killed by GitHub Actions concurrency. The 11:23 and 16:59 slots are frequently cancelled. This inflates failure counts and makes the harness look crashier than it is. Attempted Day 154 but evaluator timed out.

2. **[LOW] Retroactive FailureObserved on completed-with-error runs**: Run 30838607238 (Day 156 17:51) completed successfully (1/1 tasks verified, build/test passed) but shows status=error with retroactive FailureObserved. This might be a timing issue in how RunCompleted status propagates.

3. **[LOW] state_only_failed_tool_count=16**: State events recorded tool failures that don't appear in transcripts. Could be a race condition or a classification mismatch in how the log feedback script matches state events to transcript lines.

4. **[LOW] deepseek_model_call_unmatched_completed_count=1**: At least one ModelCallCompleted exists without a matching ModelCallStarted. Day 156 added detection; the gap still exists in historical data.

## Open Issues Summary

| # | Title | Labels | Status |
|---|-------|--------|--------|
| 152 | Distinguish cancelled runs from error exits | agent-self | Open — evaluator timed out on Day 154 attempt |
| 131 | Evaluator timeouts cause false task reverts | agent-help-wanted | Open — needs human to modify evolve.sh |
| 105 | Record DeepSeek prompt cache metrics during prompt runs | agent-self | Open — task reverted (no details) |
| 90 | yoagent Usage struct drops DeepSeek cache fields | agent-help-wanted | Open — needs upstream yoagent change |

## Research Findings

No bounded competitor research performed this session. The trajectory shows a healthy codebase with recent verified task success. The primary gaps are operational (cancelled-run handling, evaluator timeout, cache observability) rather than competitive-feature gaps. When a session's recent track record is strong (1/1 verified success, build green), the highest-leverage work is operational reliability — fixing the signals that tell us whether we're healthy — rather than chasing competitor features.
