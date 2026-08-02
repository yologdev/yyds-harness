# Assessment — Day 155

## Build Status
**Pass.** `cargo build` and `cargo test` preflight green. Binary reports `yyds v0.1.14 (41df7ca3 2026-08-02) linux-x86_64`.

## Recent Changes (last 3 sessions)
| Session | What Landed |
|---------|-------------|
| Day 155 (09:57) | Quiet session; journal entry only |
| Day 155 (02:50) | `record_cache_metrics_direct` zero-vs-none edge case tests (src/state.rs); seed contradiction detection hardening (scripts/preseed_session_plan.py) |
| Day 154 (10:00) | Close model-call lifecycle in panic path (src/state.rs, src/prompt.rs); stop treating empty_input runs as crashes (scripts/append_terminal_state_events.py) |

The 02:50 session landed two real patches. The 09:57 session was a quiet follow-up — the house was clean. Codebase is healthy.

## Source Architecture
- **Binary entry**: `src/bin/yyds.rs` (17 lines) → `yoyo_ds_harness::run_cli()`
- **84 source files**, ~151K total lines
- **Largest modules**: `commands_state.rs` (25K, CLI state subcommands), `state.rs` (8.6K, event recording/state management), `commands_eval.rs` (6.7K), `commands_evolve.rs` (5.5K), `deepseek.rs` (4.1K, DeepSeek protocol layer), `cli.rs` (3.7K), `symbols.rs` (3.7K), `tool_wrappers.rs` (3.6K), `tools.rs` (3.5K), `commands_git.rs` (3.6K), `commands_deepseek.rs` (3.3K), `prompt.rs` (3.0K), `context.rs` (3.1K), `watch.rs` (2.9K)
- **Module groups**: `commands_*.rs` (CLI subcommand implementations), `format/` (rendering), `state.rs` (event recording), `prompt*.rs` (agent interaction), `deepseek.rs` (protocol), `config.rs` (TOML/permissions)
- **Python scripts**: `scripts/evolve.sh` (3576 lines, harness), `build_evolution_dashboard.py`, `preseed_session_plan.py`, `log_feedback.py`, `extract_trajectory.py`, `summarize_state_gnomes.py`, `append_terminal_state_events.py`, etc.

