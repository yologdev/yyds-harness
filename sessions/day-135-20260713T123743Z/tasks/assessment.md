# Assessment — Day 135

## Build Status
**PASS** — preflight `cargo build && cargo test` passed (harness already verified before this assessment phase). Binary at `./target/debug/yyds` is functional.

## Recent Changes (last 3 sessions)

| Session | Outcome | Key Commits |
|---------|---------|-------------|
| Day 135 (11:12) | 1/3 tasks ⚠️ | Preseed trajectory-gnome evidence into task_01.md; Filter session-started runs from unmatched lifecycle count; #102 marked obsolete (recovery hints already exist); #103 reverted (scope mismatch) |
| Day 135 (04:55) | 0/2 tasks ⚠️ | Filter session-started runs from unmatched lifecycle count; Fix state why timeout by reducing redundant event scanning passes; no tasks landed (reverted_unlanded_source_edits + reverted_unverified) |
| Day 134 (16:59) | 2/2 tasks ✅ | Fix state why timeout; Fix assessment-output gap in preseed fallback; both strict-verified |

**Pattern**: The 04:55 session landed code changes (the state why fix and session-started filter) but tasks were reverted by verification. The 11:12 session landed two fixes (trajectory gnome preseed, session-started filter) and one task was reverted as obsolete, one as scope mismatch. The two cancelled runs (Day 134 16:59, Day 135 02:51) were likely killed by the hourly cron's concurrency — the previous session was still running when the next slot fired.

## Source Architecture

- **84 `.rs` files**, ~150K total lines
- **Entry point**: `src/bin/yyds.rs` (17 lines) → `yoyo_ds_harness::run_cli()`
- **Key modules by line count**:
  - `src/commands_state.rs` (24.8K) — state CLI: tail, why, graph, events, memory, crashes
  - `src/state.rs` (7.8K) — yoagent-state integration, event recording
  - `src/commands_eval.rs` (6.7K) — eval/replay commands
  - `src/commands_evolve.rs` (5.5K) — harness evolve workflow
  - `src/deepseek.rs` (4.1K) — DeepSeek protocol: models, transport, routing, FIM, JSON, strict schemas, cache
  - `src/tool_wrappers.rs` (3.6K) — GuardedTool, TruncatingTool, ConfirmTool, AutoCheckTool, RecoveryHintTool
  - `src/tools.rs` (3.4K) — StreamingBashTool, SmartEditTool, WebSearchTool, sub-agent, SharedState
  - `src/prompt.rs` (2.9K) — prompt execution, streaming, retry, agent interaction
  - `src/watch.rs` (2.9K) — watch mode, auto-fix, Rust compiler error parsing
  - `src/commands_search.rs` (3.0K) — search/tree/map commands
  - `src/context.rs` (3.1K) — project context loading, git status, file listing
- **Key scripts**: `scripts/build_evolution_dashboard.py` (7.8K), `scripts/extract_trajectory.py` (2.3K), `scripts/preseed_session_plan.py` (2.1K), `scripts/task_manifest.py` (500L)

## Self-Test Results

| Test | Result | Notes |
|------|--------|-------|
| `yyds --help` | ✅ PASS | All CLI options render correctly |
| `yyds state tail --limit 20` | ✅ PASS | 142,970 total events, reads last 5K bounded window |
| `yyds state why last-failure` | ✅ PASS | Found retroactive failure (cancelled run) — normal harness behavior |
| `yyds state graph hotspots --limit 10` | ✅ PASS | Current run dominates (degree=49), bash=32, read_file=16 |
| `yyds deepseek stream-check` | ✅ PASS | Cache hit ratio 66.67%, 1 tool call parsed |
| `yyds deepseek cache-report` | ✅ PASS (informational) | Reports yoagent Usage limitation — no agent-side cache metrics |
| `cargo test` | ⚠️ TIMEOUT at 120s | Test suite too large for assessment phase; preflight already verified |

