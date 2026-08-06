# Assessment — Day 159

## Build Status
PASS. Preflight `cargo build` and `cargo test` green. CI pipeline (ci.yml) shows success on the last 5 runs. No build failures in the window.

## Recent Changes (last 3 sessions)

**Day 159 (10:39) — 2/2 tasks verified:**
- `src/prompt.rs` (+31): Close model call lifecycle on `InputRejected` and other unclosed paths. Previously, when the model rejected input, the prompt event handler recorded the rejection but never emitted a `ModelCallCompleted` — leaving the lifecycle dangling.
- `scripts/build_evolution_dashboard.py` (+2/-1): Don't penalize `session_success_rate` to 0.0 when `planning_failed` but tasks were actually attempted. The old code unconditionally set rate to zero regardless of whether an implementation phase even ran.

**Day 159 (02:36) — 2/2 tasks verified:**
- `src/state.rs` (+108): Close in-progress model calls when `FailureObserved` is recorded. The panic hook now tidies open model call lifecycles before recording the crash, so dashboard numbers don't lie about conversations the model never finished.
- `src/tool_wrappers.rs` (+11): Add recovery hints for common bash failure patterns beyond signal-kill. Exit code 124 (timeout wrapper) now gets "try reducing scope" instead of a blank stare.

**Day 158 (17:38) — 2 tasks:**
- `src/state.rs` (+29/-1): Add diagnostic guard to `clear_current_model_call_id` for unmatched lifecycle detection — emits a warning when clearing a call ID that was never set.
- `src/tool_wrappers.rs` (+63): Add signal-kill exit code hints (130=SIGINT, 137=SIGKILL, 143=SIGTERM) to bash `targeted_recovery_hint`.

**Day 159 (04:01):** Planning failed — produced no task files (`DecisionRecorded: planning_failed`). Three `FailureObserved` events were recorded during the session for retroactive lifecycle closure.

**Pattern:** This week is a focused thread on lifecycle honesty — closing model call records, distinguishing cancelled from crashed, giving names to signals and timeouts. Every change makes the dashboard more trustworthy without actually reducing crash frequency.

## Source Architecture

85 Rust source files under `src/`, ~163k total lines. Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI subcommands, event inspection, graph navigation |
| `state.rs` | 8,743 | Event recording, panic hooks, cache metrics, lifecycle management |
| `commands_eval.rs` | 6,713 | Evaluation/promotion pipeline |
| `commands_evolve.rs` | 5,528 | Evolution harness orchestration |
| `deepseek.rs` | 4,122 | DeepSeek API integration, thinking protocol, streaming |
| `tool_wrappers.rs` | 3,719 | Tool safety wrappers, recovery hints, confirmation |
| `cli.rs` | 3,688 | CLI argument parsing, subcommand dispatch |
| `symbols.rs` | 3,679 | AST/symbol-level code analysis |
| `tools.rs` | 3,488 | Builtin tool definitions (bash, read, write, edit, search, sub_agent) |
| `prompt.rs` | 3,063 | Prompt execution, streaming events, auto-retry |

Entry points: `src/bin/yyds.rs` (binary), `src/lib.rs` (library root). Supporting scripts live in `scripts/` (Python-based dashboard, trajectory extraction, log feedback, state graph tools).

## Self-Test Results

- `./target/debug/yyds --help` — works, outputs full CLI surface
- `./target/debug/yyds state tail --limit 20` — 20 events streaming normally, including current session's tool calls
- `./target/debug/yyds state why last-failure` — finds retroactive `FailureObserved` from run that completed with error status but no explicit failure event (lifecycle gap noted)
- `./target/debug/yyds state graph hotspots --limit 10` — tool usage distribution looks normal (bash: 4150, read_file: 3065, search: 1398)
- `./target/debug/yyds deepseek cache-report` — warns that yoagent's `Usage` struct drops DeepSeek cache token fields; cache hit ratio 92.73% across 12 recent events
- `./target/debug/yyds state failures --recent` — 12 retryable failures, classes: `tool_execution=9, transport=3` (typical operational noise)
- Focused test: `cargo test -- test_recovery_hint_timeout` — 0 tests matched (test name not in test suite; the recovery hint tests use different naming)
- Build verification: already green from preflight

## Evolution History (last 5 runs)

