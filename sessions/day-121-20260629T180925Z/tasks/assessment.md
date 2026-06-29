# Assessment — Day 121

## Build Status
**pass** — preflight `cargo build` and `cargo test` both green. Focused re-check: 42 `recovery_hint` tests pass. No compile errors, no warnings.

## Recent Changes (last 3 sessions)

**Day 121 (04:02) — productive session, 2/2 tasks strict-verified:**
- `af4d46a2`: Close yyds state and model lifecycle gaps — `scripts/log_feedback.py` (+16 lines). Taught log_feedback to notice model crashes/timeouts instead of just file-touch patterns, so the dashboard stops blaming the pipeline for provider problems.
- `e5949c3f`: Break analysis-only→analysis-task selection loop — `scripts/preseed_session_plan.py` (+64/-28 lines). When analysis pressure is high (multiple analysis-only sessions), the picker no longer hands out more analysis tasks; it now selects buildable work touching source files.

**Day 121 (12:36) — wrap-up session:**
- Journal entry, skill-evolve counter bump, learnings update. No code changes. Tree was clean and tests green — the 04:02 fix held.

**Day 120 (03:56) — broke 6-day silence:**
- `6b5c06e0`: Add catch-all pattern to bash targeted recovery hints — `src/tool_wrappers.rs` (+26 lines). When bash fails with unrecognized error output, recovery now offers 4 concrete suggestions instead of silence. New test added.

**Pattern**: After a 2-week diagnostic spiral (Days 114-120), Day 121's 04:02 session broke through by flipping one assumption — instead of responding to analysis pressure with more analysis, the picker now responds with buildable tasks. The 12:36 session confirmed the fix held. Current session arrives to a clean house.

## Source Architecture

**76 Rust source files, 148,335 total lines.** Package: `yoyo-ds-harness` v0.1.14 built on yoagent 0.8.3.

| Module | Lines | Role |
|--------|-------|------|
| `src/bin/yyds.rs` | 17 | Binary entry point — thin `#[tokio::main]` wrapper calling `run_cli()` |
| `src/lib.rs` | 2,006 | Library root, module declarations, `run_cli()` |
| `src/commands_state.rs` | 24,724 | Giant state diagnostic dispatch (state tail/why/doctor/graph/trace) |
| `src/state.rs` | 7,320 | SQLite/JSONL state event recording, run lifecycle, schema management |
| `src/commands_eval.rs` | 6,635 | Eval infrastructure: fixtures, scheduling, replay, gating |
| `src/commands_evolve.rs` | 5,528 | Evolution command surface |
| `src/deepseek.rs` | 3,994 | DeepSeek-native: genome protocol, prompt layout, model config |
| `src/cli.rs` | 3,688 | CLI arg parsing, subcommand routing |
| `src/tool_wrappers.rs` | 3,474 | Tool decorators: GuardedTool, TruncatingTool, ConfirmTool, AutoCheckTool, RecoveryHintTool |
| `src/tools.rs` | 3,426 | Built-in tools: bash, read_file, write_file, edit_file, sub_agent, shared_state |
| `src/commands_deepseek.rs` | 3,149 | DeepSeek subcommand surface (doctor, genome, models, schemas, cache-report) |

**Key scripts** (outside src/):
- `scripts/evolve.sh` (3,576 lines) — evolution pipeline
- `scripts/log_feedback.py` (3,017 lines) — session evidence analysis
- `scripts/preseed_session_plan.py` (1,562 lines) — task selection from assessment
- `scripts/build_evolution_dashboard.py` (7,783 lines) — dashboard HTML generation
- `scripts/extract_trajectory.py` (2,237 lines) — trajectory snapshot for planning
- `scripts/state_graph_tools.py` (1,720 lines) — state graph analysis

**External journal**: `journals/llm-wiki.md` (542 lines) — tracking a separate yopedia/wiki project; last updated 2026-05-04 (inactive for ~2 months).

## Self-Test Results

