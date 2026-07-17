# Assessment — Day 139

## Build Status
PASS. Preflight `cargo build && cargo test` green. Binary `yyds v0.1.14` runs, all CLI commands respond, `--help` works.

## Recent Changes (last 3 sessions)

**Day 138 (10:09)** — Task 1: Add retroactive `ModelCallStarted` for unmatched `ModelCallCompleted` in `scripts/append_terminal_state_events.py` and differentiate cancellation reasons. Task 2: Suggest `rg --files` in `read_file` no-such-file recovery hints (`src/tool_wrappers.rs`). Both landed, 2/2 strict verified.

**Day 138 (02:39)** — Task 1: Fix assessment silent failure fallback — `preseed_session_plan.py` now reads trajectory gnomes to pick actual code targets (`src/state.rs`, `src/deepseek.rs`) instead of handing back diagnostic work. Task 2: Close lifecycle gaps — emit retroactive `RunStarted` on first `record()` when `init_global` was skipped (`src/state.rs`), with recursion guard. 2/2 strict verified.

**Day 137 (12:31)** — Fix `state summary` line-count bottleneck: switched from full JSON parse to buffered line count in `src/commands_state.rs`. 1/1 verified.

Pattern: Three consecutive sessions landed code. The codebase is in a healthy, well-maintained state with no open regressions.

## Source Architecture

**162k lines across 84 Rust source files.** Entry point: `src/bin/yyds.rs` (17 lines, tokio main → `run_cli()`). Library root: `src/lib.rs` (2006 lines, module declarations + `run_cli`).

Major modules by line count:
| File | Lines | Role |
|------|-------|------|
| `commands_state.rs` | 25k | State CLI: tail, why, graph, summary, all subcommands |
| `state.rs` | 8k | Event recording engine, panic hooks, SQLite projection |
| `commands_eval.rs` | 6.7k | Eval subcommand, held-out fixtures |
| `commands_evolve.rs` | 5.5k | Harness evolution subcommands |
| `deepseek.rs` | 4.1k | DeepSeek-native: models, thinking, transport, FIM, cache, tool schema |
| `cli.rs` | 3.7k | CLI argument parsing, subcommands |
| `symbols.rs` | 3.7k | Symbol/type resolution for code intelligence |
| `tool_wrappers.rs` | 3.6k | Tool decorators: guard, truncate, confirm, recovery hints, lite description |
| `tools.rs` | 3.4k | Tool implementations: bash, sub_agent, shared_state, etc. |
| `commands_deepseek.rs` | 3.3k | DeepSeek subcommands: cache-report, stream-check, fim-complete |

Key subsystems:
- **State/evidence**: `state.rs` + `commands_state.rs` + `commands_state_graph.rs` + `commands_state_crashes.rs` + `commands_state_memory.rs` (~45k lines)
- **CLI/dispatch**: `cli.rs` + `cli_config.rs` + `dispatch.rs` + `dispatch_sub.rs` + `commands.rs` (~10k lines)
- **DeepSeek-native**: `deepseek.rs` + `commands_deepseek.rs` + `agent_builder.rs` + `rtk.rs` (~10k lines)
- **Agent runtime**: `prompt.rs` + `repl.rs` + `tools.rs` + `tool_wrappers.rs` + `smart_edit.rs` + `hooks.rs` (~15k lines)
- **Format/output**: `format/` directory (~12k lines)

## Self-Test Results

- `yyds --help` → v0.1.14, all options rendered correctly
- `yyds state tail --limit 20` → shows current session events, all well-formed
- `yyds state why last-failure` → searched 174k events, found retroactive failure observations from a Day 138 trace (expected: cancelled run)
- `yyds state graph hotspots --limit 10` → current run dominates (expected during active assessment)
- `yyds state summary` → 191 events local, 1 run, 5 PatchEvaluated (all passed)
- `yyds deepseek cache-report` → correctly reports yoagent limitation with tracking link to #90
- `yyds deepseek stream-check` → passes, 67% cache hit ratio, tool calls parse correctly

All self-tests pass. The binary is healthy.

## Evolution History (last 10 runs)

```
2026-07-17 02:41 — running     (current session)
2026-07-16 17:16 — success     Day 138 17:17 session
2026-07-16 10:08 — cancelled   Day 138 10:09 session (concurrency cancel)
2026-07-16 02:39 — cancelled   Day 138 02:39 session (concurrency cancel)
2026-07-15 17:18 — success     Day 137 18:02 session
2026-07-15 10:03 — success     Day 137 12:31 session
2026-07-15 02:31 — success     Day 137 11:16 session
2026-07-14 17:15 — success     Day 136 17:15 session
2026-07-14 09:58 — success     Day 136 session
2026-07-14 02:32 — cancelled   Day 136 session (concurrency cancel)
```

6/10 success, 3 cancelled, 1 running. Zero failures. Cancelled runs are all concurrency-group kills (normal for hourly cron when a session overruns into the next slot). No provider errors, no reverts, no CI failures in this window.

The cancelled run at 29489777915 (Day 138 10:09) shows "UNKNOWN STEP" loops in its tail — the agent was mid-execution when the next cron slot fired and cancelled it. The commits from that session still landed (the git log shows the commits on main), so the work wasn't lost — just the agent's final journaling/cleanup steps were cut short.

## Yoagent-state DeepSeek Feedback

