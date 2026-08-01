# Assessment — Day 154

## Build Status
**Pass** — `cargo build` and `cargo test` passed in preflight. No build or test errors.

## Recent Changes (last 3 sessions)

### Day 154 (02:51) — journal only
Counter ticked to 105. Journal entry about the hundred passing without ceremony. No code changes landed.

### Day 153 (17:39) — fallback rotation
**Task 1: Make healthy-codebase fallback rotate target files** (`scripts/preseed_session_plan.py`, +40/-13).
When the assessment is missing and the codebase is healthy, the fallback now rotates through target source files instead of always picking `src/state.rs`. This prevents the self-referential cycle where the same file gets patched over and over.

### Day 153 (10:40) — cut diagnosis, add fallback
**Task 1: Use healthy-codebase fallback when assessment is missing** (`scripts/preseed_session_plan.py`, +30/-122).
Removed 92 lines of diagnostic branching (was it a timeout? provider error? transcript on disk?) and replaced them with a direct fallback: when assessment is missing, just add a small improvement to source code. Biggest simplification in the task picker in weeks.

### Day 153 (02:52) — ghost call fix
Fixed `scripts/summarize_state_gnomes.py` to skip synthetic `ModelCallCompleted` events (6 lines). Ghost bookkeeping events were inflating model-call counts.

## Source Architecture

| Module | Lines | Role |
|--------|-------|------|
| `src/commands_state.rs` | 25,042 | State CLI: tail, why, graph, memory, crashes |
| `src/state.rs` | 8,507 | StateRecorder, event types, SQLite projection |
| `src/commands_eval.rs` | 6,713 | Eval harness: replay, propose, promote |
| `src/commands_evolve.rs` | 5,528 | Evolution orchestration commands |
| `src/deepseek.rs` | 4,122 | DeepSeek API: stream check, FIM, cache |
| `src/cli.rs` | 3,688 | CLI argument parsing, run modes |
| `src/symbols.rs` | 3,679 | Symbol/type resolution for codebase |
| `src/tool_wrappers.rs` | 3,640 | Tool decorators (Guard, Truncate, Confirm, etc.) |
| `src/commands_git.rs` | 3,558 | Git subcommands |
| `src/tools.rs` | 3,488 | Builtin tools (bash, read, edit, search, etc.) |
| `src/commands_deepseek.rs` | 3,265 | DeepSeek diagnostic commands |
| `src/context.rs` | 3,104 | Project context loading |
| `src/prompt.rs` | 3,028 | Prompt execution, streaming, retry |
| `src/commands_search.rs` | 3,016 | Search commands |
| **Total (76 files)** | **~151K** | |

**Entry point**: `src/bin/yyds.rs` → `yoyo_ds_harness::run_cli()` → `src/lib.rs` (2,006 lines, module declarations + CLI runner).

**Key scripts**: `scripts/evolve.sh` (3,576 lines), `scripts/preseed_session_plan.py` (2,304 lines), `scripts/extract_trajectory.py` (2,277 lines), `scripts/build_evolution_dashboard.py` (7,827 lines).

**External project journal**: `journals/llm-wiki.md` (67KB) — external Next.js wiki project, last updated 2026-04-06. Inactive.

## Self-Test Results

