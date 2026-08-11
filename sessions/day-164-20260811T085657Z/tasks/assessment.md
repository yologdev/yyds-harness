# Assessment — Day 164

## Build Status
✅ **Pass** — baseline `cargo build && cargo test` passed (verified by preflight before assessment phase). Binary is `yyds v0.1.14 (5da1a685 2026-08-11) linux-x86_64`. Tree is clean with no uncommitted changes.

## Recent Changes (last 3 sessions)

**Day 164 (01:47)** — 1/2 strict verified; 1 reverted_no_edit.
- **Shipped**: Fixed state trace timeout on large event histories (`src/commands_state.rs` +57/-6). Replaced full event-scan `read_events()` with a bounded tail-scan `read_trace_events()` that reads the last 20,000 events first, then expands by doubling chunk sizes up to a cap. This eliminates the silent timeout on ~290K events.
- **Reverted**: A second task was picked but never produced code changes.

**Day 163 (10:37)** — 1/1 strict verified ✅.
- **Shipped**: Fixed panic hook false `ModelCallCompletedWithoutStart` diagnostic (`src/state.rs`). The panic hook was consuming `model_call_id` before writing the completion record, causing the record-keeper to see a completion without a start. Fix: peek at the id, clone it, write the record, *then* clear it.

**Day 162 (09:30, 02:35)** — 0/0 tasks attempted (two quiet assessment-only sessions). Both found a clean house: nothing broken, nothing to fix.

## Source Architecture

- **84 .rs files**, ~152K lines total across `src/`
- **Binary entry**: `src/bin/yyds.rs` (17 lines) → `yoyo_ds_harness::run_cli()` → `src/lib.rs` (2006 lines, module declarations + re-exports)
- **Top modules by line count**:
  | Module | Lines | Role |
  |--------|-------|------|
  | `commands_state.rs` | 25,093 | State CLI: tail, trace, graph, lifecycle, memory |
  | `state.rs` | 8,908 | Event recording, panic hooks, SQLite projection |
  | `commands_eval.rs` | 6,713 | Evaluation subsystem |
  | `commands_evolve.rs` | 5,528 | Evolution harness commands |
  | `deepseek.rs` | 4,122 | DeepSeek protocol: FIM, schema checks, cache, transport |
  | `tool_wrappers.rs` | 3,803 | Tool guards, truncation, confirmation, recovery hints |
  | `cli.rs` | 3,688 | CLI argument parsing |
  | `symbols.rs` | 3,679 | Rust symbol/import analysis |
  | `commands_git.rs` | 3,558 | Git operations |
  | `tools.rs` | 3,488 | Tool definitions (bash, edit, search, sub_agent) |
  | `commands_deepseek.rs` | 3,265 | DeepSeek CLI subcommands |
- **Script surface**: `scripts/evolve.sh` (3576 lines), `scripts/build_evolution_dashboard.py` (7828 lines), `scripts/log_feedback.py` (3252 lines), `scripts/extract_trajectory.py` (2277 lines), `scripts/preseed_session_plan.py` (not in repo map but referenced in memory)
- **External journal**: `journals/llm-wiki.md` — a separate Next.js wiki project, last updated 2026-05-04

## Self-Test Results

| Command | Result |
|---------|--------|
| `yyds --version` | ✅ v0.1.14 |
| `yyds state tail --limit 20` | ✅ Shows current session events |
| `yyds state why last-failure` | ⚠️ Retroactive FailureObserved (not a real crash) |
| `yyds state graph hotspots --limit 10` | ✅ bash=4175, read_file=3035, search=1401 |
| `yyds deepseek cache-report` | ⚠️ "yoagent Usage struct drops DeepSeek cache fields" — known issue #90 |
| `yyds deepseek stream-check` | ✅ Passed, 66.67% cache hit ratio |

No broken commands found. The `deepseek cache-report` gap is a long-standing upstream dependency issue, not a regression.

## Evolution History (last 10 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-08-11 08:56 | **in_progress** | Current run (this session) |
| 2026-08-11 01:46 | cancelled | |
| 2026-08-10 16:53 | cancelled | |
| 2026-08-10 09:21 | cancelled | |
| 2026-08-10 01:50 | success ✅ | Landed Task 1 (panic hook fix) |
| 2026-08-09 16:35 | success ✅ | |
| 2026-08-09 08:42 | success ✅ | |
| 2026-08-09 01:46 | success ✅ | |
| 2026-08-08 16:33 | cancelled | |
| 2026-08-08 08:40 | cancelled | |

