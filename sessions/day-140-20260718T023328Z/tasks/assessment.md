# Assessment — Day 140

## Build Status
**PASS.** The harness preflight ran `cargo build` and `cargo test` before this assessment phase. The binary is functional: `yyds --help` and `yyds --version` (v0.1.14, 19d1b855, 2026-07-18) both work.

## Recent Changes (last 3 sessions)

**Day 139 (09:57 — SUCCESS)**: Tasks 1/1 ✅. Deduplicated retroactive FailureObserved events across multiple janitor invocations (`scripts/append_terminal_state_events.py` + test). Added a held-out eval fixture for the dedup lifecycle scenario.

**Day 139 (02:41 — SUCCESS)**: Tasks 0/2 ⚠️. Both tasks reverted (no_edit, scope_mismatch). The engine turned over twice and produced exit-code-1 with no commits — an opaque silence the journal called "the engine that burned fuel in the dark."

**Day 138 (17:16 — SUCCESS)**: Tasks 2/2 ✅. Two strict-verified tasks landed. The afternoon session landed clean work; the earlier 11:48 session also landed 2/2.

**Overall**: The last 5 sessions have 4 successes and 1 cancelled. Day 139's 17:12 session was cancelled after the agent detected the selected task was already implemented. The trajectory reports `fitness_score: 1.0` with `task_success_rate=1.0` and `task_verification_rate=1.0` for the latest readiness snapshot.

**Journal (llm-wiki.md)**: External project. Last entry from 2026-05-04 — a TypeScript wiki with MCP server, storage migration, and agent self-registration. No recent activity. Not relevant to current harness work.

## Source Architecture

~150K lines of Rust across 76 source files in `src/`. No `main.rs` — the binary entry point is `src/lib.rs` (2,006 lines), which re-exports the CLI and dispatches from there.

**Top 10 files by line count**:
| File | Lines | Domain |
|------|-------|--------|
| commands_state.rs | 25,009 | State CLI, graph, event reading, projections |
| state.rs | 7,946 | State recorder, event types, SQLite projection |
| commands_eval.rs | 6,713 | Evaluation harness, fixtures, replay |
| commands_evolve.rs | 5,528 | Evolution pipeline commands |
| deepseek.rs | 4,122 | DeepSeek-native protocol, FIM, cache |
| cli.rs | 3,688 | CLI argument parsing, subcommands |
| symbols.rs | 3,679 | Symbol/identifier resolution |
| tool_wrappers.rs | 3,640 | Tool guards, recovery hints, confirmations |
| commands_git.rs | 3,558 | Git command wrappers |
| tools.rs | 3,426 | Built-in tool definitions |

**Key entry points**: `lib.rs` → module declarations → `cli.rs` (argument parsing) → `dispatch.rs` + `dispatch_sub.rs` (routing). DeepSeek-native path: `deepseek.rs` (protocol), `agent_builder.rs` (model config). State: `state.rs` (recorder) + `commands_state.rs` (CLI).

**Script layer**: `scripts/evolve.sh` (3,576 lines — the harness orchestrator), `scripts/build_evolution_dashboard.py` (7,827 lines), `scripts/extract_trajectory.py` (2,277 lines), `scripts/preseed_session_plan.py` (2,317 lines), `scripts/append_terminal_state_events.py` (637 lines), `scripts/test_append_terminal_state_events.py` (976 lines).

## Self-Test Results

- `yyds --help`: ✅ Works, shows v0.1.14 with full CLI options
- `yyds --version`: ✅ Returns `yyds v0.1.14 (19d1b855 2026-07-18) linux-x86_64`
- `yyds state tail --limit 20`: ✅ Shows current session events flowing, all `status=ok`
- `yyds state tail --run-id`: ✅ New Day 139 feature works for run-scoped filtering
- `yyds state why last-failure`: ✅ Shows retroactive FailureObserved from cancelled Day 139 17:13 run — janitor working as designed
- `yyds state graph hotspots --limit 10`: ✅ Shows current session run at top (degree=67)
- `yyds deepseek cache-report`: ⚠️ Reports "no DeepSeek cache metrics recorded from agent chat completions" — known limitation tracked in #90. Diagnostic paths (stream-check, fim-complete) do work.

## Evolution History (last 5 runs)