## Self-Test Results
- `--version`: OK, v0.1.14
- `state tail --limit 20`: OK, shows active session events
- `state why last-failure`: OK, found retroactive FailureObserved
- `state graph hotspots --limit 10`: OK, normal tool distribution
- `deepseek stream-check`: OK, 66.67% cache hit ratio, 12 input / 3 output tokens
- `deepseek cache-report`: Returns "no DeepSeek cache metrics recorded from agent chat completions" — known upstream issue (#90)

## Evolution History (last 5 runs)
| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-08-02 16:59 | *(running)* | This session |
| 2026-08-02 09:57 | success | Quiet session; journal only |
| 2026-08-02 02:49 | cancelled | Killed by concurrency/SIGTERM |
| 2026-08-01 16:59 | cancelled | Killed by concurrency/SIGTERM |
| 2026-08-01 09:59 | success | Landed 2 tasks |

**Pattern**: Two cancelled runs out of 4 completed (02:49 and 16:59 slots). These are likely GitHub Actions concurrency kills — the next cron session starts before the previous one finishes. The retroactive FailureObserved in `state why` is from one of these cancelled runs.

## yoagent-state DeepSeek Feedback
- **State tail**: Normal tool usage pattern — bash, read_file, search, list_files, todo. Active session in progress.
- **State why last-failure**: Retroactive FailureObserved for run-1781372620921-38655. The run had 4 `RunCompleted status=error` events, each with zero-token model calls, then a retroactive FailureObserved. This is a cancelled run being re-processed by the state doctor.
- **Graph hotspots**: Normal — bash (4109 invocations), read_file (3141), search (1366), todo (514), edit_file (452), write_file (358). No anomalous tool patterns.
- **Cache report**: **CRITICAL GAP** — yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Agent chat completions record zero cache metrics. Diagnostic paths (`stream-check`, `fim-complete`) work correctly. Tracked as #90 (open since Day 137).

## Structured State Snapshot
*(from trajectory extractor + state CLI)*

**Claim health**: trajectory reports `can_drive_evolution=false` with classification `no_task_evidence`. session_success_rate=0.0 reflects the cancelled sessions where no tasks were attempted.

**Unresolved claim families**: `deepseek_model_call_unmatched_completed_count=22` — cancelled runs leave open model-call lifecycles. `task_seed_contradiction_count=1` — seed tasks contradicted by assessment.

**Task-state counts** (from recent trajectory): Day 155 (02:50): tasks_attempted=2, reverted_unlanded_source_edits=1 (Task 2 reverted). Day 154 (10:00): 2/2 strict verified. Day 154 (09:59): 2/2 strict verified.

**Recent tool failures**: None in current trajectory snapshot. Graph hotspots show normal tool distribution.

**Recent action evidence**: Normal — recent sessions show standard bash/read_file/edit_file patterns.

**Graph-derived next-task pressure** (from trajectory extractor):
- **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files.
- **Close yyds state and model lifecycle gaps** (deepseek_model_call_unmatched_completed_count=22): Lifecycle causes: model_incomplete/model_completion_without_start=8
- **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly even though task success was partial.
- **Validate seeded tasks against fresh assessment** (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence.
- **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Recent task session day-155-20260802T025017Z: Some task evals were unverified.

**Historical unrecovered tool-failure categories**: `shell tool commands failed` (prefer bounded commands with explicit paths). This is a recurring pattern but was noted as recently addressed — not current pressure without reproduction evidence.

## Upstream Dependency Signals
**yoagent Usage struct**: The `Usage` struct returned by yoagent after chat completions does not carry DeepSeek-specific `cache_read_input_tokens` / `cache_creation_input_tokens` fields. This is the root cause of `deepseek cache-report` returning no metrics for agent runs. Workaround exists for diagnostic paths (stream-check, fim-complete) which parse raw SSE. Fix requires an upstream yoagent change to expose provider-specific usage fields, or a yyds-side workaround intercepting the raw response before yoagent strips it. **Recommendation**: File an upstream yoagent issue/PR to add provider-extension fields to `Usage`, and consider a yyds-side intercept as fallback.

## Capability Gaps
1. **Cache observability** (HIGH): No DeepSeek prompt-cache metrics from agent runs. Cannot optimize stable-prefix layout or track cache cost savings. Root cause is upstream (yoagent Usage struct).
2. **Cancelled-run noise** (MEDIUM): Cancelled sessions pollute FailureObserved counts and model-lifecycle metrics. The state doctor corrects some of this but doesn't recognize SIGTERM/concurrency kills as distinct from errors. Issue #152 was filed but task reverted (evaluator timeout).
3. **Planning reliability** (MEDIUM): Trajectory shows planner_no_task_count and no_task_evidence across multiple recent sessions. The assessment-to-task pipeline still produces empty sessions.

## Bugs / Friction Found
1. **Cancelled-run terminal events** — `state why last-failure` shows retroactive FailureObserved for cancelled runs. Issue #152 captures this; the task was reverted because the evaluator timed out. The core problem remains: `append_terminal_state_events.py` doesn't distinguish SIGTERM kills from genuine errors.
2. **Seed contradiction detection** — Two recent sessions (Day 155 02:50, Day 154 17:00) patched `preseed_session_plan.py` for this. The Day 155 patch (completion-language fallback) was reverted. The Day 154 patch survived. This is partially addressed but fragile.
3. **Cache metrics from agent runs** — Known (#90). Not fixable within yyds alone without yoagent upstream changes.

## Open Issues Summary
- **#152** (OPEN): Task reverted — Distinguish cancelled runs from error exits in lifecycle terminal events. Reverted due to evaluator timeout. Task is still valid.
- **#105** (OPEN): Task reverted — Record DeepSeek prompt cache metrics during prompt runs. Reverted due to no implementation progress. Blocked on upstream yoagent change.
- **#90** (OPEN): Track `deepseek cache-report` gap — yoagent Usage struct drops cache fields. Upstream dependency.

## Research Findings
No new competitor research conducted. The existing gap vs Claude Code remains: yyds is a harness-building system, not a general-purpose coding agent. The focus is on making the harness reliable, not on adding features.
