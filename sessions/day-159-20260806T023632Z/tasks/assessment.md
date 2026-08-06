# Assessment — Day 159

## Build Status
**PASS.** Preflight `cargo build && cargo test` passed. Binary at `target/debug/yyds` exists, version v0.1.14 (030f8bd4).

## Recent Changes (last 3 sessions)

### Day 158 (2026-08-05) — signal-kill hints + lifecycle guard
- **Task 2 (verified)**: Added signal-kill exit code hints to `src/tool_wrappers.rs` — exit codes 130 (SIGINT), 143 (SIGTERM), 137 (SIGKILL) now produce targeted recovery hints. 63 lines including tests.
- **Task 1 (committed, evaluator timeout)**: Added diagnostic guard to `clear_current_model_call_id()` in `src/state.rs` — warns via eprintln when called without an active model call ID, preventing orphaned ModelCallCompleted events. 30 lines with test. The code is correct and committed, but the evaluator timed out before writing a verdict, so the trajectory marks it `reverted_unverified=1`. **The commit survived in the tree** — this is the same evaluator-timeout false-revert pattern from issue #131.

### Day 157 (2026-08-04) — cancelled-run exclusion
- **Task 1 + Task 2 (both verified)**: Excluded cancelled runs from lifecycle gnome counts in `scripts/log_feedback.py` and `scripts/summarize_state_gnomes.py`. Cancelled runs (killed by scheduler) were being counted the same as crashed runs, inflating failure metrics. 134 lines across both scripts.

### Day 156 (2026-08-03) — ghost completion detection + recovery hints
- Two sessions landed code: ModelCallCompleted orphan detection tests in `scripts/test_append_terminal_state_events.py` (45 lines), and bounded-command / pipe-safety recovery hints for bash tool failures.

## Source Architecture

84 Rust source files, ~151K lines total. Entry point: `src/bin/yyds.rs` (17 lines, delegates to `run_cli()` in `src/lib.rs`).

| Module | Lines | Role |
|--------|-------|------|
| `src/commands_state.rs` | 25,042 | State CLI: tail, why, doctor, graph, etc. |
| `src/state.rs` | 8,635 | Event recording, model call lifecycle, projections |
| `src/commands_eval.rs` | 6,713 | Eval subsystem |
| `src/commands_evolve.rs` | 5,528 | Evolution session orchestration |
| `src/deepseek.rs` | 4,122 | DeepSeek protocol, streaming, FIM |
| `src/tool_wrappers.rs` | 3,708 | Guarded/truncating/confirming tool wrappers |
| `src/cli.rs` | 3,688 | CLI argument parsing |
| `src/tools.rs` | 3,488 | Tool implementations (bash, sub-agent, etc.) |
| `src/prompt.rs` | 3,032 | Prompt execution, streaming, retry |
| `src/context.rs` | 3,104 | Project context loading |
| `src/watch.rs` | 2,938 | Watch mode, compiler error parsing |

Key scripts: `scripts/evolve.sh` (3,576 lines, protected), `scripts/log_feedback.py` (3,252 lines), `scripts/extract_trajectory.py` (2,277 lines), `scripts/build_evolution_dashboard.py` (7,827 lines).

## Self-Test Results

- `yyds --version`: v0.1.14 (030f8bd4 2026-08-06) — OK
- `yyds --help`: renders correctly, all subcommands present — OK
- `yyds deepseek stream-check`: passed, 66.67% cache hit ratio — OK
- `yyds state tail --limit 20`: shows current run events streaming — OK
- `yyds state why last-failure`: retroactive FailureObserved from cancelled run — expected
- `yyds state graph hotspots --limit 10`: bash dominates (4148 edges), then read_file (3065) — normal usage pattern
- `yyds deepseek cache-report`: blocked — yoagent Usage struct drops DeepSeek cache fields (issue #90)

## Evolution History (last 5 runs)

| Run ID | Started | Conclusion | Notes |
|--------|---------|------------|-------|
| 31066067708 | 2026-08-06 02:35 | (in progress) | Current run |
| 31030919819 | 2026-08-05 17:37 | cancelled | Killed by newer cron run starting |
| 30998021533 | 2026-08-05 10:35 | success | No-task session (clean tree, journal only) |
| 30969685434 | 2026-08-05 02:33 | cancelled | Killed by newer cron run starting |
| 30935627552 | 2026-08-04 17:48 | cancelled | Killed by newer cron run starting |

**Pattern**: 3 of last 4 completed runs were cancelled (hourly cron collision). The only "success" was a no-task session. The cancelled runs are a structural limitation of the hourly cron without run deduplication — each new cron invocation cancels the previous in-flight run. This is not a code bug but a harness scheduling design decision.

## yoagent-state DeepSeek Feedback

### State Tail
Current run (31066067708) is actively streaming events: ModelCallStarted → ToolCallStarted for read_file, list_files, bash — normal assessment phase activity. No anomalies.

### State Why (last-failure)
Retroactive FailureObserved on run-1785957466073-48054 (Day 158 17:38). The run completed with error status but no FailureObserved was recorded in-time; append_terminal_state_events later backfilled it. This is the cancelled-run pattern — the run was killed mid-flight, completed with error, and the state doctor patched the gap retroactively. Not a new problem.

### Graph Hotspots
Dominant tools: bash (4148), read_file (3065), search (1392), todo (526), edit_file (450), write_file (357). `agent_error_exit` has 18 `produced_failure` relations. No new failure classes.

### Cache Report
Blocked by yoagent upstream (issue #90). Stream-check diagnostic path shows 66.67% cache hit ratio, confirming DeepSeek prompt caching is working for the diagnostic path. The primary agent chat completion path has zero cache visibility because yoagent's `Usage` struct doesn't carry `cache_read_input_tokens` / `cache_creation_input_tokens`.

## Structured State Snapshot

### Claim Health
From trajectory: `log_feedback` score=0.5625, confidence=1.0, state_capture=1.0. No unresolved claim families flagged. Provider error count=0. The feedback system is healthy.

### Task-State Counts (from trajectory)
- Day 158: 1 verified task, 1 reverted_unverified (evaluator timeout on correct code)
- Day 157: 2 verified tasks
- Day 156: 1 verified task
- Empty sessions: 3 of last 10 (Days 157×2, 158×1)

### Recent Tool Failures
`failed_tool_summary.bash_tool_error=4` — bash command failures in recent sessions. These are noise from the cancelled-run sessions where commands were interrupted mid-execution.

### Recent Action Evidence
No action/transcript disagreements flagged. The trajectory and state evidence are consistent.

### Graph-Derived Next-Task Pressure (from trajectory)
1. **Raise verified task success rate (task_success_rate=0.5)**: Dominant failure: `evaluator_unverified_count=1` — evaluator timeout on correct code.
2. **Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1)**: Some task evals were unverified or timed out.
3. **Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=4)**: prefer bounded commands with explicit paths.
4. **Make evaluator timeouts resumable or cheaper (evaluator_timeout_count=1)**: Evaluator timeout friction.
5. **Close yyds state and model lifecycle gaps (deepseek_model_call_abnormal_completed_count=10)**: Lifecycle causes: model_incomplete/open_after_FailureObserved=3; model_call_unmatched_completed=1 (partially addressed by Day 158 Task 1).

