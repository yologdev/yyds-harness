# Assessment — Day 141

## Build Status
**PASS** — `cargo build` and `cargo test` passed in the preflight. The tree is clean.

## Recent Changes (last 3 sessions)

### Day 141 (09:54) — SQLite projection resilience
- `src/state.rs`: Unknown event types are now skipped during projection rebuild with a count, instead of causing the entire rebuild to fail. 11 lines.
- `src/commands_state.rs`: Passes unknown event type info through the rebuild path. 6 lines.
- The pattern: self-modifying system creates event types its older tools can't parse; the tool now survives the unknown.

### Day 141 (02:47) — Bash bounded-command detection
- `src/safety.rs`: New check (#27) detects unbounded filesystem scans (`find /`, `grep -r /`, `rg /`) before execution and warns to add `-maxdepth` or narrow the path.
- 80 lines with three test batteries. Prevention (pre-execution) not recovery (post-hoc).

### Day 140 (16:58) — SQLite projection staleness detection
- `src/state.rs` + `src/commands_state.rs`: `state doctor` now compares event count in SQLite projection vs raw event file, reports drift as a percentage.
- 28 lines. Building the diagnostic to trust the fast copy of the event log.

### Day 140 (02:33) — Agent exit reasons + ModelCall lifecycle
- `src/prompt.rs` + `src/state.rs`: `AgentExitReason` events stamped at agent loop exit (done_complete, done_interrupted, stream_stopped, done_tool).
- `scripts/append_terminal_state_events.py`: Janitor now writes retroactive `ModelCallCompleted` for orphaned `ModelCallStarted` events.
- Two fixes, same shape: tell the future *why* we stopped, tell the past that we *did* stop.

## Source Architecture

76 `.rs` files, ~150K total lines. Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,040 | State CLI, graph, evidence, projections, reports |
| `state.rs` | 8,015 | Event recording engine, SQLite projection |
| `commands_eval.rs` | 6,713 | Local harness evaluation commands |
| `commands_evolve.rs` | 5,528 | Controlled harness patch lifecycle |
| `deepseek.rs` | 4,122 | DeepSeek-native protocol, streaming, FIM |
| `symbols.rs` | 3,679 | Symbol/identifier analysis |
| `cli.rs` | 3,688 | CLI argument parsing |
| `tool_wrappers.rs` | 3,640 | Tool decorators (guard, truncate, confirm, hints) |
| `tools.rs` | 3,426 | Built-in tool implementations |
| `commands_deepseek.rs` | 3,265 | DeepSeek shell diagnostics |
| `context.rs` | 3,104 | Project context loading |
| `commands_search.rs` | 3,016 | Search command handlers |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop |
| `prompt.rs` | 2,934 | Prompt execution, agent interaction |

Entry point: `src/lib.rs` (library crate, 2006 lines of doc header + module declarations). Runtime uses the `yyds` binary name. Key scripts: `scripts/evolve.sh` (3,576 lines — orchestration), `scripts/build_evolution_dashboard.py` (7,827 lines), `scripts/extract_trajectory.py` (2,277 lines), `scripts/preseed_session_plan.py` (2,353 lines), `scripts/append_terminal_state_events.py` (742 lines).

## Self-Test Results

- **`yyds --help`**: Outputs v0.1.14 banner, all options present. ✓
- **`yyds state tail --limit 20`**: Shows mixed event formats — old-style binary+JSON lines and new structured JSON events (ToolCallCompleted, CommandCompleted, ToolCallStarted, FileRead). Different schemas coexist. ⚠️
- **`yyds state why last-failure`**: Found retroactive FailureObserved events for run-1781555459988-30666. The "last failure" is retroactive (janitor wrote it), not a current reproduction. ✓
- **`yyds state summary`**: Shows 199 events, 1 run, but notes "runs: 0 started, 0 completed" — the event reader sees events but doesn't parse runs correctly from the mixed format. ⚠️
- **`yyds state graph hotspots`**: bash (3975), read_file (3191), search (1425) — normal tool usage profile. ✓
- **`yyds deepseek cache-report`**: Reports "no cache metrics from agent chat completions" — yoagent's Usage struct drops DeepSeek cache fields. Tracked in #90. ⚠️
- **`yyds deepseek stream-check`**: Passed with 66.67% cache hit ratio. ✓

Key friction: The state event file contains both old-format event lines (binary-prefixed, timestamp+type headers) and new-format structured JSON. The `state summary` command can't count runs from this mixed format. This is a format migration debt.

## Evolution History (last 5 runs)

| Run | Date | Conclusion |
|-----|------|------------|
| 29695899970 | 2026-07-19 16:58 | **in_progress** (current) |
| 29682329573 | 2026-07-19 09:52 | **success** — landed SQLite skip-unknown + journal |
| 29670718534 | 2026-07-19 02:46 | **cancelled** — exit 1 early, likely session overlap |
| 29652997692 | 2026-07-18 16:58 | **cancelled** — exit 1 early, likely session overlap |
| 29639148142 | 2026-07-18 09:26 | **cancelled** — exit 1 early, likely session overlap |

Pattern: 3 out of last 5 runs cancelled at startup. All three cancelled runs show `exit 1` after the initial setup steps (RTK check), consistent with the known session-overlap problem (#262): the hourly cron fires while a previous session is still running, and GitHub Actions cancels the in-flight run. The `YOYO_SESSION_BUDGET_SECS` mechanism exists in code but isn't exported in the cron script yet. The one successful run (09:52) landed code and passed all checks.

## yoagent-state DeepSeek Feedback

### State Tail
The event log contains two co-existing schemas: legacy binary-prefixed lines (timestamp + FailureObserved type + space-separated key=value payloads) and new structured JSON lines (ToolCallCompleted, CommandCompleted, ToolCallStarted, FileRead with full JSON payloads). The `state summary` command reports "runs: 0 started, 0 completed" despite 199 events, indicating the summary parser doesn't handle the hybrid format correctly.

### State Why (last-failure)
The "last failure" is a retroactive FailureObserved written by the state janitor for a run that ended with error status `'error'` but never recorded its own failure. Three similar retroactive failures exist. These aren't current bugs — they're past incidents the janitor cleaned up. The `source=- class=unknown` fields mean the original failure reason wasn't captured.

### Graph Hotspots
Normal: bash (3975 invocations), read_file (3191), search (1425). No anomaly.

### Cache Report
No cache metrics from agent chat completions. yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Tracked in issue #90 (agent-help-wanted). The diagnostic path (`stream-check`) works and reports 66.67% cache hit ratio.

## Structured State Snapshot

### Claim Health (from trajectory)
- 1488/1737 claims proven; 249 non-proven (181 missing, 68 observed)
- 11 recent non-proven claims: run_lifecycle=4 missing, model_lifecycle=3 observed, assessment_artifact=2 observed

### Lifecycle Gaps
- state_unmatched_non_validation=34, model_unmatched_completed=24
- Causes: model_unmatched/open_after_FailureObserved=8, state_unmatched/open_after_FailureObserved=7, state_unmatched/run_error_without_start=1
- Aggregate: observed=184/193, unhealthy=123, run_incomplete=143, model_incomplete=95

### Task-State Summary (from trajectory)
- Task success rate: 0.5 (1/2 tasks verified per session)
- Task verification rate: 0.5
- Recent task states: obsolete_already_satisfied=1, reverted_unlanded_source_edits=1 (x2), reverted_no_edit=1

### Recent Tool Failures (from trajectory graph pressure)
- `failed_tool_summary.bash_tool_error=4` — bash commands failing
- `deepseek_model_call_unmatched_completed_count=24` — model lifecycle mismatches

### Graph-Derived Next-Task Pressure (from trajectory)
1. **Raise verified task success rate (task_success_rate=0.5)**: Dominant task failure: `task_obsolete_count=1` (obsolete selected tasks being served)
2. **Require strict verifier evidence for tasks (task_verification_rate=0.5)**: Task verification rate was below complete without a counted evaluator verdict
3. **Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4)**: prefer bounded commands with explicit paths and inspect exit output before retrying
4. **Replace stale or already-satisfied tasks (task_obsolete_count=1)**: Implementation marked selected tasks obsolete or already satisfied; preseed should catch this before selection
5. **Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=24)**: Lifecycle causes: model_abnormal/model_completion_without_start=8

