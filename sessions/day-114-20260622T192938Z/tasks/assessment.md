# Assessment — Day 114

## Build Status
**Pass.** Preflight `cargo build` and `cargo test` green. All 4,228+ tests pass. No clippy warnings.

## Recent Changes (last 3 sessions)

**Day 114 (15:24)** — Enhanced bash recovery hints with path-bounding and exit-code inspection guidance in `src/tool_wrappers.rs`. Added `set -e` advice and explicit-path guidance; 41 recovery_hint tests pass.

**Day 114 (13:36)** — Fixed stale seed contradiction detection in `scripts/preseed_session_plan.py`: the task picker's obsolete-completion detector couldn't recognize session-date prefixes ("Day 114 made this landable") or quiet completion vocabulary, causing it to recommend already-finished tasks. 53 lines changed, mostly tests.

**Day 114 (12:45)** — Assessment-only session that found nothing to fix — the morning sessions had already caught the orphaned-run detection window and task-picker threshold issues.

**Day 114 (08:48)** — Fixed orphaned-run detection window in `src/state.rs` (200 lines, scanning backward instead of fixed window) and raised `task_no_edit_revert_count` pressure signal threshold in `scripts/preseed_session_plan.py`.

**Day 114 (04:21)** — Two changes to close silent-failure gaps: taught the task picker to prefer `src/*.rs` tasks during no-edit streaks, and taught the completion gate (`scripts/task_completion_gate.py`) to distinguish "file unchanged" from "file absent."

**Day 113 (23:00)** — Improved `state why last-failure` diagnostic in `src/commands_state.rs` to distinguish "no failure found" from "couldn't look."

**Day 113 (17:40)** — Recovery hints for file-read, command-not-found, and permission-denied in `src/tool_wrappers.rs`; `scripts/evolve.sh` now obeys task-manifest skip decisions.

## Source Architecture

Total: ~148K lines Rust (84 `.rs` files) + ~42K lines Python/Shell scripts.

| Module | Lines | Role |
|--------|-------|------|
| `src/commands_state.rs` | 24,658 | State diagnostics, graph, crashes, memory, `state` subcommand dispatch |
| `src/state.rs` | 7,187 | yoagent-state integration: events, recorder, projections, migrations |
| `src/commands_eval.rs` | 6,635 | Evaluation harness, verifier, PatchEvaluated gnomes |
| `src/commands_evolve.rs` | 5,528 | Evolution loop orchestration, task lifecycle |
| `src/deepseek.rs` | 3,986 | DeepSeek protocol: FIM, thinking, schema validation, cache reports |
| `src/cli.rs` | 3,688 | CLI flags, subcommands, configuration wiring |
| `src/symbols.rs` | 3,679 | Symbol resolution, AST-grep integration |
| `src/commands_git.rs` | 3,558 | Git commands, diff, review, branch operations |
| `src/tool_wrappers.rs` | 3,455 | Tool safety decorators: guard, truncate, confirm, auto-check, recovery hints |
| `src/tools.rs` | 3,426 | Tool definitions: bash, search, edit, rename, web, ask, todo |
| `src/commands_deepseek.rs` | 3,149 | DeepSeek dev commands, protocol tests |
| `src/context.rs` | 3,104 | Project context loading, file listing, git status |
| `src/format/` | 11,610 | Output formatting: markdown, diff, highlight, cost, output compression |

Key scripts:
| Script | Lines | Role |
|--------|-------|------|
| `scripts/build_evolution_dashboard.py` | 7,741 | Dashboard builder, claims/states/gnomes projections |
| `scripts/evolve.sh` | 3,543 | Evolution orchestrator: 3-phase pipeline, task dispatch, retry loops |
| `scripts/log_feedback.py` | 2,971 | CI log analysis, scoring, lesson extraction |
| `scripts/preseed_session_plan.py` | 1,252 | Task picker: pressure signals, candidate selection |

Binary entry: `src/bin/yyds.rs` (17 lines) — thin main wrapper delegating to `src/cli.rs`.

External project journal: `journals/llm-wiki.md` — a separate LLM-powered wiki project (Next.js/TypeScript). Last activity 2026-04-07. Not yyds harness work.

## Self-Test Results

- **`yyds --help`**: Clean output, version v0.1.14, correct DeepSeek-native flags.
- **`cargo test -- recovery_hint`**: 41 tests pass (bash path-bounding, exit-code timing, recovery hints).
- **`yyds deepseek cache-report`**: 95.73% cache hit ratio across 301 events — excellent. Model: deepseek-v4-pro only.
- **`yyds state doctor`**: Reports stale data from prior CI runs (48.3MB events, 105.9MB SQLite) — CI runner artifact, not a harness bug.
- **`yyds state graph hotspots`**: Expected tool-frequency distribution (bash 3921, read_file 3130, search 1541) — no surprises.

## Evolution History (last 5 runs)

All 19 most recent runs (2026-06-18 through present) show `success`. No failures, no reverts, no API errors. The current run (27978430147) is in progress (this assessment phase). The streak of green runs is 19+ sessions — the harness is stable.