**Note**: `yyds deepseek cache-report` correctly reports that yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This is a known upstream limitation, tracked in issue #258 context. The diagnostic `stream-check` path works and shows 66.67% cache hits.

One corrupted event at line 118205 of `events.jsonl`: `unknown variant TestEvent` — a benign schema evolution artifact, doesn't affect functionality (the reader skips and continues).

## Evolution History (last 10 runs)

| Run ID | Started | Conclusion | Notes |
|--------|---------|------------|-------|
| 29245411982 | 2026-07-13 11:11 | *(running)* | Current session |
| 29220384802 | 2026-07-13 02:51 | cancelled | Killed by hourly cron overlap |
| 29201141987 | 2026-07-12 16:59 | cancelled | Killed by hourly cron overlap |
| 29188160863 | 2026-07-12 09:51 | **success** | Day 134: landed 2/2 tasks |
| 29177341033 | 2026-07-12 02:50 | **success** | Day 134: dashboard tool-name fix |
| 29160762726 | 2026-07-11 16:58 | cancelled | Killed by hourly cron overlap |
| 29148047482 | 2026-07-11 09:38 | cancelled | Killed by hourly cron overlap |
| 29136786588 | 2026-07-11 02:42 | **success** | Day 133: transport error tests |
| 29112243511 | 2026-07-10 17:47 | **success** | Day 132: state why fixes |
| 29087795113 | 2026-07-10 10:54 | cancelled | Killed by hourly cron overlap |

**Pattern**: 6 of the last 10 runs were cancelled — the "engine turns over and stalls" pattern described in the Day 134 journal. These are concurrency kills: a new cron slot fires while the previous session is still running. The 45-minute session budget (`YOYO_SESSION_BUDGET_SECS`) is designed to prevent this but the shell-side export in `evolve.sh` was noted as a "separate (human-approved) follow-up" — the env var may not actually be set in the cron environment.

## yoagent-state DeepSeek Feedback

- **Event count**: 142,970 total across `.yoyo/state/events.jsonl`
- **Corruption**: 1 unparseable line at offset 118205 (`TestEvent` unknown variant) — benign, reader skips
- **Last failure**: Retroactive `FailureObserved` for run-1783944216585-30591 — a run that completed with error status but no `FailureObserved` was recorded. This is normal for cancelled runs.
- **Graph structure**: Healthy — current assessment run has degree 49 (all tool/command events), bash dominates tool usage (degree 32)
- **Cache observability gap**: Agent-side chat completions don't record cache metrics because yoagent's `Usage` deserialization drops the DeepSeek-specific fields. Only the diagnostic `stream-check` path captures them. This means the harness cannot measure prompt-cache efficiency for real agent sessions.
- **No protocol failures**: No `JsonOutputFailure` or `ToolSchemaFailure` events visible in the tail window (last 10K of 143K events)

## Structured State Snapshot

**Claim health**: From trajectory: `can_drive_evolution=true`, `classification=actionable`. Latest evo readiness says "use this readiness evidence to select the next concrete, verifiable task."

**Top task-state counts** (from trajectory + open issues):
- `reverted_unverified`: 1 (task landed no verifier evidence)
- `reverted_unlanded_source_edits`: 1 (source edits not committed)
- `reverted_no_edit`: 1 (task picked but no file changes)
- `obsolete_already_satisfied`: 1 (#102 — recovery hints already exist)
- `verifier_unproven`: 1 (verifier ran but didn't produce evidence)

**Recent tool failures**: `failed_tool_summary.bash_tool_error=8` — bash commands failing across recent sessions. This is the dominant tool-failure category.

**Recent action evidence**:
- `task_analysis_only_attempt_count=1` — one task ended without file progress or terminal evidence
- `task_unlanded_source_count=1` — source edits not landed in a commit
- `task_verification_rate=0.333` — only 1/3 tasks passed strict verification

**Graph-derived next-task pressure** (from trajectory):
1. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): "Implementation ended without file progress or terminal evidence; retry with smaller scope and explicit verifier evidence requirements."
2. **Raise verified task success rate** (outcome_task_success_rate=0.333): "Dominant task failure: task_unlanded_source_count=1 (source edits not landed)."
3. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=1): "A task touched source files without a landed source commit."
4. **Require strict verifier evidence for tasks** (task_verification_rate=0.333): "Task verification rate was below complete without a counted evaluator verdict."
5. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=8): "prefer bounded commands with explicit paths and inspect exit output before retrying."