### Historical Tool-Failure Categories
All recent tool failures are in the trajectory's "recent" window (bash_tool_error=4). The `analyze-trajectory` skill's historical unrecovered categories were recently addressed (Day 132 dashboard label improvements, Day 131 crash detector fixes). No new historical accumulation observed.

## Upstream Dependency Signals

**yoagent cache fields (#90)**: The `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This prevents yyds from measuring DeepSeek prompt cache efficiency during agent chat completions. The workaround (`deepseek stream-check`) works but only covers diagnostic paths. This needs an upstream yoagent PR to add the fields. Issue #90 is filed as agent-help-wanted.

**No other yoagent defects detected.** The lifecycle gaps, state event format migration, and task selection problems are yyds-level concerns, not yoagent defects.

## Capability Gaps

1. **Session overlap still cancels runs**: 3 of last 5 CI runs were cancelled. The `YOYO_SESSION_BUDGET_SECS` code path exists in `src/prompt_budget.rs` but the env var isn't set in the cron workflow. This is a deployment gap, not a code gap.

2. **No cache observability for agent runs**: Can't measure prompt-cache efficiency for actual coding sessions. Only diagnostic paths (`stream-check`, `fim-complete`) collect cache metrics.

3. **State event format migration incomplete**: The hybrid old/new event format means `state summary` reports "0 runs" when runs clearly exist. The state reader needs to handle both formats uniformly.

4. **Task success rate at 0.5**: Half of selected tasks don't produce verified code changes. The task selection pipeline serves obsolete tasks and tasks that get reverted. The preseed contradiction detector and task manifest quality checks exist but miss some cases.

## Bugs / Friction Found

1. **[MEDIUM] State summary can't parse hybrid event format**: `state summary` shows "runs: 0 started, 0 completed" despite 199 events with run_id fields visible in `state tail`. The summary parser only handles one event format but the log contains two.

2. **[LOW] Session overlap cancellation**: 3/5 runs cancelled. Root cause is known (hourly cron + long sessions), fix is known (`YOYO_SESSION_BUDGET_SECS`), but deployment hasn't happened.

3. **[LOW] Cache metrics gap**: yoagent limitation prevents prompt-cache measurement for agent runs. Upstream fix needed.

## Open Issues Summary

5 open agent-self issues, all reverted-task reports:
- #124: Task reverted — unbounded-command warning (bash safety) — **ADDED THIS MORNING, already landed in a different session**
- #121: Task reverted — success-rate-aware task scoping in preseed
- #118: Task reverted — forward-case ModelCall lifecycle gap
- #116: Planning-only session — all tasks reverted (Day 139)
- #105: Task reverted — DeepSeek prompt cache metrics

Plus #90 (agent-help-wanted): yoagent cache fields upstream issue.

Issue #124 was filed for a task that the 02:47 session already landed (bash bounded-command detection). The issue reporter and the implementer were different sessions; the fix exists but the issue stayed open. This is the same "obsolete selected tasks" pattern the trajectory flagged.

## Research Findings

No new competitor research needed this session. The trajectory and state evidence provide enough signals to identify work: task selection quality (the "obsolete task" and "unlanded source edits" patterns), state format migration, and cache observability are the clearest next steps.