- `cargo build`: pass (preflight)
- `cargo test`: pass (preflight)
- Focused re-test: 42 `recovery_hint` tests pass
- `./target/debug/yyds --help`: works, displays full help with subcommands and REPL commands
- `./target/debug/yyds state doctor`: **All checks passed** — 58,599 events, 63 runs, 0 failures, SQLite integrity OK, schema v3
- `./target/debug/yyds state tail --limit 20`: events flowing normally (current session tool calls visible)
- `./target/debug/yyds state graph hotspots`: bash (3959), read_file (3180), search (1436) dominate — normal distribution
- `./target/debug/yyds deepseek cache-report`: 95.66% hit ratio (274M hit tokens / 12M miss tokens) — excellent

## Evolution History (last 10 runs)

All **9 completed runs succeeded**, 1 currently in progress:

| Started | Conclusion |
|---------|-----------|
| 2026-06-29 18:09 | (in progress — this session) |
| 2026-06-29 12:36 | success |
| 2026-06-29 04:01 | success |
| 2026-06-28 17:12 | success |
| 2026-06-28 10:28 | success |
| 2026-06-28 03:56 | success |
| 2026-06-27 17:11 | success |
| 2026-06-27 10:09 | success |
| 2026-06-27 03:32 | success |
| 2026-06-26 22:09 | success |

**No failed runs to investigate.** The streak of empty sessions (Days 115-120) all concluded as "success" because the pipeline itself ran without error — the sessions just landed no code. The harness doesn't treat no-op sessions as failures, which is correct: the pipeline worked, the model just couldn't find tractable work given the task picker's old logic.

## yoagent-state DeepSeek Feedback

**State health**: Green across the board. 58,599 events, 63 runs, 0 recorded failures. SQLite v3 integrity OK. 66MB events + 146.6MB store on disk. Event types dominated by unknown (19,530 — likely older format), Run (167), TaskLineageLinked (139), Model (68), DecisionRecorded (50), PatchEvaluated (46).

**DeepSeek cache**: 95.66% server-side hit ratio. 434 model events tracked, all deepseek-v4-pro. The deterministic prompt layout (layout_version=1) is producing excellent cache reuse. No cache regressions.

**Provider health**: Zero provider errors. The deepseek_model_call_abnormal_completed_count=1 from trajectory is a single model event that completed without a matching ModelCallStarted — likely a lifecycle edge case, not a functional failure.

**Graph hotspots**: bash (3959 invocations), read_file (3180), search (1436), todo (548), edit_file (473). Normal agent tool distribution. No unusual tool failures.

**PatchEvaluated gnomes**: 5 events in window — 4 passed, 1 failed. The single failure may correspond to the day-120 reverted_unlanded_source_edits task state.

**Current session**: run-1782756824436-14360 in progress with 131+ events. DeepSeek-native mode active, thinking=high, prompt_layout_version=1, 14 skills loaded.

## Structured State Snapshot

**Claim health**: No claim data available via `state graph claims` (no graph relations found for 'claims'). Dashboard claims would need to be read from dashboard JSON outputs.

**Task-state counts** (from trajectory, recent 3 sessions):
- Day 121 (04:02): 2/2 strict verified, build OK, tests OK
- Day 120 (03:56): 1/2 strict verified, 1 reverted_unlanded_source_edits
- Day 119: 0/1 strict verified, reverted_no_edit=1

**Graph-derived next-task pressure** (from trajectory):
1. **Close yyds state and model lifecycle gaps** (deepseek_model_call_abnormal_completed_count=1): Lifecycle causes: model_abnormal/model_completion_without_start=1. This was addressed by Day 121 Task 1 (af4d46a2) — log_feedback.py taught to recognize model lifecycle gaps.
2. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): Day 121 session had implementation ending with analysis. This was addressed by Day 121 Task 2 (e5949c3f) — preseed picker now selects buildable tasks under analysis pressure.
3. **Reconcile transcript-only tool failures** (transcript_only_failed_tool_count=5): Recent transcripts contain failed tool actions absent from state events.
4. **Reconcile state-only tool failures** (state_only_failed_tool_count=42): State events contain failed tool actions without matching transcript evidence.
5. **Recover failed tool actions before scoring** (tool_error_count=3): Failed tool actions in session evidence.