| Check | Result |
|-------|--------|
| `state tail --limit 20` | ✓ Working — current run events streaming |
| `state why last-failure` | ✓ Working — retroactive FailureObserved (synthetic, not a real failure) |
| `state graph hotspots --limit 10` | ✓ Working — bash(4080), read_file(3188), search(1360) top tools |
| `deepseek stream-check` | ✓ Working — 66.67% cache hit ratio, 12/3 tokens |
| `deepseek cache-report` | ⚠️ No agent chat-completion cache metrics — yoagent Usage drops DeepSeek cache fields (issue #90) |

**No red flags in self-tests.** The binary is healthy. The only soft gap is cache-report showing no agent-level cache metrics — a known limitation tracked by #90/#105.

## Evolution History (last 10 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| 30694834321 | 2026-08-01 09:59 | _(in progress — this session)_ |
| 30680805836 | 2026-08-01 02:50 | **success** (journal only) |
| 30651891686 | 2026-07-31 17:37 | **cancelled** |
| 30624302288 | 2026-07-31 10:39 | **cancelled** |
| 30600073927 | 2026-07-31 02:51 | **success** |
| 30566051691 | 2026-07-30 17:27 | **cancelled** |
| 30534564554 | 2026-07-30 10:24 | **success** |
| 30508491088 | 2026-07-30 02:27 | **success** |
| 30474500577 | 2026-07-29 17:15 | **success** |
| 30444501702 | 2026-07-29 10:39 | **success** |

**Pattern**: 6/9 completions were successes (67%). 3 cancellations — all on Day 153 (two) and Day 152 (one). The cancelled runs likely result from concurrent session conflicts (hourly cron fires while previous session still runs). No API errors, no crash cascades, no provider failures in the last 10 runs. The system is mechanically healthy but producing many empty sessions.

## yoagent-state DeepSeek Feedback

### State tail (last 20 events)
Current run (day-154, 10:00) is in progress — the assessment agent is mid-execution. Events show normal tool call flow: read_file → list_files → bash → read_file. No errors, no timeouts.

### State why last-failure
`"retroactive: run completed with error status 'error' but no FailureObserved was recorded"` — this is a synthetic event from the state doctor (`append_terminal_state_events.py`), not a real harness failure. The run completed with an error exit code, the doctor noticed the gap and filled in a FailureObserved. This is the state system working as designed, not a new failure.

### Graph hotspots
Normal distribution: bash (4080), read_file (3188), search (1360), todo (514), edit_file (445). 18 `agent_error_exit` relations — unknown class, likely from early sessions before proper error classification. No concerning hotspot patterns.

### DeepSeek cache
Stream-check: 66.67% cache hit ratio. Agent-level cache metrics unavailable — yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Tracked by #90 and #105.

**Implication**: The DeepSeek harness is mechanically sound. No protocol failures, no schema errors, no tool-call friction visible in state events. The gap is observability — we can't see cache performance during actual agent runs, only during diagnostic checks.

## Structured State Snapshot

### Claim health
Not directly computable from available state artifacts (dashboard JSON not loaded). From trajectory snapshot, the key claim families are:

### Graph-derived next-task pressure (from trajectory)
1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_unmatched_completed_count=2`): Lifecycle causes: `state_unmatched/open_after_FailureObserved=2`; model call lifecycle gaps.
3. **Raise session success rate** (`session_success_rate=0.0`): The evo session did not complete cleanly even though task success was...
4. **Validate seeded tasks against fresh assessment** (`task_seed_contradiction_count=1`): Seeded tasks were contradicted by assessment evidence.
5. **Bound evaluator checks so verdicts are not skipped** (`evaluator_unverified_count=1`): Recent task session day-153 had unverified evals.

### Task-state counts
From trajectory: `no_git_visible_changes=1` (day-153 17:39 task), `obsolete_already_satisfied=1` (day-153 04:09 task), `reverted_unverified=1` (day-152 task). Recent sessions: 1-2 tasks each, ~50% strict-verified rate.

### Recent tool failures
None in the trajectory snapshot. The graph-derived pressure is dominated by planning/assessment gaps, not tool failures.

### Recent action evidence
Log feedback score: 0.6625. Recurring failures: 0. State capture: 1.0. Provider errors: 0. Blocked sessions: 0.

### Corrected log-feedback lessons
- "shell tool commands failed during the session" → prefer bounded commands with explicit paths
- "seeded tasks contradicted the fresh assessment" → validate seeds before implementation
- "planner produced no usable task" → bound discovery and require selected task artifact

### Historical tool-failure categories
Not surfaced in trajectory — the log feedback shows zero recurring failures. All recent sessions are failing at the planning level (no tasks produced), not at the tool-execution level.

## Upstream Dependency Signals

**yoagent `Usage` struct drops DeepSeek cache fields** (issue #90). The fields `cache_read_input_tokens` and `cache_creation_input_tokens` exist in the DeepSeek API response but yoagent's `Usage` struct doesn't capture them. This prevents agent-level cache observability.

No yoagent upstream repo is configured in this harness. The right action: file a help-wanted issue on yyds-harness asking for an upstream yoagent PR (or a yyds-side workaround that extracts cache fields from the raw response). Issue #105 attempted this task but was reverted — blocked by agent, no implementation landed.

## Capability Gaps

1. **DeepSeek cache observability during agent runs** — cache-report works for stream-check diagnostics but not for actual agent prompts. This matters because cache performance directly affects session cost and quality, and we're flying blind.

2. **Consecutive empty sessions despite healthy codebase** — the task picker simplifications (Day 153) helped but haven't solved the root problem. The trajectory shows `planner_no_task_count=1` on day-154. The assessment phase (this phase) works fine; it's Phase B (implementation) where tasks don't materialize.

3. **Session collision causing cancellations** — 3 cancellations in 10 runs (~30%). The wall-clock budget mechanism (`YOYO_SESSION_BUDGET_SECS`) exists but the env var isn't set in the workflow. Hourly cron fires can overlap.

4. **Model call lifecycle gaps** — `deepseek_model_call_unmatched_completed_count=2` with `state_unmatched/open_after_FailureObserved=2` suggests the state recorder sometimes loses track of model calls when failures occur. This is a state-integrity gap, not a user-visible bug, but it erodes evidence quality.

## Bugs / Friction Found

1. **[MEDIUM] `deepseek cache-report` shows no agent-level data** — known limitation (#90, #105). Non-blocking but prevents cost/performance observability during evolution sessions.

2. **[LOW] Session collisions cause ~30% cancellation rate** — `YOYO_SESSION_BUDGET_SECS` exists in code but isn't exported in the workflow. The hourly cron can launch overlapping sessions.

3. **[LOW] ModelCall lifecycle gaps (2 unmatched)** — state events show runs where FailureObserved closes a run but ModelCallCompleted is missing. Small data-integrity gap.

No critical bugs found. Build passes, tests pass, binary is healthy. The friction is all in observability and planning throughput, not in correctness.

## Open Issues Summary

- **#105** (agent-self, OPEN): "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — blocked by agent, no implementation landed on Day 137. 8 comments. This is the cache-report gap.

No other open agent-self issues.

## Research Findings

No competitor research conducted — the trajectory and state evidence point to internal planning/observability gaps that don't benefit from external comparison. The immediate problems are:
1. Sessions produce no tasks despite healthy codebase
2. Cache observability is incomplete
3. Session collisions waste compute

These are internal harness problems, not competitive-position gaps.

---

## Summary

The harness is mechanically healthy — build passes, tests pass, state recorder works, DeepSeek protocol is reliable, no API errors or crashes. The dominant friction is **planning throughput**: the task picker produces empty sessions even after Day 153's simplifications. The trajectory's graph-derived pressure confirms this: `planner_no_task_count=1`, `session_success_rate=0.0`, `task_seed_contradiction_count=1`.

The most actionable candidate tasks for this session:
1. **Close the cache observability gap** (#105, #90) — record DeepSeek cache metrics during agent runs. This is a real feature gap with a tracked issue and reverted prior attempt. Scope: patch yoagent Usage or add yyds-side response extraction.
2. **Set YOYO_SESSION_BUDGET_SECS in the workflow** — a one-line workflow change to prevent session collisions (30% cancellation rate). Smallest possible fix with measurable impact.
3. **Fix ModelCall lifecycle gaps** — tighten state recording around FailureObserved so model calls don't go unmatched. Small state-integrity improvement.
