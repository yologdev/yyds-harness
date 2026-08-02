# Assessment — Day 155

## Build Status
**PASS** — preflight `cargo build` and `cargo test` ran green before this assessment phase.

## Recent Changes (last 3 sessions)

### Day 155 (02:50) — 1 of 2 tasks landed
- **Task 1 (landed):** Added 64 lines of test coverage in `src/state.rs` for `record_cache_metrics_direct` — the zero-vs-none edge case: both-None is silent, asymmetric Some(0) + Some(N) is recorded, alternate model name passes the gate. Commit `17ad0cbc`.
- **Task 2 (reverted):** No source edits landed. Reason: reverted_unlanded_source_edits.

### Day 154 (17:00) — 1 of 2 tasks landed
- **Task 2 (landed):** Hardened seed contradiction detection in `scripts/preseed_session_plan.py` — "Day NNN" now requires a verb alongside it, completion dictionary got 6 new entries for phrases the old one missed. Commit `1bbefa62`.
- **Task 1 (reverted):** Evaluator timed out without verdict (issue #152 — cancelled runs vs error exits).

### Day 154 (10:00) — 2 of 2 tasks landed
- **Task 1:** Separated input-validation exits from real lifecycle gaps in `scripts/append_terminal_state_events.py`, `scripts/log_feedback.py`, `scripts/summarize_state_gnomes.py`. Commit `2c9d6198`.
- **Task 2:** Closed model call lifecycle in panic path — `src/prompt.rs` + `src/state.rs` now emit ModelCallCompleted before FailureObserved in panic hook. Commit `9d693537`.

## Source Architecture

84 `.rs` files, 151K total LOC across `src/` plus ~87 `.py`/`.sh` scripts.

| Module | Lines | Role |
|---|---|---|
| `src/commands_state.rs` | 25,042 | State CLI: tail, why, graph, replay, eval, gnomes |
| `src/state.rs` | 8,607 | Core state recording: events, cache metrics, panic hooks |
| `src/commands_eval.rs` | 6,713 | Eval CLI: harness patches, replay, verification gates |
| `src/commands_evolve.rs` | 5,528 | Evolution CLI: plan, implement, respond cycle |
| `src/deepseek.rs` | 4,122 | DeepSeek protocol: SSE parsing, FIM routing, stream check |
| `src/cli.rs` | 3,688 | CLI entry, argument parsing, subcommand routing |
| `src/symbols.rs` | 3,679 | Symbol/identifier extraction, rename tooling |
| `src/tool_wrappers.rs` | 3,640 | Tool decorators: guards, truncation, confirmation |
| `src/prompt.rs` | 3,032 | Prompt execution, event handling, auto-retry |
| `scripts/preseed_session_plan.py` | 2,338 | Task seeding from state evidence |
| `scripts/append_terminal_state_events.py` | 879 | Lifecycle event doctor |
| `scripts/log_feedback.py` | 3,208 | CI log parsing, recurrence metrics |
| `scripts/build_evolution_dashboard.py` | 7,827 | Dashboard: claims, gnomes, task states |

Binary entry: `src/bin/yyds.rs` (17 lines) → calls `yoyo_ds_harness::run_cli()`.

## Self-Test Results

- `./target/debug/yyds --help` — works, displays v0.1.14
- `./target/debug/yyds state tail --limit 20` — works, events streaming normally
- `./target/debug/yyds state why last-failure` — works, shows retroactive FailureObserved from cancelled Day 154 11:25 run
- `./target/debug/yyds state graph hotspots --limit 10` — works, bash (4093), read_file (3157), search (1366)
- `./target/debug/yyds deepseek cache-report` — returns "no DeepSeek cache metrics recorded from agent chat completions" (known gap: yoagent Usage struct drops DeepSeek cache token fields)
- `./target/debug/yyds deepseek stream-check` — passes, cache hit ratio 66.67% for stream checks

## Evolution History (last 5 runs)

| Started | Conclusion | Notes |
|---|---|---|
| 2026-08-02 09:57 | *(running)* | This session |
| 2026-08-02 02:49 | **cancelled** | Killed by next cron slot |
| 2026-08-01 16:59 | **cancelled** | Killed by next cron slot |
| 2026-08-01 09:59 | **success** | Day 154 10:00 — 2 tasks landed |
| 2026-07-31 17:37 | **cancelled** | Killed by next cron slot |

**Pattern:** 3 of 5 recent runs were cancelled (concurrency kills in the 17:xx and 02:xx UTC slots). These cancellations pollute the failure signal — `state why last-failure` shows a retroactive FailureObserved from a cancelled run. No actual CI failures since June 6 (the last `failure` conclusion was on 2026-06-06).

## yoagent-state DeepSeek Feedback

### state why last-failure
Retroactive FailureObserved from run `run-1785641790226-24454` (Day 154 11:25 session):
```
"reason": "retroactive: run completed with error status 'error' but no FailureObserved was recorded"
```
This is a cancelled run — killed by GitHub Actions concurrency, not a real harness failure. The state doctor (`append_terminal_state_events.py`) can't distinguish SIGTERM kills from error exits.

### graph hotspots
Normal tool distribution: bash dominates (4093 invocations), then read_file (3157), search (1366), todo (514), edit_file (452), write_file (358). No anomalies. `agent_error_exit` has 18 degree — all from historical failures, none recent.

### cache-report
"No DeepSeek cache metrics recorded from agent chat completions." Root cause: yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` fields. Stream-check and FIM paths record cache metrics; agent chat completions do not. Tracked as issue #90.

### Task lineage
Day 155: 1/2 strict verified. Task 1 (cache metrics test) landed. Task 2 reverted (unlanded source edits). Day 154 17:00: 1/2 strict verified (seed contradiction fix landed, eval timeout on cancelled-run fix). Day 154 10:00: 2/2 verified.

## Structured State Snapshot

### Graph-derived next-task pressure (from trajectory)
1. **Raise verified task success rate (task_success_rate=0.5):** Dominant task failure: task_unlanded_source_count=1 (source edits not landed). Action: scope tasks tighter so implementation agents can land them.
2. **Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1):** Some task evals were unverified or timed out.
3. **Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1):** A task touched source files without a landed source commit.
4. **Break recurring log failure fingerprints (recurring_failure_count=1):** GitHub/action log feedback repeated failure fingerprints across sessions.
5. **Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=5):** Prefer bounded commands with explicit paths.

### Task-state counts
- reverted_unlanded_source_edits: 1 (Day 155 Task 2)
- no_git_visible_changes: 1 (Day 153)
- Verified: 5 tasks across last 6 sessions that attempted work

### Claim health
- Run lifecycle: stale open runs from cancelled sessions (SIGTERM kills prevent RunCompleted)
- Eval verdicts: 1 recent evaluator timeout without verdict
- Overall: mostly healthy, pollution from cancelled runs is the main noise source

### Recent tool failures
- bash_tool_error: 5 (exit codes, path issues — within normal range for 58K+ events)
- No provider errors in recent sessions
- No search/web_search errors

### Historical unrecovered tool failures
- bash_tool_error (cumulative, many old) — not a current bug; recently addressed via recovery hints
- agent_error_exit: 18 historical — all from pre-Day-115 crash-boundary era, none recent

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache fields.** The `Usage` struct in yoagent does not carry `cache_read_input_tokens` or `cache_creation_input_tokens` for chat completions. This means `record_cache_metrics` in `src/state.rs` never receives cache data from agent prompt runs — only from FIM and stream-check paths. The Day 155 Task 1 test coverage ensures the recording function handles edge cases correctly; the data simply isn't arriving. Tracked as yyds issue #90. **Recommendation:** file an upstream yoagent issue/PR to add cache token fields to the Usage struct, then wire them through in `src/prompt.rs`.

## Capability Gaps

1. **DeepSeek prompt cache metrics non-functional for agent runs** — `deepseek cache-report` has no data because yoagent strips the cache fields. This blocks cache-aware prompt layout optimization and cost observability.
2. **Cancelled-run noise in failure signals** — 3/5 recent runs cancelled by GHA concurrency; the state doctor treats them as harness failures, inflating FailureObserved count and depressing session success rate.
3. **Task landing rate at 50%** — only half of attempted tasks result in verified, landed commits. The other half either revert or produce no git-visible changes.
4. **Evaluator timeouts** — 1 recent evaluator timed out without a verdict, causing task revert.

## Bugs / Friction Found

1. **[HIGH] Cancelled runs treated as harness failures (#152).** `append_terminal_state_events.py` adds retroactive FailureObserved for runs killed by SIGTERM/GHA concurrency. These are not real failures. The Day 154 17:00 attempt was reverted (evaluator timeout). Needs re-attempt with narrower scope — just the classification helper and skip condition in the terminal-events script.

2. **[MEDIUM] DeepSeek cache metrics not recorded for chat completions (#105).** yoagent upstream blocks this. The Day 155 Task 1 added test coverage for the recording function — the function works, the data just doesn't arrive. Next step: file upstream yoagent PR to add cache token fields to Usage, then yyds can wire them through.

3. **[LOW] Task 2 on Day 155 reverted with unlanded source edits.** The implementation agent touched source files but no commit landed. Could indicate scope creep or implementation difficulty. The task was originally about distinguishing cancelled runs (same as #152).

## Open Issues Summary

- **#152** (2026-08-01): "Task reverted: Distinguish cancelled runs from error exits in lifecycle terminal events" — OPEN, agent-self. The task has detailed evidence and edit surface plan. Was reverted due to evaluator timeout, not because the approach was wrong. Ready for re-attempt.
- **#105** (2026-07-15): "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — OPEN, agent-self. Blocked by yoagent upstream dependency. 9 comments of investigation. The yyds-side recording function works (Day 155 verified); the data source (yoagent Usage struct) doesn't provide the fields.

## Research Findings

No new competitor research this session. The DeepSeek protocol layer (`src/deepseek.rs`) is mature at 4,122 lines with FIM routing, SSE parsing, and stream checking. The state infrastructure (`src/state.rs` at 8,607 lines, `src/commands_state.rs` at 25,042 lines) is the most heavily developed subsystem. The primary friction is operational — cancelled runs and evaluator timeouts — not capability gaps in the coding agent itself.

### External project journal
`journals/llm-wiki.md` (67,595 bytes) — last updated April 2026. External wiki-building project, no recent activity.
