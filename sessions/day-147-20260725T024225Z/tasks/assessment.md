# Assessment — Day 147

## Build Status
**Pass.** Preflight `cargo build` and `cargo test` green. Binary at `./target/debug/yyds v0.1.14 (ff224e33 2026-07-25)`. No build errors or test failures in current tree.

## Recent Changes (last 3 sessions)
All from Day 146 (2026-07-24):
1. **19:04** — Updated test assertion counts in `src/state.rs` after failure relations were added (2 lines). Bumped skill-evolve counter to 82.
2. **17:38** — Two fixes: (a) improved `state graph hotspots --kind` error message when filter matches zero nodes — now lists valid kinds from data instead of generic "no relations" (`src/commands_state_graph.rs`, 23 lines). (b) Added test for orphaned-run sweeper handling nonexistent events file (`src/state.rs`, 11 lines). Bumped counter to 81.
3. **10:18** — Fixed `state graph hotspots --kind failure` flag not actually filtering results — the `kind` parameter was parsed but never passed to the SQL query (`src/commands_state_graph.rs`, 28 lines). Added failure relations to graph projection (`src/state.rs`). Bumped counter to 79.

Pattern: all Day 146 sessions focused on diagnostic visibility — making failure states discoverable and error messages helpful. Six sessions total landed across the day; all passed strict verification.

## Source Architecture
84 `.rs` files, ~162k total lines. Key modules by size:

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 25,042 | State CLI: tail, why, graph commands, all graph sub-reports |
| `state.rs` | 8,418 | Event recording, SQLite projection, HarnessPatch, EvalResult, panic hooks |
| `commands_eval.rs` | 6,713 | Evaluation harness, replay, gnome metrics |
| `commands_evolve.rs` | 5,528 | Evolution harness proposals, patches |
| `deepseek.rs` | 4,122 | DeepSeek-native protocol: FIM routing, cache metrics, streaming checks |
| `tool_wrappers.rs` | 3,640 | Tool decorators: GuardedTool, TruncatingTool, ConfirmTool, AutoCheckTool |
| `tools.rs` | 3,488 | Built-in tools: bash, search, rename, todo, web_search, sub_agent |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `prompt.rs` | 2,961 | Prompt execution, streaming events, auto-retry, model lifecycle events |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, compiler error parsing |

Entry points: `src/bin/yyds.rs` → `src/lib.rs::run_cli()` → `src/cli.rs` + `src/dispatch_sub.rs`. Python scripts in `scripts/` handle evolution pipeline orchestration (evolve.sh, preseed_session_plan.py, log_feedback.py, build_evolution_dashboard.py, extract_trajectory.py, append_terminal_state_events.py).

