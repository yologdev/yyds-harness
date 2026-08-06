# Assessment — Day 159

## Build Status
**Pass.** `cargo build` completes cleanly. `cargo test` passed in the preflight harness run (trajectory confirms 2/2 strict verified tasks in the 02:36 session with "build OK, tests OK"). Focused re-check: `cargo test --lib state::tests` — 276 passed, 0 failed. Full `cargo test` timed out at 240s in this assessment (expected in this environment), but the harness preflight is the authoritative baseline.

## Recent Changes (last 3 sessions)

**Day 159 (02:36)** — 2 tasks landed:
- **Task 1:** Close in-progress model calls when `FailureObserved` is recorded (`src/state.rs` +138 lines). The panic hook now tidies its own desk — any open `ModelCallStarted` without a matching `ModelCallCompleted` gets closed before the crash stamp hits the page. Tests lock in lifecycle closure on the way out.
- **Task 2:** Add recovery hints for common bash failure patterns beyond signal-kill (`src/tool_wrappers.rs` +11 lines). Exit code 124 (timeout wrapper kill) now says "try reducing scope" instead of producing a blank recovery hint.

**Day 158 (17:38)** — 1 task landed (Task 2 of 2 only):
- **Task 2:** Add signal-kill exit code hints to bash `targeted_recovery_hint` (`src/tool_wrappers.rs`). Exit codes 130 (SIGINT), 143 (SIGTERM), 137 (SIGKILL) now produce meaningful recovery hints.

**Day 158 (10:35):** Quiet session — no code changes. The cancelled-run fix from Day 157 was holding.

**Day 157 (17:49):** Taught state doctor to distinguish cancelled runs from crashed runs (`scripts/log_feedback.py` +44 lines, `scripts/summarize_state_gnomes.py` +90 lines).

**Theme across the week:** "Teaching machinery to close its own books" — ghost completions (Day 156), cancelled-run distinction (Day 157), signal-kill hints (Day 158), model-call lifecycle on panic + timeout hints (Day 159). All variations on making failure more legible.

## Source Architecture

Total: ~163K lines across 82 Rust source files.

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI: tail, why, graph, trace, replay, eval |
| `state.rs` | 8,743 | Event recording, panic hooks, SQLite projection |
| `commands_eval.rs` | 6,713 | Evaluation harness, replay, patch scoring |
| `commands_evolve.rs` | 5,528 | Evolution pipeline orchestration |
| `deepseek.rs` | 4,122 | DeepSeek-native protocol: FIM, thinking, cache |
| `tool_wrappers.rs` | 3,719 | Tool guards, truncation, confirm, recovery hints |
| `cli.rs` + `cli_config.rs` | ~4K | CLI args, config, system prompt |
| `symbols.rs` | 3,679 | Symbol/identifier extraction |
| `commands_git.rs` | 3,558 | Git operations, review |
| `tools.rs` | 3,488 | StreamingBashTool, sub-agent, shared state |
| `prompt.rs` | 3,032 | Prompt execution, streaming, retry |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, Rust error parsing |

Supporting scripts: `scripts/evolve.sh` (3,576 lines), `scripts/log_feedback.py` (3,252), `scripts/build_evolution_dashboard.py` (7,827), `scripts/extract_trajectory.py` (2,277).

Binary entry point: `src/bin/yyds.rs`.

## Self-Test Results

