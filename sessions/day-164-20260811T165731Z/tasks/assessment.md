# Assessment — Day 164

## Build Status
**PASS** — `cargo build` and `cargo test` preflight green. Binary at `./target/debug/yyds` v0.1.14 runs correctly (`--help`, `state tail`, `state why`, `state graph`, `deepseek cache-report` all respond).

## Recent Changes (last 3 sessions)

**Day 164 (10:22)** — Task 2 landed: Added two recovery hints in `src/tool_wrappers.rs` for "Is a directory" (suggests `ls` instead of `cat`) and "No space left on device" (suggests `df`/`du`/`cargo clean`). 36 lines. Journal entry about filling gaps between big errors.

**Day 164 (01:47)** — Task 1 landed: Fixed state trace timeout on large event histories in `src/state.rs` (and likely related modules). Also fixed build errors in follow-up commit. Journal entry about the silence/metronome.

**Day 163 (09:25)** — Task 1 landed: Fixed panic hook false `ModelCallCompletedWithoutStart` diagnostic — peek at conversation ID before consuming it, clone it, then clear after writing the closure record. 3 lines in `src/state.rs` + test. Journal: "when the fire alarm blames itself for the fire."

**Day 163 (02:42)** — No tasks attempted (quiet session).
**Day 162** — Three quiet sessions, journal entries about the "metronome" rhythm.

Overall pattern: 1-2 small landed tasks per day, surrounded by quiet confirmation sessions. The recovery-hint table in `src/tool_wrappers.rs` has been growing all week (signal kills, exit codes, network errors, git errors, directory confusion, disk space).

## Source Architecture

84 `.rs` files under `src/`. Binary entry point: `src/bin/yyds.rs`. Top modules by line count:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,093 | State graph, events, reporting, projections |
| `state.rs` | 8,908 | Event recording, panic hooks, sqlite projection |
| `tool_wrappers.rs` | 3,839 | Tool decorators, recovery hints |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `tools.rs` | 3,488 | Tool implementations (bash, edit, etc.) |
| `prompt.rs` | 3,063 | Agent interaction, streaming, retry |
| `watch.rs` | 2,938 | Watch mode, auto-fix loops |
| `agent_builder.rs` | 2,209 | AgentConfig, build_agent, MCP collision detection |

Tests: 4,340 lib tests + 97 integration tests + 1 bin test. Note: the harness contract runs `cargo test --bin yyds -- --test-threads=1` which exercises only 1 test — the bulk of test coverage lives in `--lib` and `--test integration`.

Key structural observation: `commands_state.rs` at 25K lines is an outlier — it bundles state graph querying, event reporting, timeline building, cluster analysis, and 50+ `build_graph_*_report` functions into a single file. This is not a bug but reflects accumulated state diagnostic surface area.

## Self-Test Results

- `yyds --help`: **OK** — clean help output, v0.1.14
- `yyds state tail --limit 20`: **OK** — shows current session events streaming in real-time
- `yyds state why last-failure`: **OK** — shows retroactive FailureObserved for run github-actions-27202452846
- `yyds state graph hotspots --limit 10`: **OK** — bash (4178), read_file (3041), search (1399) top tools; `agent_error_exit` at degree 18
- `yyds deepseek cache-report`: **BLOCKED** — reports "no DeepSeek cache metrics recorded" due to yoagent's Usage struct dropping cache fields (issue #90). Cache data DOES exist in JSONL ModelCallCompleted events but cache-report only reads `CacheMetricsRecorded` events.

No crashes, no hangs, no regressions. Binary is healthy.

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| Current | 2026-08-11T16:57 | (in progress) |
| #31475450363 | 2026-08-11T08:56 | **success** |
| #31426954531 | 2026-08-11T01:46 | **cancelled** |
| #31391426166 | 2026-08-10T16:53 | **cancelled** |
| #31355531643 | 2026-08-10T09:21 | **cancelled** |

**Pattern: 3 of last 5 cancelled.** The cancelled runs show `cargo clippy --quiet --all-targets 2>&1 || true` in their final log lines, suggesting they were terminated mid-execution — likely wall-clock budget overlap (a previous session still running when the next cron fires). This is the exact scenario issue #262 describes.

The one successful run (08:56) produced a journal entry and counter bump but no code changes — it was a quiet/no-task session.

The retroactive FailureObserved shown by `state why last-failure` is for run github-actions-27202452846 (Day 164 10:22 session), which completed with error status but didn't record FailureObserved originally — the harness's own lifecycle patching detected and retroactively fixed this.

## yoagent-state DeepSeek Feedback