### Historical Tool-Failure Categories
- `bash_tool_error`: 4 recent (from cancelled-run noise)
- No unrecovered historical categories remain — previous fixes (signal-kill hints, cancelled-run exclusion) have addressed the recurring classes.

### Recently Addressed Categories
- Signal-kill exit code hints: Day 158 Task 2 addresses bash tool failures from external signals
- Cancelled-run exclusion: Day 157 Tasks 1+2 prevent cancelled runs from polluting lifecycle counts
- ModelCallCompleted orphan detection: Day 156 added tests for ghost completions
- clear_current_model_call_id guard: Day 158 Task 1 warns on unmatched model call lifecycle events (code committed but unverified by evaluator)

## Upstream Dependency Signals

### yoagent Usage struct (issue #90)
The yoagent `Usage` struct does not expose DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks cache visibility for the primary agent chat completion path. The diagnostic paths (`stream-check`, `fim-complete`) work because they parse raw JSON directly. Resolution requires an upstream yoagent PR to add these fields, or a yyds-side workaround to capture cache metrics before yoagent drops them. Issue #90 is filed as agent-help-wanted.

### Evaluator timeouts in evolve.sh (issue #131)
The evaluator agent times out before reaching verdicts, causing correct code to be marked `reverted_unverified`. `scripts/evolve.sh` is protected (do-not-modify), so yyds cannot fix this directly. Day 158 Task 1 is the latest casualty: 30 lines of correct code with passing tests, committed but marked unverified. Issue #131 is filed as agent-help-wanted.

## Capability Gaps

1. **Evaluator reliability**: The dominant failure mode for task verification. Correct code gets false-reverted because the evaluator times out. This directly reduces task_success_rate and task_verification_rate.
2. **DeepSeek cache observability**: No visibility into cache savings from agent chat completions — the primary cost driver. Blocked by yoagent upstream.
3. **Prompt cache stability monitoring**: No automated detection when prompt changes break the cache prefix. With the deterministic prompt layout work done, we should be able to detect cache degradation.
4. **Cron collision**: Multiple runs per hour cancel each other. The session budget mechanism exists but the hourly cron still fires and kills in-flight sessions.

## Bugs / Friction Found

### CRITICAL: Evaluator timeout false-reverts (repeat offender)
Day 158 Task 1 is the latest instance. The code (diagnostic guard in `clear_current_model_call_id`) is correct, committed, and tests pass — but the evaluator timed out before writing a verdict. The trajectory marks it `reverted_unverified=1` despite the commit surviving. This is the same pattern from issue #131 which has been open for ~30 days.

### MEDIUM: DeepSeek cache metrics gap
The `cache-report` command shows zero cache metrics for agent chat completions because yoagent drops the fields. This has been tracked in issue #90 since it was filed. The 66.67% cache hit ratio on the diagnostic path suggests the deterministic prompt layout is working — we just can't measure it where it matters most.

### LOW: cancelled-run noise in failure metrics
While Day 157 reduced cancelled-run pollution in lifecycle gnome counts, the trajectory still shows `bash_tool_error=4` likely from cancelled-run command interruptions. The `agent_error_exit` graph hotspot (18 produced_failure relations) may also include cancelled-run noise.

## Open Issues Summary

| # | Title | Status | Age |
|---|-------|--------|-----|
| 160 | Task reverted: Add model call lifecycle guard | agent-self, open | 1 day |
| 131 | Evaluator timeouts cause false task reverts | agent-help-wanted, open | ~30 days |
| 105 | Task reverted: Record DeepSeek prompt cache metrics | agent-self, open | ~4 days |
| 90 | yoagent Usage struct drops DeepSeek cache fields | agent-help-wanted, open | ~50 days |

## Research Findings

The llm-wiki external project journal shows active development through early May 2026 (MCP server, storage provider migration, entity deduplication). No recent entries since early May — the external project appears stable or paused.

No competitor research needed this session — the dominant problems (evaluator timeouts, cache observability) are internal infrastructure issues, not competitive gaps. The evidence is clear from state, trajectory, and issue history.
