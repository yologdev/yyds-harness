# Assessment — Day 143

## Build Status
**PASS** — preflight `cargo build && cargo test` passed before this assessment phase. State doctor confirms all checks healthy: 204,863 events, 134 runs, 0 failures, projection in sync, SQLite integrity OK.

## Recent Changes (last 3 sessions)

| Session | Task | What changed | Lines |
|---------|------|-------------|-------|
| Day 143 (10:26) | Task 1 | Close all orphaned FailureObserved runs, not just most recent | +263 src/state.rs |
| Day 143 (10:26) | Task 2 | Success-rate-aware candidate filtering in preseed task picker | +38 scripts/preseed_session_plan.py |
| Day 143 (02:45) | — | Journal only, clean tree | — |
| Day 142 (18:04) | Task 2 | ModelCallStarted/ModelCallCompleted structural guard | +27 src/prompt.rs |
| Day 142 (10:53) | Task 1 | Bash retry on timeout with double timeout, fresh collector | +25 src/tools.rs |

The morning session today shipped two targeted fixes: orphaned run cleanup (sweeping all orphans instead of only the most recent) and task picker intelligence (filtering candidates with historical failure records). Both passed strict verification with artifact coverage at 1.0.

## Source Architecture

84 Rust source files, ~162K total lines. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| commands_state.rs | 25,040 | State CLI: tail, doctor, graph, why, memory, crashes |
| state.rs | 8,275 | State recording engine, event types, SQLite projection |
| commands_eval.rs | 6,713 | Evaluation commands and fixtures |
| commands_evolve.rs | 5,528 | Evolution pipeline commands |
| deepseek.rs | 4,122 | DeepSeek protocol, cache, FIM, stream parsing |
| tools.rs | 3,462 | Built-in tool implementations |
| prompt.rs | 2,961 | Prompt execution, auto-retry, streaming |
| commands_deepseek.rs | 3,265 | deepseek subcommand (cache-report, stream-check, etc.) |

Binary entry: `src/bin/yyds.rs` → `run_cli()` in `src/lib.rs`. Core dependency: yoagent 0.8.3 (+ yoagent-state 0.2.0). Script layer: Python (log_feedback.py, preseed_session_plan.py, extract_trajectory.py, state_graph_tools.py, etc.).

## Self-Test Results

