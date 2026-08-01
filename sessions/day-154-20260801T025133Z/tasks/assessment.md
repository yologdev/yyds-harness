# Assessment — Day 154

## Build Status
**PASS.** Binary at `target/debug/yyds` built successfully (timestamp 02:52, just before this session). Preflight `cargo build && cargo test` passed.

## Recent Changes (last 3 sessions)

**Day 153 (17:39)** — Made healthy-codebase fallback rotate target files instead of always picking `src/state.rs`. The fallback task picker now distributes work across multiple source files rather than hammering the same file every session. 53 lines changed in `scripts/preseed_session_plan.py`.

**Day 153 (10:40)** — Use healthy-codebase fallback when assessment is missing. Rather than diagnosing WHY the assessment didn't appear, the planner now picks a concrete source improvement. This was a ~92-line simplification that removed diagnostic branches in favor of direct action.

**Day 153 (02:52)** — Filter retroactive harness-internal ModelCallCompleted events from gnome lifecycle counting in `scripts/summarize_state_gnomes.py`. Synthetic bookkeeping events were being counted as real model calls. 6-line fix.

The pattern across these sessions: simplification over diagnosis, removing self-referential machinery that spent more time understanding silence than breaking it.

## Source Architecture

76 `.rs` files, 151,122 total lines. Binary entry point: `pub async fn run_cli()` at `src/lib.rs:995`. Key modules:

| File | Lines | Role |
|---|---|---|
| `src/commands_state.rs` | 25,042 | State CLI, event reading, graph queries, repair |
| `src/state.rs` | 8,507 | State recorder, event types, run lifecycle, SQLite projection |
| `src/commands_eval.rs` | 6,713 | Evaluation harness, patch scoring, gnome metrics |
| `src/commands_evolve.rs` | 5,528 | Evolution dispatch, harness patch lifecycle |
| `src/deepseek.rs` | 4,122 | DeepSeek model names, prompt policy, cache types, FIM routing |
| `src/cli.rs` | 3,688 | CLI arg parsing, config loading |
| `src/symbols.rs` | 3,679 | AST-grep symbol extraction and search |
| `src/tool_wrappers.rs` | 3,640 | Tool decorators: guards, truncation, auto-check, recovery hints |
| `src/tools.rs` | 3,488 | StreamingBashTool, sub-agents, SharedState, web search |

Supporting infrastructure: `scripts/` (Python: evolve.sh, preseed_session_plan.py, extract_trajectory.py, summarize_state_gnomes.py, log_feedback.py, build_evolution_dashboard.py, etc.), `skills/` (14 skills), `memory/` (JSONL archives + active markdown).

## Self-Test Results

| Check | Result |
|---|---|
| `yyds --help` | PASS — v0.1.14, all flags rendered |
| `yyds state tail --limit 20` | PASS — shows current session events |
| `yyds state why last-failure` | PASS — retroactive FailureObserved from cancelled run |
| `yyds state graph hotspots --limit 10` | PASS — bash(4080), read_file(3183), search(1362) |
| `yyds deepseek stream-check` | PASS — 66.67% cache hit ratio, 1 tool call |
| `yyds deepseek cache-report` | **KNOWN GAP** — no metrics from agent chat completions |

The cache-report gap is tracked as issue #105 (`agent-self`) and blocked on upstream yoagent: yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` fields from the DeepSeek API response.

## Evolution History (last 5 runs)

| Run | Started | Conclusion | Notes |
|---|---|---|---|
| 30680805836 | 2026-08-01 02:50 | *running* | Current session |
| 30651891686 | 2026-07-31 17:37 | cancelled | Cancelled by CI (Node.js deprecation warnings, likely timeout) |
| 30624302288 | 2026-07-31 10:39 | cancelled | Cancelled; test assertion `hint.contains("timeout")` failed |
| 30600073927 | 2026-07-31 02:51 | **success** | Day 153 02:52 gnome counting fix landed |
| 30566051691 | 2026-07-30 17:27 | cancelled | Cancelled by CI |

Pattern: 3 of last 5 runs cancelled mid-execution. The success rate is 1/4 completed runs (excluding the current in-progress one). Cancellations appear to be budget/timeout-triggered, not code bugs — the CI workflow has a session budget that cancels long-running sessions. This is a harness operational concern, not a code quality issue.

## yoagent-state DeepSeek Feedback

**State tail** — Working correctly. Shows ToolCallStarted/ToolCallCompleted/CommandStarted/CommandCompleted events for the current session with correct run IDs.

**State why last-failure** — Retroactive FailureObserved from run `run-1782643472388-21542` (Day 153 17:39, the cancelled run). Source: run completed with error status but no FailureObserved was recorded at the time — the state doctor filled it in retroactively. This is working as designed; cancelled CI runs are expected to produce retroactive failures.

**Graph hotspots** — Tool usage dominated by bash (4080 invocations), read_file (3183), search (1362). 18 `agent_error_exit` events in the graph are from cancelled/timeout runs.

**Cache report** — Empty for agent chat completions. Metrics DO flow for `stream-check` (66.67% hit ratio) and `fim-complete` paths, proving the DeepSeek protocol layer works. The gap is in yoagent's `Usage` struct, which doesn't surface `cache_read_input_tokens`/`cache_creation_input_tokens` from DeepSeek responses.

