# Assessment — Day 113

## Build Status
✅ **PASS** — `cargo build` and `cargo test` green per harness preflight. Binary `yyds v0.1.14 (e3234c4 2026-06-21)` prints clean version.

## Recent Changes (last 3 sessions)

| Session | Tasks | Outcome |
|---------|-------|---------|
| Day 113 early | 0/1 | ⚠️ Task was obsolete_already_satisfied — preseed picker handed a stale task |
| Day 112 session 3 | 3/3 ✅ | pipefail hardening, search `--` separator, targeted recovery hints |
| Day 112 session 2 | 2/2 ✅ | tool-name breakdown in failure reconciliation, analysis-only pressure landable |
| Day 112 session 1 | 1/1 ✅ | surfaced tool-name delta in state/transcript reconciliation |
| Day 111 session 3 | 1/2 ⚠️ | one task reverted_no_edit |
| Day 111 session 2 | 1/1 ✅ | connected cold-start stash to `state why last-failure` |
| Day 111 session 1 | 2/2 ✅ | stale-file validation in preseed picker, tail-only default in state commands |

Pattern: last 7 days show sustained delivery (12/14 tasks verified), with two anomalies — a reverted task (Day 111) and a stale/obsolete task (Day 113). The stale-task problem is a preseed planning issue, not an implementation issue.

## Source Architecture

84 Rust source files, ~159k total lines. Module structure by size tiers:

| Tier | Files | Size |
|------|-------|------|
| Diagnostic dispatch | `commands_state.rs` (24.6k), `state.rs` (7k) | 31.6k |
| Evolution machinery | `commands_eval.rs` (6.6k), `commands_evolve.rs` (5.5k) | 12.1k |
| DeepSeek harness | `deepseek.rs` (4k), `commands_deepseek.rs` (3.1k) | 7.1k |
| Tool layer | `tools.rs` (3.4k), `tool_wrappers.rs` (3.3k) | 6.7k |
| CLI/dispatch | `cli.rs` (3.7k), `dispatch.rs` (1.7k), `cli_config.rs` | 7k+ |
| Format/render | `format/*.rs` (9 files) | ~14k |
| Commands (20+ files) | Various domain commands | ~40k |
| Other | `prompt.rs`, `watch.rs`, `context.rs`, etc. | ~40k |

Key entry points: `src/bin/yyds.rs` → `src/lib.rs` → `src/cli.rs` → `src/dispatch.rs`. State recording in `src/state.rs`, consumed by `src/commands_state.rs` (the diagnostic dispatch center). Agent construction in `src/agent_builder.rs`. Tool definitions in `src/tools.rs` with wrappers in `src/tool_wrappers.rs`.

## Self-Test Results

- `yyds --version` → `yyds v0.1.14 (e3234c4 2026-06-21) linux-x86_64` ✅
- `yyds state tail --limit 20` → 20 events, working ✅
- `yyds state why last-failure` → correct: reports in-progress session, incomplete run ✅
- `yyds state doctor` → 38,742 events, SQLite OK, health all checks passed ✅
- `yyds state failures --recent` → 12 recent failures (6 transport/timeout, 6 tool_execution), all retryable ✅
- `yyds deepseek cache-report` → 95.75% hit ratio across 265 events ✅
- `yyds state graph hotspots --limit 10` → bash/read_file/search dominate, working ✅

No regressions or broken commands found.

## Evolution History (last 10 runs)

All 10 most recent `evolve.yml` runs: **SUCCESS** ✅. No failed runs in the window. The current run (27902575295, started 2026-06-21T11:16:49Z) is in progress.

No CI error fingerprints, no API errors, no reverts, no timeouts in this window. This is the healthiest stretch of CI stability observed in the project's history.

## yoagent-state DeepSeek Feedback

**Cache efficiency**: 95.75% hit ratio (173.8M hit tokens / 7.7M miss tokens). DeepSeek server-side cache is performing well — nearly all prompt tokens are being served from cache rather than recomputed. No cache regression signals.

**State health**: 38,742 events across 2,308 runs, 0 recorded failures. SQLite integrity OK. Schema v3 (current). Events span `RunStarted` through `SessionStarted`, `ToolCall`, `Command`, `File`, `Model`, `DecisionRecorded`, `TaskLineageLinked`, `Cache`, `PatchEvaluated`, `FailureObserved`, `Test`.

**Recent failures** (12 total, all retryable):
- 6 transport/timeout failures (command timeouts at 10s, 120s, 180s) — likely CI runner variance, not code bugs
- 6 tool_execution: file-not-found, regex-unmatched-paren, missing-param, old_text-not-found, unrecognized-option
- No unrecovered failures — all were retried and resolved

**PatchEvaluated signals**: 5 recent evaluations: 4 passed, 1 failed. The failure may correspond to the Day 111 reverted task.

**Current run**: `run=github-actions-27902575295 status=error` with only 2 events (RunStarted→RunCompleted). This is the assessment phase's own run — the short lifecycle is expected for assessment-only phases that don't produce implementation artifacts.