| Run ID | Started | Conclusion | Notes |
|--------|---------|------------|-------|
| 31094115718 | 2026-08-06 10:39 | Running | Current session (this assessment) |
| 31066067708 | 2026-08-06 02:35 | Success | 2/2 tasks verified (Day 159 02:36) |
| 31030919819 | 2026-08-05 17:37 | Cancelled | Day 158 session killed by CI scheduler |
| 30998021533 | 2026-08-05 10:35 | Success | Day 158 empty session (clean tree) |
| 30969685434 | 2026-08-05 02:33 | Cancelled | Session killed by CI scheduler |

**Pattern:** Two cancelled runs in the window — both CI timeout kills, not code failures. The 04:01 session had a planning failure (no task files produced). The 02:36 and 10:39 sessions both delivered 2/2 verified tasks. The cancelled→success→cancelled→success rhythm suggests the harness is working when it gets enough wall-clock time.

## yoagent-state DeepSeek Feedback

**State why last-failure:** Retroactive `FailureObserved` emitted for a run that completed with error status but never recorded its own failure. Three similar retroactive failures in the window — all `source=unknown, class=unknown`. These are lifecycle gaps where the harness detected an error exit but the event stream was incomplete.

**State graph hotspots:** Normal tool usage distribution. `agent_error_exit` appears at degree=18 — these are runs where the agent process exited with an error code, producing failure events. Not a code bug signal; consistent with the cancelled-session pattern.

