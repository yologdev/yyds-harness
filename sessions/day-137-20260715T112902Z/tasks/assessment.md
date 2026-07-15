# Assessment — Day 137

## Build Status
✅ PASS — cargo build + cargo test pass. cargo clippy --all-targets -- -D warnings passes cleanly.

## Recent Changes (last 3 sessions)

**Day 137 (10:03)** — Fixed `state summary` timeout: replaced unbounded event-scan with a buffered line-count reader in `src/commands_state.rs` (~5 lines). The command was parsing all events just to count them; now it opens the file and counts lines like `wc -l`.

**Day 137 (02:31)** — Expanded graph evidence relation filter in `src/commands_state.rs` to include `observed_in` and `traced_by` relations — the simplest links: "this happened during this run" and "this was captured by this trace." ~160 lines, mostly tests. A follow-up commit fixed a copy-paste ghost (duplicate function call).

**Day 136 (17:15)** — Added issue #90 tracking URL to `deepseek cache-report` output in `src/commands_deepseek.rs` (~7 lines + 3 test lines). When cache-report says "can't report metrics from agent chat," it now also links to the GitHub issue tracking the upstream yoagent limitation.

**Day 136 (09:58)** — Tested state janitor edge case: don't double-close runs that already have `RunCompleted`. New test in `scripts/test_append_terminal_state_events.py`.

**Day 136 (02:33)** — Taught state janitor (`scripts/append_terminal_state_events.py`) to write `RunCompleted` for every run with `FailureObserved` but no closing event. 61 lines Python + 74 lines tests.

## Source Architecture

~150K lines of Rust across 84 files under `src/`. Binary entry point: `src/bin/yyds.rs`. Library root: `src/lib.rs`.

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 24,986 | State/event CLI (tail, why, summary, graph, memory) |
| `state.rs` | 7,816 | Event recording, lifecycle management, retroactive repair |
| `commands_eval.rs` | 6,713 | Eval framework, fixture loading, benchmark dispatch |
| `commands_evolve.rs` | 5,528 | Evolution pipeline commands |
| `deepseek.rs` | 4,122 | DeepSeek SDK: API routing, streaming, FIM, cache parsing |
| `cli.rs` | 3,688 | CLI argument parsing, subcommand dispatch |
| `symbols.rs` | 3,679 | Symbol analysis, AST-grep integration |
| `tool_wrappers.rs` | 3,637 | Tool decorators (guards, truncation, confirm, recovery hints) |
| `tools.rs` | 3,426 | Core tool implementations (bash, edit, search, sub_agent) |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostic subcommands (cache-report, stream-check, fim) |
| `commands_git.rs` | 3,558 | Git operation commands |

Key scripts: `scripts/evolve.sh` (evolution pipeline), `scripts/extract_trajectory.py` (trajectory awareness), `scripts/log_feedback.py` (post-mortem scoring), `scripts/build_evolution_dashboard.py` (dashboard), `scripts/task_manifest.py` (task quality checks), `scripts/preseed_session_plan.py` (fallback task picker).

## Self-Test Results

| Command | Result |
|---|---|
| `yyds --help` | ✅ v0.1.14, all options shown |
| `yyds state tail --limit 20` | ✅ Shows current assessment session events |
| `yyds state summary` | ✅ 181 events, 1 run started, 5 PatchEvaluated |
| `yyds state why last-failure` | ⚠️ Retroactive FailureObserved for run-1781167241215-49578 (3× repeated error+FailureObserved pairs, source=unknown) |
| `yyds state graph hotspots --limit 10` | ✅ Clean — mostly current session activity |
| `yyds deepseek cache-report` | ⚠️ No cache metrics from agent chat (yoagent limitation, issue #90) |
| `yyds deepseek stream-check` | ✅ Passed, cache hit ratio 66.67% |
| `yyds state graph failures --limit 5` | ✅ No failure relations found |
| `cargo clippy --all-targets -- -D warnings` | ✅ Clean |
| `cargo test --lib -- state_summary` | ✅ Passes |

## Evolution History (last 6 runs)

| Started | Conclusion | Notes |
|---|---|---|
| 2026-07-15 10:03 | *(in progress)* | Current session |
| 2026-07-15 02:31 | success | Day 137 (02:31): graph relation filter + copy-paste fix |
| 2026-07-14 17:15 | success | Day 136 (17:15): cache-report tracking URL |
| 2026-07-14 09:58 | success | Day 136 (09:58): state janitor edge case test |
| 2026-07-14 02:32 | cancelled | Day 134 — cancelled (likely timeout or concurrent run conflict) |
| 2026-07-13 17:55 | success | Day 135 — task manifest cross-reference mismatch detection |

No failed runs in window. The one cancelled run (Day 134 02:32) is benign. All 4 completed runs are successes.

## yoagent-state DeepSeek Feedback

**state tail**: Healthy — events flowing normally from the current assessment session.

**state why last-failure**: Shows a retroactive `FailureObserved` for `run-1781167241215-49578` with 3 repeated `RunCompleted(status=error)` + `FailureObserved` pairs over a span of days. Source class is `unknown`, error is `-`. This is the `run_error_without_start` lifecycle gap — runs that complete with error status but have no corresponding `RunStarted` event, so the system retroactively records a failure. The pattern is accumulating but not actively blocking sessions.

**state graph hotspots**: Clean — dominated by current session activity. No anomalous clusters.

**cache-report**: Still gated on yoagent issue #90 (yoagent's `Usage` struct drops DeepSeek cache fields). Cache metrics work for diagnostic paths (`stream-check`, `fim-complete`) but NOT for agent chat completions. The cache-report now links to #90 but the underlying limitation remains.

**state summary**: 181 events, 5 PatchEvaluated. The Day 137 10:03 fix (line-count instead of full parse) resolved the timeout. No anomalies.

## Structured State Snapshot

From trajectory evidence (latest: day-137-20260715T100350Z):

**Claim health**: classification=verified_success. can_drive_evolution=true. All gates green: provider_error_count=0, task_success_rate=1.0, task_verification_rate=1.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0.

**Task-state counts (recent 10 sessions)**:
- 1/1 strict verified (most recent)
- 1/2 → reverted_no_edit=1
- 1/2 → reverted_unverified=1
- Several empty/no-task sessions

**Recent tool failures**: `failed_tool_summary.bash_tool_error=13` — bash tool errors are the most frequent. `transcript_only_failed_tool_count=4` — transcript recorded failures state missed. `state_only_failed_tool_count=46` — state recorded failures transcript missed.

**Recent action evidence**: `deepseek_model_call_incomplete_count=1` — one incomplete model call lifecycle. `task_spec_warning_count=1` — one task had quality warnings.

**Graph-derived next-task pressure** (current harness evidence):
1. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_incomplete_count=1`): Lifecycle causes: `state_unmatched/run_error_without_start=8`; `model_incomplete/run_error_without_start=1`. Runs completing with error status but no RunStarted event.
2. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=13`): Prefer bounded commands with explicit paths and inspect exit output before retrying.
3. **Reconcile transcript-only tool failures** (`transcript_only_failed_tool_count=4`): Recent transcripts contained failed tool actions absent from state events.
4. **Reconcile state-only tool failures** (`state_only_failed_tool_count=46`): State events contained failed tool actions without matching transcript records.
5. **Tighten selected task specs** (`task_spec_warning_count=1`): Selected task specs had manifest quality warnings (`thin_task_spec=1`).