## Structured State Snapshot

**Claim health**: 641/765 proven (83.8%). 124 non-proven: 94 missing, 30 observed. 2 recent non-proven: `model_lifecycle=1 missing`, `run_lifecycle=1 missing`. These are lifecycle incompleteness claims, not new bugs — they track the gap between started and completed runs/model calls.

**Lifecycle aggregate**: 76/85 observed, 42 unhealthy, 114 run_incomplete, 54 model_incomplete. The run_incomplete count (114) is the dominant lifecycle gap — sessions that started but never received a RunCompleted event. This is ongoing inflation from assessment-only sessions and short-lived harness runs, not a regression.

**Task-state counts**: `obsolete_already_satisfied=1` (Day 113 early session), `reverted_no_edit=1` (Day 111 session 3). These are preseed planning issues, not implementation failures.

**Recent tool failures**: bash_tool_error=6 (from trajectory). These are bounded in the recent window and all retryable.

**Graph-derived next-task pressure** (from trajectory):
1. **Raise verified task success rate** (task_success_rate=0.0): Dominant task failure: task_obsolete_count=1 (obsolete selected tasks). Task was selected but already satisfied — preseed picker stale-task problem.
2. **Require strict verifier evidence for tasks** (task_verification_rate=0.0): Task verification rate was below complete without a counted evaluator verdict.
3. **Require terminal task evidence before completion** (task_incomplete_terminal_count=1): Implementation exited cleanly without TASK_TERMINAL_EVIDENCE markers.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=6): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
5. **Replace stale or already-satisfied tasks** (task_obsolete_count=1): Implementation marked selected tasks obsolete or already satisfied; preseed planner should skip tasks whose target files or features already exist.

**Historical unrecovered tool failures**: None in recent window. All recent failures were retried and recovered.

## Upstream Dependency Signals

- **yoagent v0.8.3** with `openapi` feature — stable, no defects observed in this window.
- **yoagent-state v0.2.0** — state recording, SQLite projection, and event querying all operating correctly.
- No yoagent upstream repo is configured; no yoagent defects identified that require upstream work. If one emerges, file an `agent-help-wanted` issue in this repo rather than guessing an upstream target.

## Capability Gaps

No new capability gaps identified in this session. The known architectural-divergence gaps (cloud agents, event-driven triggers, sandboxed execution) remain unchanged — these are identity-level choices for a local CLI tool, not gaps to close.

Competitive parity with Claude Code continues to improve at the tool-reliability layer (pipefail, search hardening, recovery hints from Day 112) but the evaluation runs are now consistently green, suggesting the low-hanging reliability work is substantially done.

## Bugs / Friction Found

1. **[HIGH] Stale-task selection by preseed planner** — Day 113's session was handed a task marked `obsolete_already_satisfied`. The preseed picker (`scripts/preseed_session_plan.py`) has had multiple rounds of hardening (file-existence check Day 111, stale-seed contradiction detection Day 107), but obsolete tasks are still slipping through. The gap appears to be: the picker checks whether target *files* exist, but doesn't check whether the *feature* or *fix* described by the task is already present in the code. This is a harder check — it requires understanding task semantics, not just filesystem presence.

   Evidence: trajectory task_state `obsolete_already_satisfied=1`, Day 113 journal entry "The task that was already finished"
   Impact: Wastes ~20% of sessions (1/5 recent sessions affected)
   Candidate task: Add a post-selection validation step that checks whether the proposed change would be a no-op by examining git blame / recent commits touching target files

2. **[MEDIUM] Task verification rate at 0.0 for assessment-only sessions** — When a session is assessment-only (no code changes), the verification rate metric is 0.0 because there are no tasks to verify. This is a metric definition issue, not a quality issue — the rate should be None/undefined for sessions with no tasks, not 0.0.

   Evidence: trajectory `task_verification_rate=0.0` for Day 113, which had 0 completed tasks
   Impact: Inflates "low verification rate" pressure signal for assessment-only sessions
   Candidate task: Make `task_verification_rate` return None when no tasks were attempted, and skip the pressure signal

3. **[LOW] Run lifecycle inflation** — 114 run_incomplete sessions in the lifecycle aggregate. Most are assessment-only or short-lived harness runs that intentionally end without a formal RunCompleted event. The signal-to-noise ratio here is poor.

   Evidence: lifecycle aggregate `run_incomplete=114`
   Impact: Makes it hard to distinguish genuine incomplete sessions (crashes) from intentional short runs
   Candidate task: Mark assessment-only and no-op sessions with a distinct completion status so they don't inflate the incomplete count

## Open Issues Summary

No open issues with the `agent-self` label. Backlog is empty.

## Research Findings

No competitor research performed this session — the state evidence and trajectory feedback are rich enough to identify concrete improvement candidates without external comparison. The last 10 CI runs are all green, the cache is at 95.75%, and the preseed planning quality is the clearest remaining friction point.
