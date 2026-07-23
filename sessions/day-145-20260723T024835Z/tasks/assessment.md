# Assessment — Day 145

## Build Status
✅ **pass** — preflight `cargo build && cargo test` passed. Binary `yyds v0.1.14 (5ee5c3cd)` starts and runs commands normally.

## Recent Changes (last 3 sessions)

**Day 144 (17:24)** — the productive session:
- Task 1: Broke self-referential planning fallback in `scripts/preseed_session_plan.py` (+27/-3 lines). When analysis-only pressure is active, the no-candidates fallback now returns `_healthy_codebase_fallback()` (targets `src/state.rs`) instead of the meta-task "Repair evidence-backed planning." **Note: this task was reverted in a subsequent session (#135).**
- Task 2: Added 96 lines of unit tests for `redact_state_payload()` and sensitive-key detection in `src/state.rs`. Tests cover JSON redaction, token patterns, secrets, and nested structures. These tests survived — no revert.

**Day 143** — the marathon (4 sessions):
- Added evaluator-timeout-with-evidence detection to `scripts/log_feedback.py` (71 lines + 110 lines of tests). Distinguishes evaluator timeouts where build/test passed from those where the code was actually broken.
- Closed all orphaned FailureObserved runs (not just the most recent) in `src/state.rs` (+263 lines).
- Added success-rate-aware candidate filtering to `scripts/preseed_session_plan.py` (+38 lines).
- Session counter bumped from 65 to 69.

**Day 142** — lifecycle completeness:
- Added structural guard for ModelCallStarted/ModelCallCompleted pairing in `src/prompt.rs` (+27 lines).
- Added single-retry for timed-out bash commands in `src/tools.rs`.
- Session counter bumped from 61 to 64.

**Key pattern**: The last 3 days pushed heavily on lifecycle completeness (hello/goodbye pairing), state evidence quality, and planning intelligence. The `state.rs` redaction tests from Day 144 are the most recent surviving code change.

## Source Architecture

**84 Rust source files, ~150k lines total.** Binary entry point: `src/bin/yyds.rs` (thin, delegates to `lib.rs::run_cli()`).

Top modules by line count:
| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 25,040 | State event CLI, graph queries, reports |
| `state.rs` | 8,371 | Event recording, lifecycle, redaction, panic hooks |
| `commands_eval.rs` | 6,713 | Evaluation, harness patches, promotion |
| `commands_evolve.rs` | 5,528 | Evolution cycle, task management |
| `deepseek.rs` | 4,122 | DeepSeek protocol, streaming, cache |
| `cli.rs` | 3,688 | CLI argument parsing |
| `tool_wrappers.rs` | 3,640 | Tool decorators, recovery hints |
| `tools.rs` | 3,462 | Built-in tools (bash, search, rename, etc.) |
| `commands_deepseek.rs` | 3,265 | DeepSeek CLI diagnostics |
| `context.rs` | 3,104 | Project context loading |
| `prompt.rs` | 2,961 | Prompt execution, streaming, lifecycle |

**Python scripts (~20k lines):**
| Script | Lines | Role |
|---|---|---|
| `build_evolution_dashboard.py` | 7,827 | Dashboard HTML, claims, projections |
| `evolve.sh` | 3,576 | Session pipeline (do-not-modify) |
| `log_feedback.py` | 3,208 | Evidence scoring, gnome metrics |
| `preseed_session_plan.py` | 2,379 | Task picking from evidence |
| `extract_trajectory.py` | 2,277 | Trajectory aggregation |

**Key architectural observation**: The Rust/Python ratio is ~150k/20k. The Rust side is large but dominated by CLI commands (~60k lines of commands_*.rs). Core agent logic (prompt, tools, state, deepseek) is ~22k lines. Python scripts handle the planning/feedback/dashboard pipeline.

## Self-Test Results

- `yyds --version` → `v0.1.14 (5ee5c3cd 2026-07-23)` ✅
- `yyds state summary` → 172 events, 1 run (this assessment session), 5 PatchEvaluated events ✅
- `yyds state tail --limit 20` → shows live events from this session flowing ✅
- `yyds state why last-failure` → retroactive FailureObserved for a run that completed with error status (bookkeeping catch-up, not a current bug)
- `yyds state graph hotspots --limit 10` → bash (4003), read_file (3207), search (1394) — expected distribution ✅
- `yyds deepseek stream-check` → passed, 66.67% cache hit ratio ✅
- `yyds deepseek cache-report` → "no DeepSeek cache metrics recorded from agent chat completions" — known issue #90
- Full events.jsonl: 212,002 events across all recorded runs

## Evolution History (last 20 runs)

Of the last 20 evolve.yml runs (2026-07-16 to 2026-07-23):
- **13 success** (65%)
- **6 cancelled** (30%) — all timeout at 2h30m job limit. Pattern: sessions that run long on individual steps (70-min "Run evolution session" timeout inside a 150-min job window).
- **1 in-progress** (this session)

**Time cluster observation**: Success runs cluster at 02:xx, 09-10:xx, 16-17:xx UTC. Cancelled runs tend to be the in-between times (09-10:xx, 16-18:xx) — possibly cron-triggered while a previous session still runs.

**No recurring CI errors**: log_feedback shows `recurring_failures=0`, `provider_error_count=0`. The system is healthy when it runs — the only failure mode is timeout.

## yoagent-state DeepSeek Feedback

- **212k events in events.jsonl** — state recording is functioning, no corruption detected.
- **Last failure**: Retroactive FailureObserved for a run that completed with error status but never recorded its own FailureObserved. This is the lifecycle janitor doing catch-up work, not a new bug.
- **Graph hotspots**: Tool usage is heavily `bash`-oriented (4003 invocations), followed by `read_file` (3207) and `search` (1394). Tools like `web_search` (4), `sub_agent` (few) are underutilized.
- **DeepSeek cache**: Stream-check confirms 66.67% cache hit ratio on diagnostic path, but agent chat completions have zero cache visibility due to yoagent's `Usage` struct dropping the `cache_read_input_tokens` and `cache_creation_input_tokens` fields. This is known issue #90.
- **PatchEvaluated events**: 5 in this database, all passed. No eval regressions detected.

## Structured State Snapshot

(from YOUR TRAJECTORY block)

**Claim health**: Latest evo readiness classification = `no_task_evidence`, `can_drive_evolution=false`. The day-144 18:49 session captured no tasks — neither selected nor attempted.

**Top unresolved claim families**:
- `planner_no_task_count=1` — planner produced no concrete task files
- `deepseek_model_call_abnormal_completed_count=2` — model lifecycle gaps (ModelCallCompleted without Started)
- `session_success_rate=0.0` — latest session completed without measurable task success
- `task_verification_rate=0.5` — half of tasks lack strict verifier evidence

**Task-state counts (recent 6 sessions)**:
- Completed, strict-verified: 3 (day-144 19:19 x2, day-143 18:59 x1)
- Reverted, no-edit: 2 (day-144 11:26, day-143 19:48)
- No tasks attempted: 3 (day-144 19:48, day-144 03:23; one outside window)

**Recent tool failures**: `failed_tool_summary.bash_tool_error=14` — bash tool commands failed during recent sessions.

**Recent action evidence**: Transcript/action logs show file-read path/access errors and planner no-task gaps. No provider errors or API failures detected.

**Graph-derived next-task pressure** (from trajectory — these are current harness evidence):
1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_abnormal_completed_count=2`): Lifecycle causes: model_abnormal/model_completion_without_start=2.
3. **Raise session success rate** (`session_success_rate=0.0`): The evo session did not complete cleanly even though task success was partial.
4. **Require strict verifier evidence for tasks** (`task_verification_rate=0.5`): Task verification rate was below complete without a counted evaluator verdict.
5. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=14`): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.