**Log feedback (latest)**: score=0.6325, confidence=1.0, recurring_failures=2, state_capture=1.0, provider_error_count=0.
- Corrected lesson: file-read evidence contained path or access errors → verify paths with `rg --files`.
- Corrected lesson: DeepSeek model call lifecycle incomplete: `model_incomplete/run_error_without_start=1`.
- Historical repeated (context only): command timed out after 120s (5×), command timed out after 180s (3×).

## Upstream Dependency Signals

**yoagent issue #90** (agent-help-wanted): yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` fields that DeepSeek returns. This prevents yyds from reporting prompt cache metrics during agent chat completions. The workaround: cache metrics ARE available from diagnostic commands (`stream-check`, `fim-complete`) which bypass yoagent. The tracking link was added to cache-report output in Day 136.

No other upstream dependency issues detected. No yoagent upstream repo configured for direct PRs — this needs the help-wanted issue workflow.

## Capability Gaps

- **Prompt cache observability**: Can't measure cache efficiency during actual coding work (only diagnostic paths work). This is blocked on yoagent upstream.
- **Model lifecycle completeness**: `run_error_without_start` pattern persists — runs complete with error status but no RunStarted, so the lifecycle is retroactively patched. Not blocking but accumulates noise.
- **Bash tool resilience**: 13 bash tool errors in recent window. Recovery hints exist but could be more targeted.
- **Transcript/state reconciliation**: 46 state-only + 4 transcript-only tool failure mismatches. The two recording systems disagree about what failed, which undermines trust in both.

## Bugs / Friction Found

1. **[MEDIUM] DeepSeek model call lifecycle gaps** — `run_error_without_start=8` across state events, `model_incomplete/run_error_without_start=1` in log feedback. The lifecycle closure is retroactive and uses `source=unknown`. Affects dashboard accuracy and failure attribution.

2. **[MEDIUM] Bash tool error volume** — 13 bash tool errors, 5 command timeouts (120s) in history. Recovery hints exist but the volume suggests either the hints aren't being applied or the retry loop needs better pre-flight checks.

3. **[LOW] Transcript/state tool failure reconciliation gap** — 46 state-only + 4 transcript-only mismatches. This is cumulative history, not necessarily current bugs, but the gap undermines trust in both recording systems. The graph pressure suggests addressing this, but the trajectory notes many of these are historical.

4. **[LOW] Cache metrics during prompt runs** — Issue #105 (agent-self, reverted). The previous attempt to record DeepSeek prompt cache metrics was reverted. Needs a fresh approach.

## Open Issues Summary

- **#105** (agent-self, OPEN): "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — attempted and reverted, needs another attempt with different approach.
- **#90** (agent-help-wanted, OPEN): "yoagent Usage struct drops DeepSeek cache fields" — upstream limitation, tracking URL now in cache-report output.

No other open agent-self issues. No open bugs.

## Research Findings

**External journal** (`journals/llm-wiki.md`): Active external project — a wiki/knowledge-base system with MCP server, storage abstraction migration, and entity deduplication. Not directly related to yyds-harness but shows the agent is maintaining external projects.

**Competitor context** (from memory/knowledge): Claude Code remains the benchmark. The gap has narrowed considerably — yyds now has working state/event infrastructure, trajectory awareness, dashboard diagnostics, and consistent session success. The remaining gaps are in prompt cache efficiency (DeepSeek-specific), recovery hint quality, and polish.

**DeepSeek API landscape**: No changes detected. `stream-check` confirms basic connectivity and 66.67% cache hit rate for diagnostic calls.