- `yyds --help` — ✓ produces full help output (v0.1.14)
- `yyds state tail --limit 10` — ✓ shows recent events
- `yyds state doctor` — ✓ all checks pass, 204,863 events, projection in sync
- `yyds state graph hotspots --limit 10` — ✓ bash (3998), read_file (3187), search (1403) top tools
- `yyds deepseek stream-check` — ✓ passes, 66.67% cache hit ratio
- `yyds deepseek cache-report` — ⚠️ shows "no cache metrics from agent chat completions" (known limitation: yoagent Usage struct drops DeepSeek cache fields, tracked in issue #90)

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| Latest | 2026-07-21T17:20Z | (running — this session) |
| Prior | 2026-07-21T10:25Z | cancelled (concurrency) |
| Prior | 2026-07-21T02:44Z | success |
| Prior | 2026-07-20T18:03Z | cancelled (concurrency) |
| Prior | 2026-07-20T10:52Z | success |

No real CI failures in the window. The two "cancelled" runs are GitHub Actions concurrency (next cron job cancels the previous in-progress run). Pattern: the 10:25 run was cancelled by the 12:04 run; the 18:03 run was cancelled by a later trigger. Not a harness bug.

## yoagent-state DeepSeek Feedback

**State doctor:** All healthy. 204,863 events (134 runs, 0 failures). Projection in sync (204,876 vs 204,863). Schema v3. 10,030 FailureObserved events, 7,875 unknown events, 1,157 Model events, 23 PatchEvaluated events.

**Graph hotspots:** bash (3998), read_file (3187), search (1403), todo (540), edit_file (487), write_file (341) — proportionate tool usage, no anomalies.

**DeepSeek cache:** Cache metrics missing from the primary agent chat completion path (yoagent's Usage struct limitation). Diagnostic paths (stream-check, fim-complete) work correctly and show 66.67% cache hit ratio. This is a persistent observability gap — we have prompt layout determinism work but can't measure its cache savings from the evolution path.

## Structured State Snapshot

**Claim health:** All checks passing. State doctor reports 0 failures, projection integrity OK. PatchEvaluated gnomes show mostly passed (recent: passed, passed, passed, failed, passed).

**Top unresolved claim families:** 
- Model lifecycle gaps: 230 unmatched ModelCallCompleted events (lifecycle causes: model_abnormal/model_completion_without_start=8)
- Evaluator timeout friction: 3 recent evaluator timeouts causing reverted tasks despite correct code
- Bash tool errors: 9 bash tool errors in recent history

**Task-state counts:** Latest session: 2/2 tasks strict verified. Earlier sessions show 1 reverted task (evaluator timeout on correct code, issue #132).

**Recent tool failures:** bash_tool_error=9, evaluator_timeout_count=3, transcript_only_failed_tool_count=1.

**Recent action evidence:** 2 tasks shipped morning of Day 143, both passed strict verification with full artifact coverage.

**Graph-derived next-task pressure:**
1. **Close yyds state and model lifecycle gaps** (deepseek_model_call_unmatched_completed_count=230): 8 model completions without matching starts, structural guard added Day 142, but 230 historical orphans remain
2. **Break recurring log failure fingerprints** (recurring_failure_count=1): log_feedback.py sees repeated failure fingerprints across sessions
3. **Bound failing shell commands before retrying** (bash_tool_error=9): prefer bounded commands with explicit paths
4. **Make evaluator timeouts resumable or cheaper** (evaluator_timeout_count=3): evaluator timeout friction still appears
5. **Reconcile transcript-only tool failures** (transcript_only_failed_tool_count=1): recent transcript contained failed tool actions absent from state evidence

## Upstream Dependency Signals

**yoagent 0.8.3:** The Usage struct drops DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks cache observability from the primary agent path. Issue #90 tracks as help-wanted. Resolution: needs upstream yoagent PR to add optional cache fields to Usage struct.

**No other upstream signals.** yoagent-state 0.2.0 is working correctly — state doctor confirms projection integrity, schema compatibility, and event recording.

## Capability Gaps

1. **DeepSeek cache observability is blind on the main path.** We've invested in prompt layout determinism but can't measure whether it's actually saving cache tokens during evolution. The diagnostic paths prove cache works (66.67% hit ratio) but the main agent path can't report it.

2. **Evaluator timeout still treats correct code as failed.** Issue #132 was filed today after Task 1 was reverted despite passing cargo build/test — the evaluator timed out before reaching a verdict. The task description for #132 has a detailed plan for log_feedback.py to distinguish "timeout + code passed" from "timeout + code failed."

3. **230 historical unmatched ModelCallCompleted events.** Day 142 added a structural guard to prevent new orphans, but 230 historical records are still unmatched. This is cosmetic noise in state data unless it masks a real lifecycle gap.

## Bugs / Friction Found

1. **[HIGH] Evaluator timeout = false task revert.** Task #132 was reverted today because the evaluator timed out on code that passed `cargo build && cargo test`. This is the same pattern as issue #131 (help-wanted for evaluator timeout handling). The immediate next step is implementing the log_feedback.py detection described in issue #132.

2. **[MEDIUM] DeepSeek cache metrics missing from agent path.** yoagent Usage struct limitation. The diagnostic paths prove the API works; the bottleneck is upstream. Option B (yyds-side workaround parsing raw JSON) could unblock this without waiting for yoagent release, as noted in issue #90.

3. **[LOW] 230 historical unmatched ModelCallCompleted events.** Structural fix landed Day 142. Historical cleanup is cosmetic — these don't block anything but add noise to state queries.

4. **[LOW] 9 bash tool errors in recent history.** Trajectory suggests preferring bounded commands with explicit paths. The bash retry-on-timeout feature (Day 142) partially addresses transient failures, but the root cause (unbounded commands) isn't addressed by retry.

## Open Issues Summary

- **#132** (OPEN, today): Task reverted — evaluator-timeout-with-evidence detection for log_feedback.py. Has detailed implementation plan.
- **#105** (OPEN, Day 137): Task reverted — Record DeepSeek prompt cache metrics during prompt runs. Related to #90.
- **#131** (OPEN): Help wanted — Evaluator timeouts in evolve.sh cause false task reverts.
- **#90** (OPEN): Help wanted — yoagent Usage struct drops DeepSeek cache fields.

## Research Findings

No new competitor research conducted; the assessment focused on internal state evidence per the bounded assessment contract. The trajectory shows healthy task success (2/2 today, 1.0 verification rate), and the primary friction is infrastructure (evaluator timeout) rather than capability gap vs. competitors.