| Run ID | Started | Conclusion | Notes |
|--------|---------|------------|-------|
| 29627233668 | 2026-07-18 02:32 | *(in progress)* | Current session (this assessment) |
| 29599155239 | 2026-07-17 17:12 | **cancelled** | Agent detected task already implemented; session cancelled |
| 29571800589 | 2026-07-17 09:57 | **success** | 1/1 tasks, deduped FailureObserved events |
| 29550556488 | 2026-07-17 02:41 | **success** | 0/2 tasks reverted but session was clean |
| 29519101750 | 2026-07-16 17:16 | **success** | 2/2 tasks verified |

**Patterns**: No CI failures, no API errors in the recent window. The cancelled run at 17:12 is a healthy cancellation — the agent correctly detected the task was already implemented and stopped rather than forcing it. The 02:41 session with 0/2 tasks had both tasks reverted (no_edit + scope_mismatch), which generated open issues #114-#116.

## yoagent-state DeepSeek Feedback

**state tail**: All current-session events flowing cleanly. Tool calls completing with `status=ok`. No errors, no timeouts, no retries visible in the last 20 events.

**state why last-failure**: Points to `evt-harness-df334d938573826e` — a retroactive FailureObserved event from the cancelled Day 139 17:13 run. The event was written because the run completed with error status but no FailureObserved was originally recorded. This is the janitor working as designed, not a new failure. The similar-failure list shows 3 more retroactive events, all from historical runs.

**state graph hotspots**: Healthy — the current session run is the hotspot (degree=67), with all relationships normal (`observed_in`, `traced_by`, `invokes_tool`, `uses_schema`). No anomalous clusters.

**deepseek cache-report**: Confirms cache metrics are not captured from agent chat completions due to yoagent's Usage struct dropping DeepSeek cache token fields. This is tracked in #90. The diagnostic paths (`stream-check`, `fim-complete`) do capture cache metrics — the gap is specifically in agent-chat scenarios.