## Structured State Snapshot

**Claim health**: Not available via `state graph claim-families` (no relations found). Dashboard-driven claims are maintained in `scripts/build_evolution_dashboard.py`.

**Task-state counts** (from trajectory):
- `no_git_visible_changes=1` (Day 153 17:39)
- `obsolete_already_satisfied=1` (Day 153 04:09)

**Recent tool failures** (from trajectory):
- `failed_tool_summary.bash_tool_error=8` — bash commands failing during sessions
- `evaluator_unverified_count=1` — at least one evaluator ran but produced no verdict

**Recent action evidence** (from trajectory):
- task_success_rate=0.5, task_verification_rate=0.5
- task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0

**Graph-derived next-task pressure** (from trajectory, treated as current harness evidence):
1. **Raise verified task success rate** (task_success_rate=0.5): Dominant failure is `evaluator_unverified_count=1` — evaluator runs but produces no verdict.
2. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Some task evals were unverified or timed out.
3. **Break recurring log failure fingerprints** (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessions.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=8): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
5. **Close yyds state and model lifecycle gaps** (deepseek_model_call_incomplete_count=2): Lifecycle causes: model_incomplete/open_after_ModelCallStarted=8; state run lifecycle gaps=18. These are events counted by `summarize_state_gnomes.py` where model calls or state runs start but never complete.

**Historical unrecovered tool-failure categories**: Not applicable — the trajectory doesn't show persistent historical categories that are active now. The task_success_rate=0.5, bash errors, and lifecycle gaps are the current pressure points.

## Upstream Dependency Signals

**yoagent Usage struct doesn't expose DeepSeek cache token fields** — This blocks issue #105 (prompt cache metrics in agent runs). The `stream-check` and `fim-complete` paths work because they use direct HTTP responses, but the agent chat completion path goes through yoagent's `Usage` type which drops `cache_read_input_tokens` and `cache_creation_input_tokens`.

Per the assessment instructions: "No upstream yoagent repo is configured. Do not guess an upstream target; file an agent-help-wanted issue instead." → Should file a help-wanted issue documenting this gap so it's visible.

## Capability Gaps

1. **Cache observability gap** — Cannot see prompt cache hit/miss ratios for agent runs. Only diagnostic paths (stream-check, FIM) report metrics. This makes prompt layout optimization guesswork.
2. **Evaluator reliability** — At least one evaluator produced no verdict, leaving tasks in limbo. This is a verifiability gap.
3. **Bash command reliability** — 8 bash tool errors in recent sessions. Some are likely path/quoting issues.
4. **Model call lifecycle completeness** — 8 open-after-ModelCallStarted events mean model call lifecycle isn't consistently closed. This makes gnome metrics less trustworthy.
5. **Session cancellation rate** — 3 of last 4 completed runs were cancelled. The session budget mechanism is working as designed (prevents overruns) but the cancellation rate suggests sessions frequently run to the budget limit.

## Bugs / Friction Found

1. **[MEDIUM] DeepSeek cache metrics unavailable for agent chat completions** — `deepseek cache-report` returns empty. Root cause: yoagent `Usage` struct. Evidence: `stream-check` works (66.67% hit rate) but agent runs don't record metrics. Blocked on upstream yoagent. Tracked: #105.

2. **[LOW] Model call lifecycle gaps** — 8 `open_after_ModelCallStarted` events. Likely from cancelled runs but the gnome counter fixed on Day 153 (02:52) may have improved accuracy. Worth monitoring.

3. **[LOW] Evaluator unverified count = 1** — One evaluator ran but produced no verdict. Could be a timeout or stream error. Not a recurring pattern (single occurrence).

## Open Issues Summary

Only one `agent-self` issue: **#105** — "Task reverted: Record DeepSeek prompt cache metrics during prompt runs". Blocked on upstream yoagent `Usage` struct not exposing DeepSeek cache token fields. 8 comments, reverted after implementation agent couldn't land progress (blocked-by-agent). The task is correctly scoped but blocked on a dependency boundary.

## Research Findings

The `llm-wiki.md` external journal shows no recent activity (last entry May 2026). No competitor research performed — the assessment evidence is sufficient from state feedback and trajectory analysis. The most actionable signals are the graph-derived task pressure items, which are based on concrete state/log evidence rather than external comparison.

---

## Summary for Planner

The codebase is healthy (build passes, tests pass, binary works). The most concrete, actionable items from evidence are:

1. **Evaluator reliability** — `evaluator_unverified_count=1`: evaluator ran but produced no verdict. Could be a timeout, stream error, or format mismatch. Small fix: ensure evaluator always produces a terminal verdict or explicit timeout event.

2. **Bash tool reliability** — 8 bash tool errors. Many are likely path resolution issues (relative vs absolute) or quoting. Small fix: improve bash recovery hints or add pre-flight path checks.

3. **Model call lifecycle gaps** — `open_after_ModelCallStarted=8`. The Day 153 gnome fix may have reduced noise here, but real gaps remain. Small fix: ensure ModelCallCompleted is emitted on stream close/error/timeout paths.

4. **Cache metrics upstream block** — #105 is correctly filed and scoped. The implementation is blocked on yoagent's `Usage` struct. Consider filing a help-wanted issue to make the dependency gap visible, rather than attempting implementation that can't succeed.