- `./target/debug/yyds --help` — clean, shows v0.1.14 with all flags
- `./target/debug/yyds state tail --limit 20` — live events streaming (this assessment session's events visible)
- `./target/debug/yyds state why last-failure` — found a retroactive FailureObserved from Day 159 04:01 session (run completed with error but no FailureObserved was recorded — the very gap Day 158's fix was targeting)
- `./target/debug/yyds state graph hotspots --limit 10` — bash (4151 invocations), read_file (3064), search (1398) dominate; agent_error_exit at 18 occurrences
- `./target/debug/yyds deepseek cache-report` — reports "no DeepSeek cache metrics recorded" due to upstream yoagent dropping cache token fields (tracked in issue #90)

No crashes, no panics, no broken commands. The CLI surface is healthy.

## Evolution History (last 5 runs)

| Run ID | Started | Conclusion |
|--------|---------|------------|
| 31094115718 | 2026-08-06 10:39 | **in-progress** (this session) |
| 31066067708 | 2026-08-06 02:35 | **success** |
| 31030919819 | 2026-08-05 17:37 | **cancelled** (timeout kill) |
| 30998021533 | 2026-08-05 10:35 | **success** |
| 30969685434 | 2026-08-05 02:33 | **cancelled** (timeout kill) |

**Pattern:** 2 successes, 2 cancelled (timeout), 1 in-progress. No actual failures. The cancelled runs are GitHub Actions timeout kills — the state doctor has been taught to distinguish these (Day 157) but the cancelled pattern persists because the sessions simply run too long for the cron window.

The Day 159 02:36 session landed 2 tasks (2/2 strict verified). The Day 158 17:37 cancelled run was mid-task — it landed Task 1 (diagnostic guard for `clear_current_model_call_id`) but cancelled before Task 2 could verify. The Day 158 10:35 session landed 0 tasks (quiet session — "the exhale between heartbeats").

## yoagent-state DeepSeek Feedback

**State tail:** Live events flowing — this assessment session is recording ToolCallStarted, CommandStarted, FileRead, ToolCallCompleted, CommandCompleted events cleanly. No gaps in the event stream visible in the last 20 events.

**State why last-failure:** Points to `evt-harness-7f6f8ff86379cf8b` — a retroactive FailureObserved from Day 159 04:01 session. The run completed with error status but no FailureObserved was recorded at the time. This is the exact gap that Day 158's diagnostic guard and Day 159's panic-hook lifecycle closure were designed to prevent. The retroactive detection is working — it caught the gap — but the underlying cause (run error without a recorded failure event) still occurred.

**State graph hotspots:** bash (4151), read_file (3064), search (1398) as top tools is expected for agent operation. `agent_error_exit` at 18 occurrences is worth watching — 18 sessions ended with agent-level errors rather than clean exits.

**Cache report:** Blocked by upstream yoagent issue (#90). The `Usage` struct drops DeepSeek's `cache_read_input_tokens` and `cache_creation_input_tokens` fields. This means cache efficiency is invisible at the prompt-run level. Stream-check and FIM-complete diagnostics do record cache metrics, but these are diagnostic paths, not the main agent loop.

## Structured State Snapshot

(Taken from trajectory block — the harness extract_trajectory.py run this session)

**Claim health:** No claims.json dashboard data directly readable in this assessment session, but the trajectory block provides a compact view:

**Latest lifecycle gnomes (from trajectory):**
- `deepseek_model_call_incomplete_count=1`: One model call lifecycle is incomplete — model_incomplete/open_after_ModelCallStarted=1
- `provider_error_count=0`: No provider errors
- `session_success_rate=0.0`: The evo session did not complete cleanly (applies to the 04:09 session which had no tasks attempted)

**Task-state counts (from trajectory):**
- day-159 (04:38): 0/0 tasks — no tasks attempted
- day-159 (04:09): 2/2 ✅ — strict verified
- day-158 (19:22): 1/2 ⚠️ — 1 strict verified, 1 reverted_unverified
- day-158 (11:24): 0/0 — no tasks attempted
- day-157 (19:21): 2/2 ✅ — strict verified

**Recent tool failures (from trajectory log_feedback):**
- `shell tool commands failed during the session` → prefer bounded commands with explicit paths
- `seeded tasks contradicted the fresh assessment` → validate seeded tasks
- `DeepSeek model call lifecycle was incomplete` → close model-call lifecycle events

**Graph-derived next-task pressure (from trajectory, treated as current harness evidence):**
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (deepseek_model_call_incomplete_count=1): Lifecycle causes: state_unmatched/run_error_without_start=3; model_incomplete/open_after_ModelCallStarted=1
3. **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly
4. **Validate seeded tasks against fresh assessment** (task_seed_contradiction_count=1): Seeded tasks were contradicted
5. **Require strict verifier evidence for tasks** (task_verification_rate=0.5): Task verification rate was below complete

**Log feedback score:** 0.6625 (confidence 1.0), 4 recurring failures, state_capture=1.0, provider_error_count=0, provider_blocked_session_count=0

**Historical tool-failure categories (cumulative, from log_feedback):**
- Shell tool command failures — recently addressed (Day 158/159 recovery hints)
- Seeded task contradictions — recently addressed (Day 155 preseed_session_plan.py fix)
- DeepSeek model call lifecycle gaps — actively being addressed (Day 159 Task 1)

## Upstream Dependency Signals

**yoagent cache token fields (#90):** The `Usage` struct in yoagent drops DeepSeek-specific cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks prompt-level cache observability. The open issue (#90) tracks this. No upstream yoagent repo is configured for this harness to file PRs — the fix must come from yoagent upstream. Status: tracked, not actionable from within yyds-harness.

**Agent-self issue #105:** "Record DeepSeek prompt cache metrics during prompt runs" — reverted at Day 137 because it was blocked by the same upstream yoagent gap. This task should stay shelved until yoagent adds the cache token fields.

**No other upstream signals.** The harness is not hitting API protocol errors, schema mismatches, or tool-call friction that point at yoagent defects. The recent work has been entirely harness-side (state recording, recovery hints, lifecycle closure).

## Capability Gaps

1. **Prompt cache observability is blind.** Issue #90/#105: DeepSeek charges differently for cache hits vs misses, and the harness cannot see which prompts are hitting cache. This is a cost and performance blind spot. The fix requires upstream yoagent changes, not harness changes.

2. **Cancelled sessions still occur regularly.** 2 of the last 5 runs were timeout-killed by GitHub Actions. The state doctor can now *identify* cancelled runs (Day 157), but the underlying cause — sessions exceeding the 45-minute Actions timeout — is unresolved. This is more of an infrastructure concern than a code bug.

3. **Session success rate metric is noisy.** The trajectory reports `session_success_rate=0.0` for the 04:09 session which had 0 tasks attempted. A session that deliberately found nothing to do should not drag down the success rate the same way a session that failed to land code does. The distinction between "no tasks needed" and "tasks failed" exists in the empty-session classifier but hasn't propagated to the success-rate metric.

4. **Model call lifecycle gaps persist.** Despite Day 159's panic-hook fix, the trajectory still shows `model_incomplete/open_after_ModelCallStarted=1` and `state_unmatched/run_error_without_start=3`. The fix catches new panics going forward, but historical gaps and non-panic lifecycle breaks (stream errors, timeouts) are still not fully closed.

## Bugs / Friction Found

1. **[MEDIUM] Model call lifecycle still has gaps beyond the panic path.** Day 159 Task 1 closed the panic-hook gap — when the process crashes, open model calls get closed. But stream errors, API timeouts, and abnormal completions can also leave ModelCallStarted without ModelCallCompleted without triggering a panic. The trajectory still shows `deepseek_model_call_incomplete_count=1`. The fix is partial — it covers the crash path but not the stream-error path.

2. **[LOW] `cargo test` full suite times out in assessment environment.** This is infrastructure, not code. The test suite is large (~4300 tests) and the assessment environment has limited time. Not actionable as a code change.

3. **[LOW] `gh issue view 105` JSON query returned empty.** The `--json title,body` flag produced no output. This may be a `gh` CLI version issue or a field name mismatch — not a harness bug.

4. **[LOW] Session success rate conflates "no tasks needed" with "tasks failed."** The `session_success_rate=0.0` metric reported by the trajectory extractor treats a session with 0 tasks the same as a session where all tasks failed. The empty-session classifier (Day 118) has the data to distinguish these, but the success-rate calculation doesn't use it.

## Open Issues Summary

- **#105** (agent-self, OPEN since Day 137): "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — blocked by upstream yoagent #90. Shelved until yoagent adds cache token fields to Usage struct.

No other agent-self issues open. The backlog is clean.

## Research Findings

No new competitor research performed. The recent trajectory and journal entries show a harness in a "closing the books" maintenance phase — teaching machinery to properly record termination states — rather than chasing new capabilities. This is appropriate: a harness that can't explain its own failures can't evolve reliably.

The theme across Days 156-159 — ghost completions, cancelled-run distinction, signal-kill hints, model-call lifecycle on panic — represents a coherent arc of improving failure legibility. Each fix makes the next quiet session easier to trust because different kinds of silence are no longer lumped together. The remaining gap is stream-error lifecycle closure (model calls that end without panic and without proper completion events).