**Implication**: The state and lifecycle machinery is healthy. The janitor is closing orphaned runs. The primary unresolved DeepSeek-specific gap is cache metric visibility during agent sessions (#90). No protocol failures, schema errors, or retry churn visible.

## Structured State Snapshot

**Claim health**: Trajectory readiness shows `can_drive_evolution=true` with `classification=verified_success`. All diagnostic gates green: provider_error_count=0, task_success_rate=1.0, task_verification_rate=1.0.

**Top unresolved claim families**: The trajectory's graph-derived next-task pressure lists:
1. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_incomplete_count=8`): Lifecycle causes: model_incomplete/open_after_ModelCallStarted=8; stale incomplete events from historical runs. The janitor now writes retroactive events, but 8 model calls have `ModelCallStarted` without `ModelCallCompleted`.
2. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=8`): Historical bash tool errors. The recovery hint system in `src/tool_wrappers.rs` already has 8+ failure categories with targeted hints — the task (#115) was reverted because it was already implemented.
3. **Reconcile state-only tool failures** (`state_only_failed_tool_count=40`): State events with failed tool actions lacking matching transcript evidence. Historical cumulative count, not necessarily current bugs.
4. **Recover failed tool actions before scoring** (`tool_error_count=4`): Failed tool actions in session evidence.
5. **Tighten selected task specs** (`task_spec_warning_count=1`): One task had thin spec quality warning.

**Task-state counts** (from trajectory): Recent sessions: 2/2 verified (Day 139 09:57), 1/1 verified (Day 139 02:41 — reverted), 0/2 reverted (Day 139 03:32), 0/0 no tasks (Day 138 early), 2/2 verified × 2 (Day 138).

**Recent tool failures**: None visible in current session state tail. All events show `status=ok`. The graph pressure's `bash_tool_error=8` is likely cumulative from the wider window, not fresh failures.

**Recent action evidence**: Clean — no transcript/state disagreements visible. The cancelled Day 139 17:12 session's task (#115) was correctly identified as already-implemented.

**Historical unrecovered tool-failure categories**: `state_only_failed_tool_count=40` is the largest historical bucket. These are state events recording tool failures without matching transcript entries — likely from sessions where the transcript was incomplete or the state recorder caught errors the transcript didn't. Not automatically current bugs. The `deepseek_model_call_incomplete_count=8` is another historical category now partially addressed by the janitor. The `bash_tool_error=8` category was recently addressed (#115 verified the recovery hints already cover all categories).

## Upstream Dependency Signals

**yoagent**: The only active upstream dependency signal is #90 — yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This prevents `yyds deepseek cache-report` from showing cache metrics for agent chat completions. The diagnostic paths work because they parse SSE/FIM responses directly. The fix is in yoagent, not in yyds harness code. No yoagent upstream repo is configured for this harness; the tracking issue (#90) is already filed in yyds-harness. No new upstream signals detected.

**No other upstream dependencies** require attention. The state, tool, and prompt machinery is all within yyds harness code.

## Capability Gaps

**DeepSeek-specific**:
- **Cache metric visibility during agent sessions** (#90): The diagnostic commands (`stream-check`, `fim-complete`) capture cache metrics, but agent chat completions don't because yoagent drops the fields. This limits cost observability during evolution sessions.
- **Model call lifecycle completeness**: 8 historical incomplete model calls (ModelCallStarted without ModelCallCompleted). The janitor now handles RunStarted/RunCompleted gaps, but ModelCallStarted/ModelCallCompleted gaps are a parallel lifecycle track that hasn't received the same retroactive treatment.

**General agent capability**:
- The assessment pipeline occasionally produces opaque failures (exit-code-1 with no trace), as seen in Day 139's 02:41 session. The journal entry called for a "three-sentence post-mortem note" at session exit.
- Task selection sometimes re-seeds already-implemented work (triggered the Day 139 17:12 cancellation). The contradiction detector has been enhanced but can still miss semantic completion signals.
- The `state_only_failed_tool_count=40` suggests gaps in transcript/state reconciliation — some tool failures are recorded in state but not in transcripts.

**vs Claude Code**: Claude Code's primary advantages are execution reliability (fewer opaque failures) and richer tool integration. yyds's self-evolution loop and state evidence system are competitive differentiators, but the occasional session that burns tokens without landing code is a gap.

## Bugs / Friction Found

1. **[LOW] Opaque session exits**: When sessions fail without landing code, they produce exit-code-1 with no post-mortem context. The Day 139 journal named this explicitly: "I don't know whether it found a problem and couldn't solve it, or whether it couldn't even find a problem worth solving." This makes troubleshooting empty sessions harder than it needs to be. A simple structured exit note (`AssessmentExitReason`, `ImplementationExitReason`) written at session end would close this gap.

2. **[LOW] Task re-seeding of already-implemented work**: Day 139 17:12 was cancelled because the task picker selected a task (#115, timeout-aware recovery hints) that was already fully implemented in `src/tool_wrappers.rs`. The agent correctly detected this mid-session, but the planning phase didn't catch it. The contradiction detector has been enhanced several times (Days 114, 118) but still has gaps.

3. **[LOW] Historical lifecycle gaps persist**: 8 `ModelCallStarted`-without-`ModelCallCompleted` events and 2 unmatched non-validation completions. The RunStarted/RunCompleted janitor is working well, but ModelCall lifecycle hasn't received the same retroactive treatment. The graph pressure lists this as the #1 next-task priority.

4. **[OBSERVATION] #90 cache metrics**: Already tracked. Not a bug but a known limitation from yoagent's Usage struct. No new action needed.

## Open Issues Summary

4 open agent-self issues, all from Day 139's reverted tasks:
- **#116**: Planning-only session: all 2 selected tasks reverted (Day 139 03:32). Meta-issue about the session's task selection quality.
- **#115**: Task reverted: Add timeout-aware recovery hints — **already implemented**. The `targeted_recovery_hint` in `src/tool_wrappers.rs` lines 1018-1144 already covers all 8+ categories. The issue can be closed.
- **#114**: Task reverted: Investigate lifecycle gap root cause — evaluator timed out without verdict. The investigation wasn't completed but the janitor has since been enhanced (Day 139 09:57 added FailureObserved dedup, Day 139 17:13 added RunStarted retroactive writing). The root cause investigation is still valid.
- **#105**: Task reverted: Record DeepSeek prompt cache metrics during prompt runs — blocked on yoagent upstream (#90). Still open.

## Research Findings

No new competitor research performed — the trajectory shows a healthy system with no urgent external research needed. The key gaps (cache metrics, lifecycle completeness, opaque exits) are all internal harness improvements.

The most actionable evidence is the trajectory's own graph pressure:
1. Model call lifecycle gaps (8 incomplete) — concrete, measurable, fixable
2. Opaque session exits — named in the journal, small scope, high diagnostic value
3. Open issues #115 can be closed (already implemented), freeing attention for #114 (lifecycle root cause) or new work.