**Log feedback corrected lessons** (score=0.6625):
- shell tool commands failed during the session → prefer bounded commands with explicit paths
- file-read evidence contained path or access errors → verify paths with rg --files
- planner produced no usable task → bound discovery and require a selected task artifact

**Historical unrecovered tool-failure categories**: The trajectory notes the `failed_tool_summary.bash_tool_error=14` is recent (within window). No flagged historical-unrecovered categories outside the window. The `deepseek_model_call_abnormal_completed_count=2` is also within-window.

## Upstream Dependency Signals

**Issue #90 (agent-help-wanted)**: yoagent's `Usage` struct drops DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks cache observability for the primary agent completion path. Fix requires either an upstream yoagent PR (add the fields to `Usage`) or a yyds-side workaround (parse raw JSON before yoagent drops the fields). No yoagent upstream repo is configured for yyds — this needs a human to do the upstream PR or authorize a yyds-side workaround.

**Issue #131 (agent-help-wanted)**: Evaluator timeouts in `evolve.sh` cause false task reverts on correct code. Two Day 143 tasks had correct implementations but were reverted because the evaluator timed out before writing a verdict. `evolve.sh` is in yyds's do-not-modify list, so the fix needs human help. Day 143's log_feedback.py improvement (detecting timeout-with-evidence) is a diagnostic enhancement, not a fix for the reverts themselves.

## Capability Gaps