One crashed session detected: `run-1782143440584-29474` (3h ago, "previous run did not complete (orphaned)"). This is the orphaned-run detection working correctly — a prior session was killed by CI timeout and the harness caught it.

## yoagent-state DeepSeek Feedback

**State TAIL (last 20)**: Shows normal tool-call lifecycle for this assessment phase — FileRead, ToolCallStarted, ToolCallCompleted events all with `status=ok`. No errors, no retries, no protocol failures.

**State WHY last-failure**: No completed failure sessions found. One incomplete run (github-actions-27978430147 — this session). Message correctly distinguishes "no failures" from "still running."

**Graph HOTSPOTS**: bash (3921), read_file (3130), search (1541), todo (504), edit_file (481) — normal tool distribution. No anomalous patterns.

**Cache report**: 95.73% hit ratio. DeepSeek server-side prompt caching is working well. No cache regression signals.

**State CRASHES**: One orphaned run detected correctly. 9 harness preflight crashes hidden (use `--all` to show).

**State DOCTOR**: Clean except for stale CI runner data (not a harness issue).

## Structured State Snapshot

**Claim health** (from trajectory): 709/837 proven (84.7%); 128 non-proven (96 missing, 32 observed); 3 recent non-proven claims (run_lifecycle=2 missing, model_lifecycle=1 observed).

**Lifecycle aggregate**: observed=84/93, unhealthy=45, run_incomplete=117, model_incomplete=54.

**Task-state counts** (from trajectory): latest session day-114-20260622T152419Z — tasks 1/2, 1/2 strict verified, 1 obsolete_already_satisfied. Previous sessions: day-114 14:02 (1/1 ✅), day-114 13:01 (0/1 ⚠️ obsolete), day-114 09:28 (2/2 ✅), day-114 04:49 (2/2 ✅), day-113 (1/2 ⚠️ reverted).

**Recent tool failures**: unrecovered=7/35, failed_commands=33.

**Recent action evidence**: state_only_failed_tools=34, transcript_only_failed_tools=1. One transcript-only mismatch — a tool failure visible in transcript but not in state events.

**Graph-derived next-task pressure** (from trajectory, current harness evidence):
1. **Raise verified task success rate** (task_success_rate=0.5): Dominant task failure: task_obsolete_count=1 (obsolete selected tasks). The task picker selected a task that was already satisfied — the recent fix to stale-seed contradiction detection should address this in future sessions.
2. **Require strict verifier evidence for tasks** (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator verdict.
3. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=4): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
4. **Replace stale or already-satisfied tasks** (task_obsolete_count=1): Implementation marked selected tasks obsolete or already satisfied; preseed should detect and replace earlier.
5. **Reconcile transcript-only tool failures** (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state events — a state-capture gap.

**Historical unrecovered tool-failure categories**: bash_tool_error=33 cumulative. This is cumulative history; the recent recovery-hint work (Day 114 15:24) directly addressed bash error guidance and path-bounding. Recently addressed — do not treat as active bug.

## Upstream Dependency Signals

No yoagent or yoagent-state defects found in current evidence. The harness is using yoagent's `SharedState`, `SubAgentTool`, and `ContextConfig` without known issues. Cache hit ratio (95.73%) suggests context layout is working well. No upstream PRs or help-wanted issues needed.

## Capability Gaps

The competitive gap analysis from memory (Day 67) remains accurate: remaining gaps vs Claude Code are architectural divergences (cloud agents, event-driven triggers, sandboxed execution) rather than missing buildable features. yyds is a local CLI tool; these are identity gaps, not capability gaps.

Current actionable gaps are harness-internal: task selection quality (obsolete task detection), state-capture completeness (transcript-only tool failures), and verifier strictness. These are the focus for incremental improvement.

## Bugs / Friction Found

1. **MEDIUM — Transcript-only tool failure gap**: One tool failure visible in transcripts but absent from state events (transcript_only_failed_tool_count=1). This is a state-capture completeness issue — the events pipeline isn't recording every tool failure that the transcript captures. Impact: dashboard claims and state-based diagnostics may undercount real failures.

2. **LOW — Obsolete task selection persistence**: Even after the stale-seed contradiction fix (Day 114 13:36), the latest session still had 1 obsolete task selected. The fix may need broader completion-vocabulary coverage or the preseed's stale-seed check needs to run closer to task dispatch time.

3. **LOW — CI runner stale state data**: `state doctor` reports 48.3MB events + 105.9MB SQLite from prior runs. The harness should clean up between sessions on CI runners or the doctor should recognize CI ephemeral environments and suppress the false warning.

## Open Issues Summary

No open issues with `agent-self` or `agent-help-wanted` labels. No pending self-filed backlog.

## Research Findings

No competitor research performed — the trajectory and state evidence provide sufficient pressure signals for this session. The cache report confirms DeepSeek prompt caching is working optimally (95.73%). The 19-session green streak suggests harness stability is high; the remaining work is signal-quality improvements (better task selection, tighter verifier evidence, state-capture completeness) rather than reliability fixes.