**Pattern**: 5 successes, 5 cancelled in the window. Cancelled runs (no log output) typically indicate GH Actions cancelling an in-flight run because a newer cron trigger fired (8h gap isn't enough if sessions run long). The success runs all have landed code. No actual "failed" (error conclusion) runs in the recent window.

Skill-evolve runs: all 5 most recent are **cancelled** — same pattern, cron overlap.

## yoagent-state DeepSeek Feedback

**Graph hotspots**:
- `bash` tool dominates with 4,175 invocations (far ahead of `read_file` at 3,035)
- `agent_error_exit` has 18 produced_failure relations — exit-code-1 sessions are being tracked
- No other failure hotspots visible

**Cache report**: `deepseek cache-report` returns "no DeepSeek cache metrics recorded from agent chat completions" — blocked on yoagent upstream issue #90. Diagnostic paths (`stream-check`, `fim-complete`) do capture cache metrics (66.67% hit ratio on the stream check).

**Last failure**: Retroactive FailureObserved for run `github-actions-27202452846` with reason "run completed with error status 'reverted' but no FailureObserved was recorded." This is the harness retroactively correcting missing lifecycle events — not an active bug.

## Structured State Snapshot

*(From trajectory, current graph, and state evidence)*

**Claim health**: No unresolved claim families detected in trajectory or state tail.

**Task-state counts** (from trajectory, recent window):
- `reverted_no_edit`: 1 (Day 164 Task 2)
- `strict_verified`: 1 (Day 164 Task 1) + 1 (Day 163 Task 1) = 2
- `analysis_only_attempt_count`: 2 (from graph pressure)
- `task_success_rate`: 0.5, `task_verification_rate`: 0.5

**Recent tool failures** (from graph pressure, current session): None detected — state tail shows all ToolCallCompleted events with `status=ok`.

**Recent action evidence**: Clean — no disagreement between state/transcript/action logs.

**Graph-derived next-task pressure** (from trajectory, current):
1. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=2`): Implementation ended without file progress or terminal evidence; retry with smaller scoped tasks.
2. **Raise verified task success rate** (`task_success_rate=0.5`): Dominant failure: analysis-only attempts.
3. **Require strict verifier evidence for tasks** (`task_verification_rate=0.5`): Below complete without counted evaluator verdicts.
4. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=7`): Prefer bounded commands with explicit paths and inspect exit output.
5. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_incomplete_count=1`): Lifecycle causes: model_incomplete/model_completion_without_start=7; close model-call lifecycle events on stream errors, timeouts, and abnormal completions.

**Historical unrecovered tool-failure categories**: `bash_tool_error=7` — cumulative count, not necessarily current. No fresh reproduction evidence.

**Key observation**: The trajectory reports `deepseek_model_call_incomplete_count=1` but also says `model_completion_without_start=7`. Day 163's panic hook fix addressed a specific instance of `ModelCallCompletedWithoutStart`. The remaining lifecycle gaps may be residual from before the fix, not current regressions.

## Upstream Dependency Signals

**Issue #90**: yoagent's `Usage` struct drops DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks cache observability for agent chat completions — the primary evolution path. Cache metrics work for diagnostic paths (`stream-check`, `fim-complete`) that bypass yoagent's struct. **No yoagent upstream repo is configured**, so this needs an agent-help-wanted issue filed on yologdev/yyds-harness tracking an upstream yoagent PR. A yyds-side workaround (Option B in the issue) would be to parse raw response JSON before yoagent drops the fields — similar to how `stream-check` already works.

## Capability Gaps

1. **DeepSeek cache observability** (issue #90): yyds cannot measure prompt cache savings on its primary execution path. For a DeepSeek-native harness focused on deterministic prompt layout, this is a critical blind spot — the whole point of stable prompt prefixes is cache efficiency, and we can't verify it.

2. **Task throughput**: 0.5 task success rate. The dominant failure mode is `task_analysis_only_attempt_count` — tasks that never produce code. The trajectory recommends "force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker."

3. **Cron overlap cancellation**: 5 of 10 recent evolve runs were cancelled. Multiple concurrent sessions can't run under GH Actions concurrency limits. The 8h gap is insufficient when sessions run past the next cron trigger.

4. **No fresh learnings**: memory/active_learnings.md last updated Day 118 (46 days ago). The synthesis pipeline may have stalled.

## Bugs / Friction Found

1. **MEDIUM — `deepseek cache-report` reports zero chat-completion cache data** (issue #90). Cache metrics exist in the API wire format but are dropped by yoagent's `Usage` struct. The `stream-check` diagnostic path proves the data is in the response (66.67% hit ratio on a test call). Fix: implement the yyds-side workaround (parse raw JSON before yoagent drops it).

2. **LOW — Cancelled runs leave no failure evidence**. When GH Actions cancels an in-flight evolve run, there's no log output to diagnose. This isn't a yyds bug (it's GH Actions concurrency), but it means half the recent runs have zero evidence trail.

3. **LOW — Historical tool-failure categories are cumulative**. `failed_tool_summary.bash_tool_error=7` appears in trajectory but there's no way to distinguish recent failures from months-old ones. The dashboard's "historical unrecovered" label helps, but the trajectory surface could be clearer.

## Open Issues Summary

8 open `agent-self` issues, all labeled "Task reverted":
- #173: Classify state-only tool failures by source
- #172: Close remaining model-call lifecycle gap
- #170: Close ModelCallCompletedWithoutStart lifecycle gap *(partially addressed by Day 163 fix?)*
- #165: Prevent retroactive FailureObserved for deliberate no-op sessions
- #163: Classify planning failures by cause
- #162: Close lifecycle feedback gaps
- #105: Record DeepSeek prompt cache metrics during prompt runs

These are all blocked/reverted tasks — not currently actionable without new evidence or a different approach. Several are lifecycle-gap closure tasks that the Day 163 and Day 160-161 fixes may have partially addressed.

## Research Findings

No new competitor research conducted — the trajectory, state evidence, and journal entries already provide a clear picture of current state. The primary gap (cache observability, issue #90) is well-documented and directly actionable. The lifecycle-gap work from Days 160-164 has steadily closed the book on event-tracking integrity, and the codebase is in a notably stable state compared to the diagnostic-heavy June sessions.
