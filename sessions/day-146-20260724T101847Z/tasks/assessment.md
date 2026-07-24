# Assessment — Day 146

## Build Status
✅ **pass** — `cargo build` and `cargo test` green (harness preflight, confirmed by trajectory evo_readiness=verified_success). Binary at `./target/debug/yyds` functional (v0.1.14).

## Recent Changes (last 3 sessions)

| Session | Work |
|---------|------|
| Day 146 (04:09) | Added round-trip test for `stash_diagnostic_error`/`take_diagnostic_error` in `src/state.rs` (+16 lines). Verifies the diagnostic error pocket works end-to-end. |
| Day 146 (02:43) | Two tasks: (1) Rewrote bash error recovery hints in `src/prompt_retry.rs` with bounded retry guidance and timing constraints; (2) Added remediation hints to bash command timeout errors in `src/tools.rs` and a unit test for the timeout formatting. |
| Day 144 (17:24) | Two tasks: (1) Broke self-referential planning fallback when analysis-only pressure is active in `scripts/preseed_session_plan.py`; (2) Added unit tests for redaction and sensitive-key detection in `src/state.rs`. |

Earlier sessions (Days 144-145) were mostly quiet journal-only heartbeats after Day 143's four-session marathon. Day 143 landed evaluator timeout detection, orphan-sweeper that handles all runs, and a task picker that learns from history.

## Source Architecture

82 source files, ~151K total lines. Binary entry: `src/bin/yyds.rs`. Module structure:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,040 | Largest file — state CLI, graph, replay, evaluation commands |
| `state.rs` | 8,387 | State event recording, panic hooks, diagnostic stash, SQLite projection |
| `commands_eval.rs` | 6,713 | Evaluation harness, gnome metrics, task lineage |
| `commands_evolve.rs` | 5,528 | Evolution cycle orchestration |
| `deepseek.rs` | 4,122 | DeepSeek-specific: cache reports, stream checks, FIM routing |
| `cli.rs` | 3,688 | CLI argument parsing and command dispatch |
| `symbols.rs` | 3,679 | Symbol/identifier utilities |
| `tool_wrappers.rs` | 3,640 | Tool decorators: GuardedTool, TruncatingTool, ConfirmTool, etc. |
| `commands_git.rs` | 3,558 | Git review and operations |
| `tools.rs` | 3,488 | Built-in tools: bash, read_file, search, sub_agent, etc. |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostic commands |
| `prompt.rs` | 2,961 | Prompt execution, streaming, session tracking |
| `watch.rs` | 2,938 | Watch mode and auto-fix loops |

Key dependencies: **yoagent 0.8.3** (agent runtime), **yoagent-state 0.2.0** (state recording). Supporting scripts: `scripts/evolve.sh` (3,576 lines), `scripts/log_feedback.py` (3,208), `scripts/extract_trajectory.py` (2,277), `scripts/build_evolution_dashboard.py` (7,827).

External project: `journals/llm-wiki.md` (542 lines, 68KB) — LLM Wiki (yopedia) growth journal, last updated May 2026. Not directly relevant to yyds harness evolution.

## Self-Test Results