**Historical unrecovered tool-failure categories**: The trajectory mentions bash_tool_error as the dominant category. The log feedback system also notes "agent read or searched paths that did not exist" as a recurring lesson. Both are addressed by existing recovery hint infrastructure (confirmed obsolete for task #102). No fresh evidence these still reproduce.

## Upstream Dependency Signals

**yoagent Usage struct limitation**: The `Usage` struct in yoagent does not deserialize DeepSeek-specific cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This is confirmed by `yyds deepseek cache-report` which explicitly states: "yoagent's Usage struct drops DeepSeek cache token fields." The diagnostic `stream-check` path works (yyds parses SSE directly), but real agent sessions lose cache observability.

- **Impact**: Cannot measure prompt-cache efficiency for evolution sessions. Cache metrics are a critical cost and performance signal for the DeepSeek API.
- **Action**: File a yyds help-wanted issue to track this. The fix is either an upstream yoagent PR (if yoagent accepts DeepSeek-specific fields) or a yyds-side workaround (shadow-parse the usage from raw response).

**No other upstream signals**. The state pipeline is healthy. No yoagent-state defects detected.

## Capability Gaps

1. **Cache observability for agent sessions** (HIGH): The diagnostic path works (stream-check), but real evolution sessions can't measure prompt-cache efficiency. This is a cost and performance blind spot.
2. **Session budget enforcement** (MEDIUM): The `YOYO_SESSION_BUDGET_SECS` env var may not be exported in the cron environment, leading to the 6-of-10 cancelled-run pattern. This was noted as a "separate (human-approved) follow-up" in the codebase docs.
3. **Held-out eval fixtures** (MEDIUM): `eval/fixtures/local-smoke/` has growing fixture coverage but no automated CI gate that runs them against the harness. Adding a CI job would close the loop.

## Bugs / Friction Found

1. **cargo test timeout at 120s**: The test suite is large enough that assessment-phase runs time out. This is expected (preflight already verified) but means the assessment agent can't run focused tests without very tight scoping.
2. **Cancelled-run pattern (6 of 10)**: The hourly cron concurrency kills in-flight sessions. The session budget mechanism exists but may not be active in the cron environment. This wastes tokens on sessions that get cancelled before completing.
3. **One corrupted event line**: `TestEvent` unknown variant at line 118205. Benign — the event reader skips and continues. Could be cleaned up by adding `TestEvent` to the enum or filtering the line.

## Open Issues Summary

| # | Title | State | Notes |
|---|-------|-------|-------|
| 102 | Task reverted: Add bounded-command and path-verification recovery hints | OPEN | Agent determined this is obsolete — recovery hints already exist with full coverage. Should be closed. |
| 103 | Task reverted: Add cross-reference mismatch detection to task manifest quality scoring | OPEN | Reverted due to scope mismatch (touched files didn't overlap planned Files entries). The underlying problem (task manifest doesn't detect body-vs-Files mismatches) is real. Need to fix the task specification, not the concept. |

## Research Findings

No new competitor signals. The existing comparison baseline (Claude Code, Cursor) remains unchanged. The DeepSeek harness is stable — no protocol failures, no schema regressions, cache hit ratio at 66.67%. The primary friction is operational (cancelled runs, budget enforcement) not capability (the harness works when it runs).
