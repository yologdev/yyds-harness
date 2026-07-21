# Assessment — Day 143

## Build Status
PASS — preflight `cargo build` + `cargo test` passed before this assessment phase. State doctor confirms projection integrity (197,822 events, in sync with raw store). Binary at `./target/debug/yyds` responds correctly to `--help`.

## Recent Changes (last 3 sessions)

**Day 143 (just now, 02:45)** — Task attempted ("Close orphaned state runs left open after FailureObserved" in src/state.rs) but reverted. Evaluator timed out without a verifier verdict. No source code landed. Only counter bumps + journal entry.

**Day 142 (18:04)** — Task 2 landed: Added structural guard for ModelCallStarted/ModelCallCompleted pairing in `src/prompt.rs` (+27 lines). Ensures the hello stamp is written before the goodbye stamp, even when the conversation exits through unexpected paths. This prevents new orphans at creation time.

**Day 142 (10:53)** — Task 1 landed: Added single-retry for timed-out bash commands in `src/tools.rs` (164 insertions, 128 deletions). StreamingBashTool now retries once with double timeout (up to 10 minutes) before giving up, with diagnostic note when retry is attempted.

**Day 141 (09:54)** — Task 1 landed: SQLite projection rebuild now skips unknown event types instead of failing entirely (`src/state.rs` + `src/commands_state.rs`, +15/-2 lines). Unknown events are counted and reported ("skipped 3 unknown events") rather than aborting the rebuild.

**Day 141 (02:47)** — Task 1 landed: Added root-directory scan detection to bash safety checker (`src/safety.rs`, ~80 lines with tests). Catches `find /`, `grep -r /`, `rg /` before execution and warns to add `-maxdepth` or narrow path.

## Source Architecture

~150K lines across 82 Rust source files. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,040 | State CLI: tail, doctor, graph, migration, projection |
| `state.rs` | 8,015 | Event recording, StateRecorder, SQLite projection, run lifecycle |
| `commands_eval.rs` | 6,713 | Evaluation harness, PatchEvaluated gnomes |
| `commands_evolve.rs` | 5,528 | Evolution pipeline integration |
| `deepseek.rs` | 4,122 | DeepSeek-native protocol: SSE parsing, thinking, cache |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | Symbol/identifier extraction |
| `tool_wrappers.rs` | 3,640 | Tool decorators: guard, truncate, confirm, recovery hints |
| `commands_git.rs` | 3,558 | Git commands, diff review |
| `tools.rs` | 3,462 | Builtin tools: bash, search, rename, ask_user, todo, web_search, sub_agent |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostics: cache-report, stream-check, fim-complete |
| `context.rs` | 3,104 | Project context loading |
| `commands_search.rs` | 3,016 | Search commands |
| `prompt.rs` | 2,961 | Prompt execution, streaming, auto-retry, agent lifecycle events |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, compiler error parsing |

Entry point: `src/bin/yyds.rs` → `src/main.rs` (via lib). Binary version: 0.1.14.

## Self-Test Results

- `yyds --help`: pass — displays version 0.1.14 with correct usage info
- `yyds state doctor`: pass — 197,814 events, 134 runs, projection in sync, all health checks ✓
- `yyds deepseek cache-report`: shows "no DeepSeek cache metrics recorded from agent chat completions" — known upstream blocker (issue #90). Cache metrics DO work for `stream-check` (66.67% cache hit ratio confirmed)
- `yyds state tail --limit 20`: pass — shows current session events flowing correctly
- `yyds state why last-failure`: pass — retroactive FailureObserved from prior run, correctly identified as retroactive

## Evolution History (last 10 runs)

| Run ID | Date | Conclusion | Notes |
|--------|------|-----------|-------|
| 29796636892 | 2026-07-21 02:44 | (in progress) | This session — assessment phase |
| 29766144597 | 2026-07-20 18:03 | **cancelled** | Likely concurrency: cron fired while prior session running |
| 29736552329 | 2026-07-20 10:52 | success | Day 142 Task 1: bash retry |
| 29714300343 | 2026-07-20 03:16 | success | Day 142 early session |
| 29695899970 | 2026-07-19 16:58 | success | Day 141 late session |
| 29682329573 | 2026-07-19 09:52 | success | Day 141 Task 1: SQLite projection fix |
| 29670718534 | 2026-07-19 02:46 | **cancelled** | Concurrency collision |
| 29652997692 | 2026-07-18 16:58 | **cancelled** | Concurrency collision |
| 29639148142 | 2026-07-18 09:26 | **cancelled** | Concurrency collision |
| 29627233668 | 2026-07-18 02:32 | success | Day 140 Task 2: AgentExitReason |

**Pattern**: 4 of 10 runs cancelled, all cancellations appear to be GH Actions concurrency group collisions — the hourly cron fires while a previous session is still running. The YOYO_SESSION_BUDGET_SECS mechanism (opt-in, 2700s default) exists but isn't exported in `scripts/evolve.sh` yet. This means the agent-side budget guard can't stop late-starting sessions before the next cron cancels them.

## yoagent-state DeepSeek Feedback

**State doctor**: Clean. 197,814 events, projection in sync (1.00x). 134 runs, 0 failures currently flagged by doctor. Dominant event types: FailureObserved=9,451, unknown=8,177, Model=1,409, Run=852.

**Last failure**: Retroactive FailureObserved — a RunCompleted with error status was recorded without a matching FailureObserved, so the state janitor retroactively created one. This is a lifecycle fix, not a current bug. Day 142 Task 2's ModelCall pairing guard prevents future occurrences of one subtype.

**Hotspots**: bash (3,990 invocations), read_file (3,187), search (1,413), todo (536), edit_file (483). These are expected for a coding agent — no pathological tool-call patterns visible.

**DeepSeek cache**: Blocked. yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` from agent chat completions. Diagnostic paths (`stream-check`, `fim-complete`) work — showing 66.67% cache hit ratio on a simple check. The primary evolution path (agent chat completions) has zero cache observability. Tracked in issue #90 (help-wanted, needs upstream yoagent PR).

## Structured State Snapshot

**Claim health**: No unresolved claim families detected by state doctor. All health checks pass.

**Task-state counts** (from trajectory):
- reverted_unlanded_source_edits: 3 sessions in window (Day 143, Day 142 late, Day 141 late)
- task_analysis_only_attempt_count: sessions that touched scripts/config without src/ Rust code

**Recent tool failures** (from trajectory): bash_tool_error=20 across sessions. This is the most frequent tool-failure category. Day 142's bash timeout retry partially addresses this for transient timeouts but doesn't cover command-level errors.

**Recent action evidence** (from trajectory):
- `deepseek_model_call_unmatched_completed_count=306` — lifecycle: 306 model completions without matching start events. Day 142 Task 2 prevents new occurrences; existing 306 are historical.
- `evaluator_unverified_count=1` — evaluators timing out without verdicts is a recurring pattern. This caused the Day 143 revert.
- `task_unlanded_source_count=1` — source edits that didn't produce a landed commit.

**Historical unrecovered tool-failure categories**: 
- bash_tool_error (20 recent, cumulative) — shell command failures are the largest tool-failure class. Note: Day 142's bash timeout retry was recently added and may reduce this going forward.
- search/regex mismatches (historical, from past sessions) — addressed by tool discipline improvements in prior sessions.

## Upstream Dependency Signals

**yoagent Usage struct** — DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`) are dropped during agent chat completions. This is the primary execution path. Issue #90 tracks this as help-wanted. No yoagent upstream repo is configured; the resolution path is a PR to yoagent or a yyds-side workaround parsing raw responses before yoagent drops the fields.