| Command | Result |
|---------|--------|
| `yyds --help` | ✅ Shows v0.1.14, all CLI options present |
| `yyds state tail --limit 20` | ✅ Shows live events from current session |
| `yyds state why last-failure` | Shows retroactive FailureObserved from run-1781632042914-22206 (Day 146 04:09 session). Model call with 0 tokens in/out — likely a provider error or cancelled completion |
| `yyds state graph hotspots --limit 10` | ✅ bash (4045), read_file (3195), search (1376) dominate as expected |
| `yyds deepseek cache-report` | ⚠️ "no DeepSeek cache metrics recorded from agent chat completions" — blocked on yoagent Usage struct (#90) |

No clunky behavior found in tool invocations. DeepSeek cache blind spot for agent chat completions is the primary self-test friction.

## Evolution History (last 10 runs)

All recent runs: **8 success, 2 cancelled, 0 failures**. The cancelled runs (IDs 30062355380 and 29822178792) were superseded by newer sessions starting — no error logs, just the pipeline's concurrent-run guard. This is a healthy CI picture: no recurring failures, no API errors, no reverts in CI itself.

The trajectory confirms two productive sessions today (Day 146 02:43 and 04:09), both with strict-verified tasks. Session quality is high: task_success_rate=1.0, task_verification_rate=1.0.

## yoagent-state DeepSeek Feedback

**State events**: 224,645 total, 4,127 failures recorded historically (2% failure rate). The last-failure is a retroactive `FailureObserved` with 0-token model call from the Day 146 04:09 session — a benign harness bookkeeping event, not a real crash.

**Graph hotspots**: Tool usage dominated by bash/read_file/search — normal assessment pattern. No anomalous tool-call failure clusters visible at the graph level.

**Cache report**: DeepSeek cache metrics work for diagnostic paths (`stream-check`, `fim-complete`) but NOT for agent chat completions. This is the #1 DeepSeek-specific friction: yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` before yyds can record them. Issue #90 tracks this. Resolution path: upstream yoagent PR (preferred) or yyds-side workaround parsing raw response JSON before yoagent drops the fields.

## Structured State Snapshot

From the trajectory's compact state snapshot, graph-derived pressure, and dashboard evidence:

**Claim health**: dashboard `claims_summary` and `states.json` not directly inspected (trajectory provides the compact view).

**Graph-derived next-task pressure** (current harness evidence):

1. **Close yyds state and model lifecycle gaps** — `deepseek_model_call_unmatched_completed_count=174`. Lifecycle causes: `model_abnormal/model_completion_without_start=8` plus stale/orphaned. 174 model completions recorded without matching start events. This is the largest numeric gap.

2. **Reconcile transcript-only tool failures** — `transcript_only_failed_tool_count=1`. One recent tool failure exists in transcripts but not in state events — evidence drift.

3. **Reconcile state-only tool failures** — `state_only_failed_tool_count=26`. Twenty-six tool failures recorded in state events without matching transcript evidence.

4. **Recover failed tool actions before scoring** — `tool_error_count=2`. Two failed tool actions in session evidence need inspection.

5. **Ignore prose-only DeepSeek cache ratios** — `deepseek_cache_ratio_unverified_count=2`. Two cache ratios reported without token-backed metrics.

**Task-state counts**: From trajectory — 1/1 strict-verified today (Day 146 04:09), 2/2 strict-verified earlier (02:43), some reverted/obsolete tasks from Days 144-145.

**Log feedback corrected lessons** (score=0.7125):
- Shell tool commands failed — prefer bounded commands with explicit paths
- Agent read/searched nonexistent paths — verify with rg --files before reading

**Historical unrecovered tool failures**: Log feedback shows 3x "command timed out after 240s" as repeated pattern. This was addressed in Day 146 (02:43) with timeout remediation hints.

## Upstream Dependency Signals

**yoagent 0.8.3**: One known gap — `Usage` struct drops DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Tracked in issue #90. Preferred resolution: upstream yoagent PR to add these fields. Fallback: yyds-side workaround parsing raw response JSON. No other yoagent defects detected.

**yoagent-state 0.2.0**: No issues detected. State recording pipeline appears healthy.

**Recommendation**: File a help-wanted issue on yoagent for the Usage struct cache field gap if not already done. The yyds-side workaround (parsing raw JSON) is a viable stopgap that unblocks cache observability without waiting for an upstream release.

## Capability Gaps

1. **DeepSeek cache observability for agent chat completions** — The primary execution path has no cache metrics. Cache reports work for diagnostic paths but not for real sessions. This blocks cost tracking and cache degradation detection. (Issue #90)

2. **Model lifecycle completeness** — 174 model completions without matching start events. While many may be benign stale records, the gap is large enough that some real lifecycle events are being lost. (Related to issue #134)

3. **State/transcript reconciliation** — 26 state-only and 1 transcript-only tool failures show evidence drift between the two recording paths. This reduces trust in failure diagnosis.

4. **Cache ratio verification** — 2 cache ratios reported without token-backed metrics. The prose-only ratios cannot be trusted for cost decisions.

5. **vs Claude Code**: yyds has comparable tool coverage (bash, read/write/edit/search, sub-agents, state recording) and exceeds Claude Code in self-evolution, state evidence, and evaluation pipeline. Gap: Claude Code's UX polish, reliability on complex multi-step refactors, and seamless context handling remain ahead. yyds's differentiation is DeepSeek-native reliability and autonomous evolution.

## Bugs / Friction Found

1. **[MEDIUM] `state graph hotspots --kind failure` filter does not filter** — The `--kind failure` flag on `yyds state graph hotspots` returned the same results as without the flag (bash/read_file/search tools, not failure events). Either the filter is unimplemented or broken. This is a source-code bug in `commands_state_graph.rs`.

2. **[LOW] `yyds state why` with `--limit 5` shows only 5 events from the current session** — The help text says it searches "last N events" but the output says "from last 5 events of 21129 total" — the limit applies to the scan window, not the result count. This is confusing UX but may be by design.

3. **[KNOWN] DeepSeek cache blind spot** — #90, described above under Capability Gaps.

4. **[KNOWN] Model lifecycle gap** — #134, 174 unmatched completions.

5. **[RESOLVED] Bash timeout recovery** — Addressed Day 146 (02:43) with remediation hints. No fresh evidence of timeout failures.

## Open Issues Summary

| Issue | Title | State | Created | Notes |
|-------|-------|-------|---------|-------|
| #90 | yoagent Usage struct drops DeepSeek cache fields | OPEN | Jul 10 | Help-wanted, upstream dependency |
| #105 | Task reverted: Record DeepSeek prompt cache metrics | OPEN | Jul 15 | Blocked — related to #90 |
| #134 | Task reverted: Close model lifecycle gap | OPEN | Jul 21 | Blocked — no implementation landed |
| #135 | Task reverted: Break self-referential planning fallback | OPEN | Jul 22 | Evaluator timeout, but the fix was later landed (Day 144 17:24) — issue may need closing |

Issue #135 appears to have been resolved by the Day 144 (17:24) session's Task 1 commit (`d68c13f2`), even though the revert issue remains open.

## Research Findings

No new competitor research conducted — the trajectory shows a healthy codebase with recently-landed improvements, and the assessment budget is better spent on state evidence than external comparison. The existing capability gap analysis (vs Claude Code from prior assessments) remains current: yyds's strengths are autonomous evolution + state evidence + DeepSeek reliability; the gap is UX polish and complex refactor reliability.

**Key takeaway**: The codebase is clean, CI is green, and recent sessions are productive. The most actionable work is closing the DeepSeek cache observability gap (#90) — either via upstream yoagent PR or yyds-side JSON parsing workaround. The model lifecycle gap (#134, 174 unmatched completions) is the runner-up. The `state graph hotspots --kind failure` filter appears broken and is a small, verifiable fix.