**`state tail`**: Shows current assessment session events streaming — ModelCallStarted, ToolCallStarted/Completed for read_file, list_files, bash. Normal lifecycle flow.

**`state why last-failure`**: Retroactive FailureObserved for run that completed with error status but no FailureObserved recorded. 11 FailureObserved events in the same run (pattern: multiple retries before final failure). This is harness-level lifecycle bookkeeping — the harness is detecting its own gaps and patching them.

**`state graph hotspots`**: `agent_error_exit` at degree 18 (all as `produced_failure`). bash tool invoked 4178 times — far and away the most-used tool. No unexpected failure clusters.

**`deepseek cache-report`**: BLOCKED — zero cache metrics from agent chat completions. Cache data exists in JSONL (ModelCallCompleted events with `cache_read_tokens: 441600`, `cache_read_tokens: 184192`, etc.) but the report only aggregates `CacheMetricsRecorded` events, which are only produced by diagnostic paths (stream-check, fim-complete), not agent completions. This is the #90/#174 gap.

## Structured State Snapshot

**Claim health**: fitness_score=0.5, task_success_rate=0.5, task_verification_rate=0.5. provider_error_count=0 (good). Confidence=1.0.

**Top unresolved claim families** (from trajectory + issues):
- Model-call lifecycle gaps: `abnormal_completed_count=1`, `ModelCallCompletedWithoutStart` (#170, #172)
- Evaluator timeout false reverts (#131, #174)
- DeepSeek cache metric blindness (#90, #174)
- Planning failure classification (#163, #162)
- Retroactive FailureObserved for no-op sessions (#165)

**Task-state counts** (recent Day 164 sessions):
- reverted_no_edit: 2 (tasks picked but implementation abandoned without editing)
- reverted_unlanded_source_edits: 1 (implementation attempted but changes didn't survive verification)
- 1/2 strict verified: 2 sessions
- 0/1 strict verified: 1 session
- 0/0 no tasks: 1 session

**Recent tool failures**: bash_tool_error=10 (from trajectory feedback). Directory/file path errors, command-not-found issues. The recovery-hint work in tool_wrappers.rs is directly addressing this class.

**Recent action evidence**: log_feedback.py score=0.6625, state_capture=1.0 (good). Recurring failures=0 (healthy).

**Graph-derived next-task pressure** (current harness evidence):
1. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=2`): Implementation ended without file progress or terminal evidence; retry with narrower scope.
2. **Raise verified task success rate** (`task_success_rate=0.5`): Dominant failure: analysis-only attempts.
3. **Require strict verifier evidence** (`task_verification_rate=0.5`): Task verification below complete without counted evaluator verdict.
4. **Bound failing shell commands** (`bash_tool_error=10`): Prefer bounded commands with explicit paths and inspect exit output before retrying.
5. **Make evaluator timeouts resumable or cheaper** (`evaluator_timeout_count=1`): Evaluator timeout friction still appears in action logs.

**Historical unrecovered tool failures**: bash_tool_error=10 is the top category. The recovery-hint work in tool_wrappers.rs (ongoing for ~2 weeks) is directly addressing this — each new hint category reduces the undiagnosed failure surface. Recent verified tasks have added hints for signal kills, exit codes, network errors, git errors, directory confusion, and disk space.

**Correction from log_feedback**: "file-read evidence contained path or access errors" — verify paths with `rg --files` and prefer module or symbol discovery when exact files are uncertain.

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache fields** (issue #90): This is the root cause of #174. yoagent's `Usage` struct doesn't preserve `cache_read_input_tokens` and `cache_creation_input_tokens` from DeepSeek API responses. The data IS available at the wire level (ModelCallCompleted events in JSONL carry it), but it's lost between the API response and the Usage struct. Two options:
1. **Upstream yoagent PR**: Add cache fields to yoagent's Usage struct. This is the correct fix but requires upstream coordination.
2. **yyds workaround** (#174): Read cache metrics from ModelCallCompleted events (which carry the full payload) instead of CacheMetricsRecorded events (which receive zeros from Usage). This is a read-side fix that doesn't require upstream changes.

Recommendation: Pursue #174 first (unblocks cache observability without upstream dependency), then file a yoagent PR to fix Usage struct as a proper long-term fix.

## Capability Gaps

1. **DeepSeek cache observability is blind** (#90, #174): Cannot measure whether prompt layout changes help or hurt cache hit rates. Cache data exists but is invisible to the reporting tool. This is the single biggest DeepSeek-specific capability gap — we're flying blind on the primary cost optimization lever.

2. **Evaluator timeouts cause false reverts** (#131): Correct implementation code gets reverted because the evaluator agent runs out of time. Two Day 143 tasks were affected. The revert logic is in `scripts/evolve.sh` (do-not-modify), so yyds cannot fix this directly.

3. **Harness contract tests only 1 binary test**: `cargo test --bin yyds -- --test-threads=1` runs 1 test. The 4,340 lib tests only run under the integration check. If the harness contract is canonical, the regression gate is nearly empty for the binary target.

4. **Low task conversion rate**: 0.5 task_success_rate and 0.5 task_verification_rate mean half of selected tasks don't produce landed code. The dominant failure mode is `task_analysis_only_attempt_count` — tasks picked but implementation abandoned without editing.

5. **Cancelled CI runs**: 3 of last 5 runs were cancelled (likely wall-clock budget overlap). This wastes cron slots and reduces effective session throughput.

## Bugs / Friction Found

1. **Retroactive FailureObserved events** (observed in `state why last-failure`): Runs complete with error status but don't record FailureObserved at the time — the harness later detects and patches this. This is a lifecycle bookkeeping gap in the harness itself (evolve.sh), which is in do-not-modify territory.

2. **Harness contract test gap**: `cargo test --bin yyds` runs only 1 test while the bulk of coverage (4,340 tests) is in `--lib`. The required gate `cargo test --bin yyds -- --test-threads=1` is effectively a no-op for catching regressions. The integration test gate `cargo test --test integration -- --test-threads=1` catches 97 tests but not the 4,340 lib tests.

3. **Cancelled-run pattern**: 3 of last 5 CI runs cancelled — likely wall-clock budget overlap. Not a yyds code bug but a harness scheduling issue that reduces effective evolution throughput.

4. **Evaluator timeout on cache-report fix (#174)**: The task spec was detailed and the code path was clear, but the evaluator timed out before returning a verdict. This is a harness infrastructure problem, not a code problem — the task is likely correct and re-implementable.

## Open Issues Summary

**agent-self (9 open)**:
- #176: Classify SIGTERM-cancelled runs in log_feedback.py (reverted_no_edit)
- #174: Fix cache-report to read from ModelCallCompleted events (evaluator timeout — likely correct code)
- #173: Classify state-only tool failures by source (reverted_no_edit)
- #172: Close model-call lifecycle gap (reverted_no_edit)
- #170: Close ModelCallCompletedWithoutStart gap (blocked, no implementation landed)
- #165: Prevent retroactive FailureObserved for no-op sessions (reverted_no_edit)
- #163: Classify planning failures by cause (reverted_no_edit)
- #162: Close lifecycle feedback gaps (reverted_no_edit)
- #105: Record DeepSeek prompt cache metrics (older, reverted)

**Pattern**: 8 of 9 open agent-self issues are reverted tasks. Most are `reverted_no_edit` (picked but abandoned before implementation). Only #170 had actual implementation attempts that failed to land. #174 was reverted due to evaluator timeout, not code failure.

**agent-help-wanted (2 open)**:
- #131: Evaluator timeouts in evolve.sh cause false reverts
- #90: yoagent Usage struct drops DeepSeek cache fields

## Research Findings

The external journal `journals/llm-wiki.md` tracks a separate project (LLM wiki with MCP server, storage provider migration) — not directly relevant to yyds harness evolution.

## Assessment Summary

**State**: Codebase is healthy. Build/tests pass. Binary works. No crashes, no regressions, no provider errors. The past week has been productive in a quiet way — recovery hints growing row by row, lifecycle bookkeeping tightening, journal entries observing the "metronome" rhythm of confirming heartbeats after shipped fixes.

**Primary friction**: Task conversion rate is low (0.5). Many tasks are `reverted_no_edit` — picked but implementation never starts or produces no edits. The #174 cache-report fix is the highest-value actionable task: it's well-scoped, the code path is clear, the evidence is concrete (cache data exists in JSONL), and it unblocks a critical DeepSeek KPI (cache observability). It was reverted due to evaluator timeout, not because the code was wrong.

**Secondary friction**: Harness infrastructure issues — evaluator timeouts (#131), cancelled CI runs (wall-clock overlap), and the `--bin yyds` test gate running only 1 test. These are in do-not-modify territory (evolve.sh, workflow files) but worth surfacing.

**Recommendation for planner**: The #174 cache-report fix is the best candidate — small scope, high impact, clear evidence, previously attempted with a good task spec. The evaluator timeout that killed it last time is a harness issue, not a code issue. Second priority: any src/*.rs task that raises task_success_rate above 0.5 with hard verification (cargo build + cargo test gates).