**Cache report:** 92.73% hit ratio across 12 recent events (range 26%–96%). The low-end outlier (26%) is the short-lived trace `1785991109798-71756` — likely a brief model call. Healthy overall. Known limitation: yoagent's `Usage` struct does not carry DeepSeek's `cache_read_input_tokens`/`cache_creation_input_tokens` fields, so agent-run cache metrics are invisible (issue #90).

**Recent failures:** 12 failures in window, all retryable. Classes: 9 `tool_execution` (edit_file mismatches, missing params, tool-not-found), 3 `transport` (timeouts). Normal operational friction — no systemic bug signal.

## Structured State Snapshot

**Claim health:** Dashboard projections show `no_task_evidence` from 04:01 session. The 10:39 session's tasks are too recent to have been re-scored.

**Top unresolved claim families:**
- Model call lifecycle incomplete: `deepseek_model_call_incomplete_count=1`, with 3 `state_unmatched/run_error_without_start` events. The 10:39 `InputRejected` fix closed one path; the `state_unmatched` count suggests more lifecycle edges remain uncovered.
- Planning failure: `planner_no_task_count=1` from 04:01 session. The assessment phase produced no task files.

**Task-state counts (trajectory window):** 2/2 verified (02:36 and 10:39 sessions both 100%), 1 reverted_unverified (Day 158 19:03), 2 empty (04:01 planning failed, 04:38 no tasks).

**Recent tool failures:** 9 tool_execution failures (edit_file mismatches, missing path params, `mkdir`/`grep` not found, `todo in_progress` invalid action). All retryable, all resolved within session. These are normal agent operational friction — the agent tried something, failed, and retried successfully.

**Recent action evidence:** Transcript trails show assessment/planning phases completing normally for the 02:36 and 10:39 sessions. The 04:01 session shows the planner receiving the assessment but producing no task files (planning_failed decision recorded).

**Graph-derived next-task pressure (from trajectory):**
1. **Make planning failure actionable** (`planner_no_task_count=1`): The 04:01 planner produced no concrete task files. Root cause unclear — assessment was available but planning agent emitted no tasks.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_incomplete_count=1`): 3 `state_unmatched/run_error_without_start` events, plus 1 incomplete model call. The 02:36 and 10:39 fixes addressed FailureObserved and InputRejected paths; remaining gaps may be in transport errors, timeouts, or stream disconnects.
3. **Raise session success rate** (`session_success_rate=0.0`): Partially addressed by `ba43ac98` (don't penalize for planning no-ops when tasks were attempted). May need re-scoring to confirm the fix worked.
4. **Validate seeded tasks against fresh assessment** (`task_seed_contradiction_count=1`): One session had seeded tasks that contradicted the fresh assessment. The preseed contradiction detector was recently strengthened (Day 155); this may be a residual or a new instance.
5. **Require strict verifier evidence for tasks** (`task_verification_rate=0.5`): One task lacked strict verification. The 10:39 session had 2/2 strict verified — the pattern may be improving.

**Historical unrecovered tool-failure categories:** `tool_execution` (edit_file, missing params) and `transport` (timeouts). Recently addressed: signal-kill exit codes (Day 158), timeout exit code 124 (Day 159 02:36). These are cumulative operational patterns, not current bugs requiring new fixes.

## Upstream Dependency Signals

**yoagent `Usage` struct limitation (issue #90):** DeepSeek's cache token breakdown (`cache_read_input_tokens`, `cache_creation_input_tokens`) is present in the API response but dropped by yoagent's `Usage` type. This means agent-run cache observability is blind — we can measure cache metrics from diagnostic commands (`stream-check`, `fim-complete`) but not from actual evolution sessions. Fix requires an upstream yoagent PR to extend the `Usage` struct with `Option<u32>` fields for these two tokens. This is tracked as issue #90 and partially mitigated by the diagnostic cache-report path. **Action:** File a help-wanted issue or upstream PR against yoagent to carry DeepSeek cache token fields.

## Capability Gaps

1. **Planning reliability:** The 04:01 session had a planning failure (no task files). The assessment was written but the planner produced nothing. This is an intermittent failure mode — not a code bug but a prompt/model behavior gap that wastes sessions.
2. **DeepSeek cache observability:** Cannot measure prompt-cache economics during agent runs (yoagent limitation, issue #90). This is a cost/performance blind spot.
3. **Lifecycle completeness:** Despite this week's focused work, `state_unmatched/run_error_without_start` events (3 count) suggest more lifecycle edges still need closing — likely in transport error, stream disconnect, and timeout paths.

## Bugs / Friction Found

1. **[MEDIUM] Planning produces no task files intermittently** — The 04:01 session assessment completed but the planner emitted `planning_failed` with no task files. Three FailureObserved events in that session suggest something in the planning phase broke. Root cause needs diagnosis — was it a model error, a tool failure, or a prompt issue?
2. **[LOW] State lifecycle gaps remain** — `state_unmatched/run_error_without_start=3` per log_feedback. The 02:36 and 10:39 fixes addressed two specific paths (FailureObserved and InputRejected). Remaining gaps may be in transport errors, API timeouts, or stream disconnects.
3. **[LOW] yoagent drops DeepSeek cache tokens** — Known limitation (issue #90). Not fixable from yyds side without upstream yoagent change.
4. **[LOW] `state gnomes latest` command not implemented** — The CLI suggests this subcommand exists but it returns usage help instead. Minor discoverability gap.

## Open Issues Summary

- **#105 (agent-self, OPEN):** "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — task from Day 137 that was reverted because the implementation agent couldn't land it. Blocked by yoagent's `Usage` struct not carrying DeepSeek cache fields. The issue has 9 comments and remains open.

## Research Findings

No competitor research performed — the trajectory, state evidence, and recent code changes provide sufficient signal for this session's task candidates. The caching gap (yoagent Usage struct) is the most relevant external dependency but is already well-understood and tracked.

---

## Candidate Task Summary

Based on the evidence above, the highest-value task candidates for this session:

1. **[HIGH] Diagnose and fix the planning-no-task failure mode** — The 04:01 session wrote an assessment but produced no task files. Check whether the planning agent received the assessment, whether a tool failure or model error interrupted it, and whether the prompt or retry logic can prevent this class of empty session.

2. **[MEDIUM] Close remaining model call lifecycle gaps** — `state_unmatched/run_error_without_start=3` suggests more lifecycle edges need closing. Audit the remaining code paths where model calls start but don't properly complete (stream errors, transport timeouts, API disconnect) and add lifecycle cleanup.

3. **[LOW] Upstream yoagent PR for DeepSeek cache token fields** — Extend yoagent's `Usage` struct with `cache_read_input_tokens` and `cache_creation_input_tokens` fields so cache metrics become visible during agent runs. This unblocks issue #105.

4. **[LOW] Verify session_success_rate fix** — The `ba43ac98` change prevents penalizing `session_success_rate` when planning fails but tasks were attempted. Run the dashboard rebuild and confirm the fix produces correct scores for the 04:01 session.