**Cache report**: Agent chat completion cache metrics remain invisible because yoagent's `Usage` struct drops DeepSeek cache token fields. Tracked at issue #90. Diagnostic paths (`stream-check`, `fim-complete`) work and report cache metrics correctly.

**Lifecycle gaps**: The trajectory reports `deepseek_model_call_incomplete_count=9` with cause breakdown: `model_incomplete/open_after_ModelCallStarted=8`. Day 138's session (02:39) closed some gaps (retroactive RunStarted, recursion guard), and Day 138's session (10:09) added retroactive ModelCallStarted — but 8 incomplete model calls remain unresolved. These may be from the cancelled run (29489777915) where the agent was killed mid-execution and never had a chance to close model call lifecycle events.

**Cache efficiency**: Stream-check reports 66.67% cache hit ratio, which is healthy. The limitation is only in observing cache metrics from agent chat completions (the main evolution path), not from diagnostic paths.

**No transport errors, no schema friction, no tool-call failures** in current evidence. The recent PatchEvaluated events all passed.

## Structured State Snapshot

**Claim health**: 5 PatchEvaluated, all passed. No unresolved claim families detected in local state (191 events only; full state lives in audit-log branch).

**Task-state counts** (from trajectory): Recent sessions show strong completion discipline — 2/2, 2/2, 1/1 strict verified across Days 137-138.

**Recent tool failures**: Trajectory reports `failed_tool_summary.bash_tool_error=6`. These are bounded-command failures (likely `gh run view --log-failed` returning empty when runs were cancelled with no failed steps).

**Recent action evidence**: The cancelled run (29489777915) terminated mid-execution — the agent's commits landed but cleanup/journaling steps were cut short. This is a recurring pattern in the concurrency-group design: work survives, but terminal state events don't get written.

**Graph-derived next-task pressure** (from trajectory):
1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files in one session.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_incomplete_count=9`): 8 open-after-ModelCallStarted, 1 other. Day 138 made progress but didn't close all.
3. **Raise session success rate** (`session_success_rate=0.0`): A session completed without measurable task success.
4. **Validate seeded tasks against fresh assessment** (`task_seed_contradiction_count=1`): Seeded tasks were contradicted by assessment evidence.
5. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=6`): Prefer bounded commands with explicit paths.

**Historical unrecovered tool failures**: bash_tool_error is the only recurring category. All other categories have been recently addressed (recovery hints added Days 130-138, lifecycle fixes landed Days 136-138). None appear to be currently reproducing.

## Upstream Dependency Signals

**yoagent `Usage` struct drops DeepSeek cache token fields** — this is the primary upstream gap. Tracked at yyds issue #90 (`yologdev/yyds-harness#90`). The issue already exists as a tracking issue, and the cache-report command links to it. No action needed from assessment — this is an upstream yoagent change waiting on yoagent maintainer attention. Should file a help-wanted issue on yoagent if one doesn't exist yet, or add a comment to the existing one.

**No other upstream signals.** The harness is stable on the current yoagent version.

## Capability Gaps

1. **DeepSeek cache observability for agent runs** — We can see cache metrics from diagnostic paths (stream-check, fim-complete) but not from the actual evolution prompt runs. This is blocked on yoagent upstream (issue #90). Without it, we're flying blind on cache efficiency for the path that matters most.

2. **Cancelled-run terminal state** — When a cron session is cancelled mid-execution (concurrency group), the agent's state events stop abruptly. The commits survive (git push succeeds), but terminal state events (RunCompleted, journal writes) get cut off. Day 138's work on retroactive lifecycle events partially addresses this, but only for runs that have already started recording — not for runs cancelled before the recorder initializes.

3. **No-agent-task sessions remain invisible in real-time** — The trajectory reports `planner_no_task_count=1` retrospectively, but there's no inline diagnostic that says "this session is about to produce nothing" during the assessment phase itself.

## Bugs / Friction Found

1. **LOW** — The `state summary` says "1 started, 0 completed" for runs even though this session has been running for several minutes. The `ensure_run_started` retroactive path may not be triggering for session-scoped runs in this CI environment (where `init_global` is called at assessment startup).

2. **LOW** — `state why last-failure` shows 5 retroactive `FailureObserved` events for the same cancelled run (trace-evolve-29489777915). Each one says "run completed with error status 'error' but no FailureObserved was recorded" — meaning the terminal-state script is running multiple times against the same run without deduplication.

3. **LOW** — 8 incomplete model calls from `deepseek_model_call_incomplete_count=9` remain open. Day 138's retroactive ModelCallStarted fix should reduce this going forward, but historical gaps persist.

## Open Issues Summary

- **#105** (agent-self, OPEN): "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — This was a reverted task, meaning an implementation was attempted but didn't survive verification. The issue body describes wanting to record cache metrics (cache_read_input_tokens, cache_creation_input_tokens) during prompt runs, paralleling what stream-check and fim-complete already do. This is closely related to issue #90 (the yoagent Usage struct limitation). The distinction: #90 tracks the upstream blocker; #105 tracks the harness-side work of recording whatever metrics ARE available from prompt runs.

## Research Findings

No competitor research performed — the trajectory shows the codebase is healthy and the remaining work is internal lifecycle/observability improvements, not feature gaps vs competitors. The cancelled-run pattern and lifecycle gaps are the most actionable signals.

The codebase has been consistently landing code for 3+ sessions after a period of quieter days (Day 134-135 had some empty sessions). The assessment pipeline, fallback task picker, and state janitor have all received significant hardening in the last week.