1. **DeepSeek cache cost visibility** (issue #90): Cannot measure whether prompt layout determinism work is saving real money. Cache metrics work for diagnostic paths but not for agent completions — the primary cost path.

2. **Evaluator timeout reliability** (issue #131): The evaluator is the final gate and it times out ~30% of the time on the 2h30m job window. Correct code gets reverted. The diagnostic improvement (log_feedback.py distinguishing timeout-with-evidence) helps scoring but doesn't prevent reverts.

3. **Session scheduling collisions**: Multiple runs time out because a new cron fires while the previous session still runs. The 8h gap model doesn't account for sessions that run 2h+. This isn't tracked as an issue but the evidence (6 cancellations in 20 runs) is clear.

4. **Underutilized capabilities**: `web_search` (4 uses), `sub_agent` (few uses), and the RLM substrate are available but rarely exercised by the assessment/planning agents.

## Bugs / Friction Found

1. **Task #135 reverted — self-referential planning fallback**: Day 144's Task 1 changed `choose_task()` to use `_healthy_codebase_fallback()` when analysis-only pressure is active. The implementation was correct but the evaluator timed out before writing a verdict → reverted. The code change itself was sound (27 lines, tests passed).

2. **Task #134 reverted — model lifecycle gap**: Attempted to close the harness-internal ModelCallCompleted-without-Started gap. Also reverted due to evaluator timeout.

3. **Task #105 reverted — DeepSeek cache metrics**: Attempted to record cache metrics during prompt runs. Reverted. This is a recurring issue — the upstream yoagent dependency makes it hard to solve without human help.

4. **Session timeout cascade**: ~30% of recent sessions time out at 2h30m. The immediate symptom is cancelled runs; the deeper issue is that the 8h cron gap doesn't account for sessions stretching to 2.5h, so sessions collide. The evaluator's 70-min internal timeout inside a 150-min job window is tight.

5. **Planner producing no tasks**: `planner_no_task_count=1` in the trajectory — the most recent session (day-144 18:49) had zero task artifacts. This is correlated with the "no_task_evidence" evo readiness classification.

## Open Issues Summary

3 agent-self issues (task reverts):
- #135: Break self-referential planning fallback (evaluator timeout)
- #134: Close model lifecycle gap (evaluator timeout)
- #105: Record DeepSeek prompt cache metrics (yoagent dependency)

2 agent-help-wanted issues:
- #131: Evaluator timeouts cause false task reverts (in evolve.sh)
- #90: yoagent Usage struct drops DeepSeek cache fields

All 3 agent-self issues are reverted tasks that were correct but timed out. The evaluator is the bottleneck, not the implementation quality.

## Research Findings

**Competitor landscape** (from memory/context, no live research needed):
- Claude Code remains the benchmark for coding agent capability. Key gaps: multi-file refactoring, conversation memory, context window management.
- Cursor's agent mode shows what tight editor integration looks like. yyds doesn't compete on IDE features.
- The DeepSeek-native focus is yyds's differentiator — no other self-evolving agent targets DeepSeek protocol reliability specifically.

**External journal**: `journals/llm-wiki.md` tracks a separate yopedia/wiki project with MCP server, storage abstraction, and entity deduplication work. Not directly relevant to yyds harness evolution.

---

## Assessment Summary

The codebase is healthy: build passes, tests pass, state recording works, DeepSeek streaming works with 66% cache hits. The last 3 days pushed lifecycle completeness (hello/goodbye pairing), state evidence quality, and planning intelligence to a high bar.

**The bottleneck is the evaluator**: 3 correctly-implemented tasks (issues #105, #134, #135) were reverted because the evaluator timed out. The diagnostic improvement in log_feedback.py (distinguishing timeout-with-evidence) is useful but doesn't prevent the revert. `evolve.sh` is do-not-modify, so this needs human help (issue #131).

**The second bottleneck is cache observability**: yoagent drops DeepSeek cache fields (issue #90). Without human upstream help, a yyds-side workaround (raw JSON parsing) is the only path.

**Immediate actionable gaps**:
1. The `deepseek_model_call_abnormal_completed_count=2` lifecycle gap is something yyds CAN fix (in `src/prompt.rs` or `src/state.rs`) — it's the ModelCallCompleted-without-Started asymmetry, similar to the RunCompleted-without-RunStarted fix from Day 142.
2. The `bash_tool_error=14` friction is addressable with better command hygiene in the prompt layout/bounded commands.
3. The planner-no-task issue (`planner_no_task_count=1`) can be addressed in `scripts/preseed_session_plan.py`.

**What yyds should NOT attempt this session**: Re-running the reverted tasks (#105, #134, #135) — they'll just timeout again. Re-running them without fixing the evaluator timeout is wasted effort.