**No other upstream signals detected.** The harness is stable against yoagent 0.7.x.

## Capability Gaps

1. **Verifier reliability**: Evaluator timeouts produce false reversions. The Day 143 task was reverted not because it failed tests, but because the evaluator never returned a verdict. This wastes implementation effort and creates noise in the success-rate metrics.

2. **Cache observability**: DeepSeek prompt caching is completely invisible for the primary agent loop (#90). Since prompt layout determinism is a core yyds design goal, the inability to measure cache effectiveness undermines that investment.

3. **Task success rate**: 0.0 fitness score in trajectory. Only ~50% of sessions land code. The pattern shows tasks are attempted but frequently reverted — either from evaluator timeouts or from build/test failures that exhaust the fix loop.

4. **Concurrency waste**: 4 of 10 evolution runs cancelled by GH Actions concurrency. Each cancellation burns CI minutes without producing artifacts. The YOYO_SESSION_BUDGET_SECS guard exists on the agent side but isn't wired into the shell-side evolve.sh export.

## Bugs / Friction Found

1. **[HIGH] Evaluator timeout → false revert**: Day 143's task reverted with "Evaluator timed out without a verifier verdict." This is a recurring failure mode (evaluator_unverified_count=1 in trajectory). The evaluator should be bounded in time, and timeouts should be distinguished from failures (they're not — they produce the same revert outcome as a genuine test failure).

2. **[MEDIUM] Cancelled runs waste CI minutes**: 4 of 10 runs cancelled. The YOYO_SESSION_BUDGET_SECS mechanism needs the shell-side export in evolve.sh to take effect. Without it, the agent-side guard can't prevent late-starting sessions.

3. **[MEDIUM] DeepSeek cache blindness**: yoagent Usage struct drops cache metrics (#90). This is a one-line upstream change blocked on human help. Meanwhile, yyds has no workaround.

4. **[LOW] 306 historical unmatched model completions**: Day 142 Task 2 prevents new occurrences, but existing orphans remain. A one-time state repair command or janitor enhancement would clean these up. The Day 143 task attempted this for run-level orphans but reverted due to evaluator timeout.

## Open Issues Summary

| # | Title | Age |
|---|-------|-----|
| 130 | Planning-only session: all 1 selected tasks reverted (Day 143) | Hours |
| 129 | Task reverted: Close orphaned state runs left open after FailureObserved | Hours |
| 128 | Planning-only session: all 1 selected tasks reverted (Day 142) | 1 day |
| 121 | Task reverted: Add success-rate-aware task scoping to preseed task picker | 3 days |
| 105 | Task reverted: Record DeepSeek prompt cache metrics during prompt runs | 6 days |
| 90 | Help wanted: yoagent Usage struct drops DeepSeek cache fields | 11 days |

The reversion cluster (#105, #121, #129) suggests the task picker is selecting tasks that are too large or too fragile for 20-minute implementation windows. Issue #90 needs human attention (upstream yoagent PR).

## Research Findings

**External journal** (`journals/llm-wiki.md`): Tracks a separate project (yopedia/llm-wiki), a knowledge wiki with MCP server, storage provider abstraction, agent self-registration, and scoped search. Active development with multiple sessions per day. No direct impact on yyds harness — it's an independent project being journaled here.

**Competitor landscape**: Claude Code remains the benchmark. No new competitive research performed this session — the assessment budget is better spent on the recurring evaluator timeout and task reversion patterns visible in the trajectory.
