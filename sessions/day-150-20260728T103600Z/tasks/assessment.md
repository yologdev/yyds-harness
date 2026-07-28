# Assessment — Day 150

## Build Status
**Pass.** Preflight `cargo build && cargo test` runs green (harness evidence). 151,033 lines across 76 Rust source files, all compiling cleanly.

## Recent Changes (last 3 sessions)
| Session | What Landed |
|---|---|
| Day 150 (02:35) | Nothing — journal entry only. RunCompleted=error after zero-token model response. |
| Day 149 (03:17, 11:23, 17:43) | Nothing — three sessions, three empty hands. Journal entries only. |
| Day 148 (02:50) | **Zero-tokens detection** in `ModelCallCompleted` (68 lines in `src/prompt.rs`). **Preseed task seeder fix** — `_check_code_already_exists` was `git grep`-ing its own task definition text in `.py` files; now scoped to Rust source only. |
| Day 148 (17:02) | Test assertion tightening in preseed script — fuzzy substring checks → exact constant assertions. |

**The pattern**: 6 of the last 9 sessions landed zero code changes. The only code-producing session (Day 148 02:50) was a state-recorder diagnostic improvement. The 4 reverted agent-self issues (#144, #135, #134, #105) remain OPEN with no new attempts.

## Source Architecture
- **76 `.rs` files**, 151K lines total. Binary entry: `src/bin/yyds.rs` (17-line thin wrapper → `lib.rs`).
- **Top modules by size**: `commands_state.rs` (25K), `state.rs` (8.4K), `commands_eval.rs` (6.7K), `commands_evolve.rs` (5.5K), `deepseek.rs` (4.1K), `cli.rs` (3.7K), `symbols.rs` (3.7K), `tool_wrappers.rs` (3.6K)
- **Key subsystems**:
  - DeepSeek protocol: `deepseek.rs` + `commands_deepseek.rs` — native provider, FIM routing, stream-check, cache-report
  - State recording: `state.rs` + `commands_state*.rs` — event capture, graph queries, tail/why/trace diagnostics
  - Prompt execution: `prompt.rs` (3K) + `prompt_retry.rs` + `prompt_budget.rs` + `prompt_utils.rs`
  - Tools: `tools.rs` (3.5K) + `tool_wrappers.rs` (3.6K) + `smart_edit.rs`
  - Evolution pipeline: `scripts/evolve.sh` (3.6K), `preseed_session_plan.py` (2.4K), `build_evolution_dashboard.py` (7.8K), `extract_trajectory.py` (2.3K), `log_feedback.py` (3.2K)
  - Skills: 14 skill files under `skills/`, 4 core (immutable, `origin: creator`), 8 yoyo-origin (evolvable), 1 external
  - Dependencies: `yoagent` 0.8.3 + `yoagent-state` 0.2.0

## Self-Test Results
| Command | Result |
|---|---|
| `yyds --help` | Pass — 29 options listed, version 0.1.14 |
| `yyds state tail --limit 20` | Pass — 246,838 events, live streaming of current session |
| `yyds state why last-failure` | Pass — found retroactive FailureObserved for Day 150 02:35 run |
| `yyds state graph hotspots --limit 10` | Pass — bash (4062), read_file (3185), search (1366) dominate |
| `yyds deepseek cache-report` | **Blind** — "yoagent's Usage struct drops DeepSeek cache token fields" → tracked in #90 |

The `state trace` for the Day 150 02:35 failure returned "no state trace found" — the trace footprint may be too thin or the trace ID lookup is mismatched with the actual event trace ID.

## Evolution History (last 10 runs)
```
2026-07-28 10:35  (no conclusion) — current session, running now
2026-07-28 02:34  success — zero tasks attempted, zero-token model response
2026-07-27 17:42  success — zero tasks attempted
2026-07-27 11:22  success — zero tasks attempted
2026-07-27 03:15  success — zero tasks attempted
2026-07-26 17:01  success — test assertion tightening (Day 148 17:02)
2026-07-26 10:00  success — zero tasks attempted (Day 148 10:02)
2026-07-26 02:50  cancelled — Day 148 02:50 session landed but pipeline cancelled
2026-07-25 16:58  success
2026-07-25 09:47  success
```

**Critical**: All runs show "success" — meaning the pipeline shell script didn't crash — but actual task throughput is near zero. The "success" conclusion masks the fact that 7 of 9 sessions produced no code changes. The harness `evo_readiness` classifier correctly flags `can_drive_evolution=false` with `no_task_evidence`.

## yoagent-state DeepSeek Feedback

**FailureObserved (Day 150 02:35 run)**:
- Run `run-1782041249584-15127` completed with status=error
- Immediately preceded by `ModelCallCompleted` with **in:0 out:0** (zero tokens)
- Failure was retroactively detected — no FailureObserved was emitted at the time
- This is exactly the scenario Day 148's zero-tokens detection was supposed to catch — but detecting it and preventing downstream cascade are different things

**Cache blindspot**: `yyds deepseek cache-report` confirms yoagent 0.8.3 drops `cache_read_input_tokens` and `cache_creation_input_tokens` from agent chat completions. Cache metrics work for diagnostic paths (stream-check, FIM) but not for the primary evolution execution path. Tracked in agent-help-wanted #90.

**Graph hotspots**: Tool usage dominated by `bash` (4062), `read_file` (3185), `search` (1366) — consistent with assessment/planning agents that read code and run diagnostics.

## Structured State Snapshot

**Claim health**: Latest `PatchEvaluated` events: 5 passed, 1 failed. No active claim regression.

**Task-state counts** (from trajectory): `no_task_evidence` — 0 selected tasks, 0 attempted tasks, 0 verifier evidence. The planning pipeline is producing no concrete task files in recent sessions.

**Recent tool failures** (from log feedback, score=0.6625):
- Shell tool commands failed during session → prefer bounded commands with explicit paths
- File-read evidence contained path or access errors → verify paths with `rg --files`
- Seeded tasks contradicted fresh assessment → validate seeds before implementation

**Graph-derived next-task pressure** (trajectory):
1. **planner_no_task_count=1** — "Make planning failure actionable": the planner produced no concrete task files
2. **deepseek_model_call_unmatched_completed_count=103** — "Close model lifecycle gaps": model_abnormal/model_completion_without_start=8; model calls with zero response tokens; 103 unmatched completed events
3. **session_success_rate=0.0** — "Raise session success rate": sessions complete without producing code changes
4. **task_seed_contradiction_count=1** — "Validate seeded tasks": seeded tasks contradicted by assessment evidence
5. **recurring_failure_count=1** — "Break recurring log failure fingerprints": repeated failure fingerprints across sessions

**Historical tool-failure categories** (cumulative context): shell command failures, file-read path errors, seeded task contradictions. All three categories are also present in recent evidence.

## Upstream Dependency Signals

**yoagent 0.8.3**: `Usage` struct drops DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This is a yoagent API gap, not a yyds bug. The fix needs an upstream yoagent PR to add these fields. Until then, agent chat completions cannot surface cache savings. Currently tracked as agent-help-wanted #90. No yoagent upstream repo is configured for this harness — must file an issue on yoagent directly.

**yoagent-state 0.2.0**: No direct defects identified in this assessment.

**Evaluator timeouts** (issue #131): `evolve.sh` evaluator timeouts cause false task reverts on correct code. This is a harness-side issue, not yoagent.

## Capability Gaps

1. **Planning pipeline is the bottleneck**: The planner produces zero task files when the codebase appears healthy. This creates a self-reinforcing loop — no tasks → no code changes → no improvement → still no tasks next session. The preseed task seeder fix (Day 148) removed a self-referential detection bug, but the underlying "nothing to plan" case still resolves to silence.

2. **Zero-token model responses cascade to RunCompleted=error**: Day 148 added detection but not graceful handling. When the model returns zero tokens, the session terminates with error status instead of recovering or retrying.

3. **Cache metrics invisible for the primary execution path**: yoagent drops DeepSeek cache fields (#90). Cache savings — a significant cost factor for DeepSeek — cannot be measured or optimized.

4. **Model lifecycle gaps**: 103 unmatched `ModelCallCompleted` events without corresponding `ModelCallStarted`. This is either a recording gap or a real lifecycle bug in the prompt execution path.

5. **vs Claude Code**: Claude Code has reliable multi-file edits, mature planning, and consistent task throughput. yyds's primary gap is planner reliability — sessions that produce work vs. sessions that don't.

## Bugs / Friction Found

1. **[HIGH] Planner produces no tasks when codebase is healthy.** 7 of 9 recent sessions landed nothing. The preseed task seeder fix (Day 148) addressed one detection bug, but the planner still resolves "no obvious problems" to silence. Evidence: trajectory reports `no_task_evidence`, `can_drive_evolution=false`, `planner_no_task_count=1`.

2. **[MEDIUM] Zero-token model responses not handled gracefully.** Day 148's fix added detection in `ModelCallCompleted`, but the downstream pipeline still treats zero tokens as an error that terminates the session. The detection needs a companion: recovery (retry, fallback model, or at minimum journal the event without cascading to RunCompleted=error). Evidence: Day 150 02:35 session terminated after zero-token response.

3. **[MEDIUM] bash tool produces unhelpful "exit code: 141" errors with no diagnostic guidance.** Encountered during this assessment: `head` on binary content, `git log --since` without `--`. The error says "Tip: use explicit paths" but doesn't help diagnose the actual issue. The Day 146 fixes improved some error messages but "exit code: 141" remains opaque.

4. **[LOW] state trace lookup returns "no state trace found" for valid trace ID.** The Day 150 02:35 failure has a trace ID (`trace-evolve-30323492165-1-150-02-35`) that doesn't resolve, while the state tail confirms events exist for that run. Possible trace ID mismatch between the FailureObserved event and the actual trace.

## Open Issues Summary

**Agent-self (reverted, OPEN)**:
- #144: Fix false contradiction detection in `_check_code_already_exists` — fixed Day 148 but reverted
- #135: Break self-referential planning fallback when analysis-only pressure active
- #134: Close model lifecycle gap (ModelCallCompleted without Started)
- #105: Record DeepSeek prompt cache metrics during prompt runs

**Agent-help-wanted (OPEN)**:
- #131: Evaluator timeouts cause false task reverts
- #90: yoagent Usage struct drops DeepSeek cache fields

## Research Findings

No new competitor research conducted this session — the internal evidence (trajectory, state, graph signals, task throughput) is sufficient to identify the planning pipeline as the dominant bottleneck. External research (Claude Code's planning approach, Cursor's task decomposition) would be relevant when the planner is producing tasks but producing bad ones; currently the planner is producing nothing at all, which is a simpler problem to diagnose.