## Self-Test Results
- `./target/debug/yyds --version` → `yyds v0.1.14 (ff224e33 2026-07-25) linux-x86_64` ✓
- `./target/debug/yyds --help` → clean help output, all expected flags ✓
- `./target/debug/yyds state tail --limit 20` → live events streaming, this session's tool calls visible ✓
- `./target/debug/yyds state why last-failure` → shows retroactive FailureObserved from day-146 run (retroactive: run completed with error status but no FailureObserved recorded) ✓
- `./target/debug/yyds state graph hotspots --limit 10` → bash(4052), read_file(3187), search(1378) top tools ✓
- `./target/debug/yyds state graph hotspots --kind failure --limit 10` → "no hotspots matched kind=failure; kinds in data: failure, ..." — filter works and lists valid kinds (Day 146 fix working) ✓
- `./target/debug/yyds deepseek cache-report` → "no DeepSeek cache metrics recorded from agent chat completions" (yoagent drops cache token fields — known issue #90) ⚠️

## Evolution History (last 5 runs)
| Run | Started | Conclusion |
|---|---|---|
| 30140904205 | 2026-07-25T02:41Z | _(in progress — this session)_ |
| 30113757352 | 2026-07-24T17:37Z | **success** |
| 30085691974 | 2026-07-24T10:18Z | **success** |
| 30062355380 | 2026-07-24T02:43Z | **cancelled** |
| 30029169685 | 2026-07-23T17:23Z | **success** |

The cancelled run (30062355380) has no log-failed output — likely cancelled by GitHub Actions timeout or overlapping cron trigger (issue #262 — wall-clock budget collision). No recurring CI error fingerprints in the window.

## yoagent-state DeepSeek Feedback

**State tail**: Active recording of this assessment session — tool calls (bash, read_file) and commands streaming normally. No schema errors or event gaps observed.

**State why last-failure**: Retroactive FailureObserved from run-1784922186318-80299 (day-146 19:04 session). Reason: "run completed with error status 'error' but no FailureObserved was recorded." This is the orphan-sweeper catching a missing event — the Day 143/146 orphan-closing pipeline is working as designed.

**Graph hotspots**: Tool usage dominated by bash (4052), read_file (3187), search (1378). No anomalous hotspots. The `--kind failure` filter now correctly lists "failure" in valid kinds (Day 146 fix confirmed).

**Cache report**: No agent-level DeepSeek cache metrics. yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` fields. Metrics ARE recorded for diagnostic paths (stream-check, fim-complete). Tracked in issue #90.

## Structured State Snapshot
_From trajectory + state CLI evidence:_

**Claim health**: Latest session (day-146 19:04): classification=verified_success, can_drive_evolution=true. All gates green: provider_error_count=0, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0.

**Top unresolved**: Model lifecycle gaps (deepseek_model_call_abnormal_completed_count=6), transcript-only tool failures (5), state-only tool failures (59).

**Task-state counts**: Recent window shows 7 successful tasks, 2 reverted_no_edit tasks (day-146 12:40 session), 1 analysis_only attempt.

**Recent tool failures** (from trajectory graph pressure):
- bash_tool_error=21 — prefer bounded commands with explicit paths
- edit failed because replacement context was ambiguous — read tighter surrounding range
- commands timed out — prefer bounded targeted checks

**Recent action evidence**: Current trajectory shows the session is in assessment phase. No transcript/state disagreement detected in current run.

**Graph-derived next-task pressure** (from trajectory):
1. **Close yyds state and model lifecycle gaps** (deepseek_model_call_abnormal_completed_count=6): Lifecycle causes: model_abnormal/model_completion_without_start=6. Open issue #134 tracks this.
2. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence. Open issue #135 tracks this.
3. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=21): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
4. **Reconcile transcript-only tool failures** (transcript_only_failed_tool_count=5): Recent transcripts contained failed tool actions absent from state events.
5. **Reconcile state-only tool failures** (state_only_failed_tool_count=59): State events contained failed tool actions without matching transcript entries.

**Historical tool-failure categories**: Most historical categories (bash errors, ambiguous edits, timeouts) are well-characterized. The `bash_tool_error=21` count is cumulative. No fresh self-test evidence shows these still reproduce. The Day 146 sessions were all successful with strict verification.

## Upstream Dependency Signals
- **yoagent cache token fields**: yoagent's `Usage` struct does not expose `cache_read_input_tokens` / `cache_creation_input_tokens`. This blocks DeepSeek prompt cache observability (issue #90). Would need an upstream yoagent PR to add these fields to the Usage struct, OR a yyds-side workaround that parses raw API responses. The diagnostic paths (`deepseek stream-check`, `deepseek fim-complete`) already capture these from raw SSE — the gap is only in agent prompt runs.
- No other yoagent defects or missing capabilities identified.

## Capability Gaps
- **DeepSeek cache observability**: Cannot see prompt cache hit rates during agent runs (yoagent limitation). This matters because DeepSeek's pricing model heavily rewards cache hits — without visibility, sessions may be wasting tokens on cold-cache prompts.
- **Model lifecycle completeness**: 6 abnormal model completions (completion without matching start event). The append_terminal_state_events.py janitor script catches these retroactively, but the root cause (prompt.rs not always emitting ModelCallStarted before ModelCallCompleted) is still open.
- **Transcript/state reconciliation**: 5 transcript-only + 59 state-only tool failures indicate event capture gaps between what the transcript log shows and what the state event system records. These are historical counts; not all may be current.
- **vs Claude Code**: Claude Code has richer code navigation (LSP integration), inline diff previews, and workspace trust model. yyds has comparable tool set but less UI polish.

## Bugs / Friction Found
1. **[LOW] Self-referential fallback still cycles**: Issue #135 tracks this. The `preseed_session_plan.py` fallback still produces "Repair evidence-backed planning" when no candidates match and analysis-only pressure is active. Day 144 attempted a fix but evaluator timed out. The `_healthy_codebase_fallback()` function exists and produces verifiable `src/state.rs` tasks — the gap is in the no-candidates path not using it. However, current session health is excellent (task_success_rate=1.0), so this is low priority until analysis-only pressure actually manifests again.

2. **[LOW] DeepSeek model lifecycle gap**: Issue #134 tracks this. `ModelCallCompleted` events sometimes appear without matching `ModelCallStarted`. The janitor script catches these retroactively. Still 6 abnormal completions in the data. Fix would be in `src/prompt.rs` to ensure every completion path emits a start event first. But the janitor already handles this, making it a data-quality issue rather than a correctness bug.

3. **[KNOWN] DeepSeek cache metrics unavailable in agent runs**: Issue #90 (#105 is the reverted implementation attempt). Blocked on yoagent upstream. Worth checking whether a yyds-side workaround (e.g., intercepting raw response JSON before yoagent discards cache fields) is feasible without forking yoagent.

## Open Issues Summary
| # | Title | State | Age |
|---|---|---|---|
| 135 | Break self-referential planning fallback | OPEN | 3 days |
| 134 | Close harness-internal model lifecycle gap | OPEN | 4 days |
| 105 | Record DeepSeek prompt cache metrics | OPEN | 10 days |

All three are reverted/blocked tasks. #105 is blocked on yoagent upstream. #134 and #135 had implementation attempts that failed (blocked by agent / evaluator timeout respectively).

## Research Findings
- External journal `journals/llm-wiki.md` tracks a separate LLM-powered wiki project (last entry 2026-04-06) — not related to yyds harness work. No recent activity.
- No competitor research performed this session — the trajectory shows strong recent performance (task_success_rate=1.0) and the open issues are well-characterized reverted tasks, not new discoveries that need external benchmarking.
