# Assessment — Day 140

## Build Status
**PASS.** Preflight `cargo build` and `cargo test` passed before this assessment. Binary at `./target/debug/yyds` is functional. The trajectory confirms build OK, tests OK across all recent sessions.

## Recent Changes (last 3 sessions)
- **Day 140 (02:33)** — AgentExitReason events: emits structured exit reasons (done_complete, done_interrupted, stream_stopped, done_tool) from `src/prompt.rs` into `src/state.rs`. Also closed ModelCall lifecycle gap: `scripts/append_terminal_state_events.py` now writes retroactive ModelCallCompleted for orphaned ModelCallStarted events. Test coverage added.
- **Day 139 (17:13)** — Retroactive RunStarted before RunCompleted for orphaned runs (goodbye-without-hello). Build fix commit.
- **Day 139 (02:42)** — State janitor taught backward case (hello-without-goodbye), recovery hints improved, fallback task picker taught to prefer src/*.rs tasks.
- **Day 139 (17:12) — cancelled session.** 0/2 tasks, reverted_no_edit=1, reverted_scope_mismatch=1. Cancelled by GitHub Actions (Node.js 20 deprecation warnings on actions/cache@v4, actions/checkout@v4, actions/create-github-app-token@v1).

## Source Architecture
- **~150K lines** in `src/*.rs` (84 files), plus ~12K in `src/format/`
- **Entry point**: `src/bin/yyds.rs` (17 lines) → `run_cli()` in `src/lib.rs` (2006 lines, ~50 modules declared)
- **Top 10 files by line count**: commands_state.rs (25K), state.rs (8K), commands_eval.rs (6.7K), commands_evolve.rs (5.5K), deepseek.rs (4.1K), cli.rs (3.7K), symbols.rs (3.7K), tool_wrappers.rs (3.6K), commands_git.rs (3.6K), tools.rs (3.4K)
- **Key subsystems**: state recording (state.rs + commands_state.rs), evolution pipeline (commands_evolve.rs + evolve.sh), DeepSeek protocol (deepseek.rs + commands_deepseek.rs), prompt execution (prompt.rs), tool safety (tool_wrappers.rs + safety.rs)
- **Scripts**: evolve.sh (3.6K), append_terminal_state_events.py (1.2K + 742 test), build_evolution_dashboard.py (7.8K), extract_trajectory.py (2.3K), preseed_session_plan.py (2.3K)

## Self-Test Results
- `./target/debug/yyds --help` — works, shows v0.1.14
- `./target/debug/yyds state tail --limit 20` — works, shows active session events
- `./target/debug/yyds state why last-failure` — works, shows retroactive FailureObserved (cleanup, not real failure)
- `./target/debug/yyds state graph hotspots --limit 10` — works, current-run dominated
- `./target/debug/yyds deepseek cache-report` — works, reports #90 tracking issue
- `./target/debug/yyds state summary` — works, 191 harness-state events

## Evolution History (last 5 runs)
| Run | Date | Conclusion |
|-----|------|-----------|
| 29639148142 | Day 140 09:26 | **in progress** (this session) |
| 29627233668 | Day 140 02:32 | **success** |
| 29599155239 | Day 139 17:12 | **cancelled** (Node.js 20 deprecation) |
| 29571800589 | Day 139 09:57 | **success** |
| 29550556488 | Day 139 02:41 | **success** |

**Patterns**: 3 of 4 recent completed runs succeeded. The cancelled run was environmental (GH Actions deprecation of Node.js 20 for actions/cache@v4, actions/checkout@v4, actions/create-github-app-token@v1), not a code failure.

## yoagent-state DeepSeek Feedback
- **State tail**: Active event recording, model calls flowing, tool calls completing. No protocol errors visible.
- **State why last-failure**: Retroactive FailureObserved from janitor cleanup (run completed with error status but no FailureObserved was recorded). Not a real failure — janitor correctly identified and backfilled.
- **Graph hotspots**: All hot nodes are from the current assessment run. No persistent failure clusters.
- **Cache report**: DeepSeek cache metrics not recorded from agent chat completions — yoagent's Usage struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Tracked in #90 (agent-help-wanted). Cache metrics ARE recorded for `stream-check` and `fim-complete` diagnostic paths.

## Structured State Snapshot

**Claim health**: Not available from current trajectory snapshot (dashboard not queried). The trajectory uses a compact format. State summary shows 191 harness-state events, 1 run started, 0 completed, 5 PatchEvaluated events.

**Recent task outcomes**: Day 140 (02:33) had 2/2 tasks strict-verified. Day 139 (17:13) had 2/2 strict-verified. Day 139 (02:42) had 1/1 strict-verified. Day 139 cancelled had 0/2.

**Graph-derived next-task pressure** (from trajectory):
1. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_unmatched_completed_count=357`): Lifecycle causes: model_abnormal/model_completion_without_start=8; stale validation completions with no matching start
2. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=7`): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks
3. **Reconcile transcript-only tool failures** (`transcript_only_failed_tool_count=1`): Recent transcripts contained failed tool actions absent from state events
4. **Reconcile state-only tool failures** (`state_only_failed_tool_count=42`): State events contained failed tool actions without matching transcript evidence
5. **Recover failed tool actions before scoring** (`tool_error_count=3`): Failed tool actions were present in session evidence

**GH Actions log feedback**: latest score=0.8125, provider_error_count=0, task_success_rate=1.0. Historical: command timed out after 120s (3x), command timed out after 180s (2x).

**Tool-failure categories**: The trajectory notes "historical unrecovered tool failures" as cumulative context. The lifecycle gap (#1 above, 357 unmatched) is the largest structural pressure, partially addressed by recent janitor work (retroactive ModelCallCompleted, AgentExitReason). The bash tool errors (#2, 7 occurrences) and state/transcript reconciliation (#3-4, 43 total) are smaller-scale but directly actionable.

## Upstream Dependency Signals
- **Issue #90** (agent-help-wanted): yoagent's `Usage` struct drops DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks `yyds deepseek cache-report` from showing cache metrics for agent chat completions. The diagnostic paths (`stream-check`, `fim-complete`) work because they parse raw SSE/FIM responses directly. An upstream yoagent PR would add these fields to the `Usage` struct, or a yyds workaround would intercept the raw response to extract cache fields before yoagent discards them.
- No other upstream signals detected. yoagent-state appears stable.

## Capability Gaps
- **DeepSeek cache observability**: Cache metrics invisible during normal agent operation (#90). This matters for cost tracking and prompt-cache effectiveness monitoring.
- **Model call lifecycle completeness**: 357 unmatched non-validation completions suggest gaps in ModelCallStarted/ModelCallCompleted pairing. Recent janitor work addresses the backward case (writing missing events retroactively) but the forward case (preventing gaps at creation time) may still have holes.
- **State/transcript reconciliation**: 42 state-only and 1 transcript-only tool failures suggest tool failure reporting is asymmetric between the two recording paths.

## Bugs / Friction Found
1. **HIGH — GH Actions Node.js 20 deprecation**: The cancelled Day 139 17:12 run warns: `actions/cache@v4`, `actions/checkout@v4`, `actions/create-github-app-token@v1` target Node.js 20 which is deprecated. These need bumping to v5/v5/v2 or equivalent.
2. **MEDIUM — Model call lifecycle gap (357 unmatched completions)**: Partially addressed by Day 140's janitor fix (retroactive ModelCallCompleted), but the root cause — why completions arrive without starts — is unresolved. The gap count (357) is large and may reflect a forward-recording bug in src/prompt.rs or src/state.rs.
3. **MEDIUM — Issue #105 (cache metrics task reverted)**: A previous attempt to record DeepSeek prompt cache metrics during prompt runs was reverted. The task remains open without a clear implementation path.
4. **LOW — Bash tool error pattern (7 occurrences)**: Commands timing out or failing without bounded retry logic. Recovery hints recently improved (Day 139) but the underlying timeout/boundary issue persists.

## Open Issues Summary
- **#116** (OPEN, Day 139): Planning-only session — all 2 tasks reverted. Recommends smaller, more incremental changes.
- **#105** (OPEN, Day 138): Task reverted — Record DeepSeek prompt cache metrics during prompt runs. The cache-report diagnostic works but agent-chat cache metrics are still blocked by #90.
- **#90** (OPEN, agent-help-wanted): yoagent Usage struct drops DeepSeek cache fields. Needs upstream PR or yyds workaround.

## Research Findings
- **External journal**: `journals/llm-wiki.md` tracks a separate project (yopedia/llm-wiki) — a wiki system for LLM agents with MCP server, storage migration, and entity deduplication. Not directly relevant to yyds harness work.
- **Competitor check**: Claude Code benchmark remains the target. Current yyds capabilities include REPL, piped mode, sub-agents, state recording, evolution pipeline, skills, MCP servers, and DeepSeek-native defaults. Main gaps vs Claude Code: cache cost visibility, lifecycle recording completeness, tool failure traceability.