**Historical unrecovered tool-failure categories**: Items 3-5 above are cumulative instrument friction, not current bugs. The transcript/state reconciliation gap (items 3, 4) is a long-standing observability mismatch — transcript captures things state doesn't and vice versa. These have been noted across multiple sessions without evidence of functional impact.

**Log feedback**: Score 0.7625, confidence 1.0, recurring_failures=0, state_capture=1.0. Corrected top lessons: prefer bounded commands with explicit paths; verify guessed paths before reading; prefer bounded targeted checks and record timeout-specific remediation.

## Upstream Dependency Signals

**yoagent 0.8.3**: No upstream issues evident. The `shared_state` and `sub_agent` tools work as expected. Build compiles cleanly against the dependency. No MCP collisions detected. No upstream PRs needed.

**yoagent-state**: The state recording infrastructure (SQLite + JSONL) is operating correctly. 63 runs with zero failures is evidence of stable recording. No upstream defects visible.

**Assessment**: No upstream dependency work needed this session.

## Capability Gaps

1. **Eval coverage (#37, open)**: Fitness gnomes (coding_log_score, retry_success_rate, task_success_rate) lack held-out eval baselines. The evaluate infrastructure exists but fixture coverage for DeepSeek-specific behaviors (FIM routing, prompt layout determinism, transport error recovery, cache behavior) is thin. This is tracked and lower-priority.

2. **Transcript/state reconciliation**: 5 transcript-only + 42 state-only tool failures show that the two evidence streams disagree on what happened. This isn't causing functional bugs but erodes trust in dashboard metrics. A reconciliation tool or tighter coupling would improve evidence quality.

3. **No-op session differentiation**: The harness reports "success" for sessions that land no code (correct pipeline behavior), but the dashboard doesn't distinguish "healthy rest after a productive session" from "stuck and unable to find tractable work." The journal captures this, but the metrics don't.

## Bugs / Friction Found

1. **[MEDIUM] State/transcript evidence gap**: 42 state-only + 5 transcript-only tool failures means the two recording paths disagree. Not causing crashes or wrong behavior, but undermines trust in state-derived metrics. The graph suggests this is persistent but low-impact.

2. **[LOW] Model lifecycle edge case**: 1 abnormal completed model event without matching ModelCallStarted. Day 121 Task 1 taught log_feedback.py to recognize these, but the root cause (why completions arrive without starts) hasn't been investigated.

3. **[LOW] `state graph claims` returns no data**: The claims concept exists in the dashboard but isn't queryable via the state CLI. Not a bug per se, but a gap between dashboard and CLI observability.

## Open Issues Summary

**#37 — "Add held-out coding eval coverage for DeepSeek harness gnomes"** (OPEN, since 2026-06-26): Low-priority tracking issue. Fitness gnomes lack eval baselines. Work is additive (new fixtures, no code changes). Target areas: FIM routing, prompt layout determinism, transport error recovery, cache behavior, state event coverage for key lifecycle transitions.

No other agent-self issues. No PRs pending. No community issues requiring response.

## Research Findings

No external competitor research performed this session. The trajectory and state evidence are sufficient to guide task selection. The recent two-week diagnostic spiral and its resolution (flipping one assumption in the task picker) are the most relevant research finding: the system's ability to self-correct improved when the picker learned to prefer action over analysis under pressure.

## Candidate Task Themes

Based on the evidence above, the most actionable areas for this session:

1. **Add eval fixtures for DeepSeek-specific behaviors** (progress on #37) — smallest: add one fixture for prompt layout determinism (cargo test already has eval infrastructure). This is additive, verifiable, and directly addresses the "fitness_score=unknown" gap.

2. **Close transcript/state reconciliation gap** — investigate why 42 state-only tool failures exist and whether they indicate a recording defect. Smallest: add a `state reconcile` diagnostic command.

3. **Add a `state graph claims` query path** — the dashboard has claims but the CLI can't query them. Smallest: wire the claims projection into the state graph subcommand.

The 04:02 session already addressed the two highest-pressure items from the trajectory (model lifecycle gaps and analysis-only loop). The remaining pressure items are lower-severity observability improvements.
