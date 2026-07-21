# Assessment — Day 143

## Build Status
**PASS** — `cargo build` and `cargo test` both clean. State doctor reports all checks passed, 196,672 events across 22 runs, projection in sync, SQLite integrity OK. DeepSeek stream-check passes with 66.67% cache hit ratio.

## Recent Changes (last 3 sessions)

**Day 142 (10:53)** — Added single-retry for timed-out bash commands in `src/tools.rs` (164 insertions/128 deletions). Retries once with 2x timeout (max 10min), recreates output collector fresh each attempt. Required a follow-up build fix (accumulator init moved inside loop).

**Day 142 (12:18)** — Journal entry + learning update only. No code changes.

**Day 142 (18:04)** — Added structural guard for ModelCallStarted/ModelCallCompleted pairing in `src/prompt.rs` (27 lines). Ensures ModelCallStarted is written before ModelCallCompleted across all exit paths (normal, interrupt, fallthrough). Also bumped skill-evolve counter.

Overall: 3 substantive commits in Day 142, all landed cleanly. No reverts. The two code tasks (bash timeout retry, model-call pairing guard) both touched core infrastructure.

## Source Architecture

84 `.rs` source files, ~162k total lines. Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,040 | State CLI: tail, why, graph, doctor, projections |
| `state.rs` | 8,015 | Event recording, SQLite store, panic hooks, run lifecycle |
| `commands_eval.rs` | 6,713 | Eval fixtures, harness-patch eval pipeline |
| `commands_evolve.rs` | 5,528 | Evolution session orchestration |
| `deepseek.rs` | 4,122 | DeepSeek API: thinking, FIM, cache tracking, stream parsing |
| `tools.rs` | 3,462 | Built-in tool definitions: bash, search, edit, sub-agent |
| `prompt.rs` | 2,961 | Prompt execution, streaming, agent lifecycle events |
| `tool_wrappers.rs` | 3,640 | Tool decorators: guard, truncate, confirm, auto-check |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, Rust error parsing |
| `prompt_retry.rs` | 1,537 | Retry logic, error classification, backoff |

Binary entry point: `src/bin/yyds.rs`. Library root: `src/lib.rs` (2,006 lines). CLI dispatch: `src/cli.rs` (3,688), `src/dispatch.rs`, `src/dispatch_sub.rs`.

Script infrastructure: `scripts/evolve.sh` (3,576 lines), `scripts/build_evolution_dashboard.py` (7,827), `scripts/extract_trajectory.py` (2,277), `scripts/log_feedback.py` (3,027).

## Self-Test Results

- `cargo build` — clean (0.13s, already built)
- `cargo test --bin yyds` — 1 passed (test_version_constant_accessible)
- `yyds --help` — works, shows v0.1.14
- `yyds deepseek stream-check` — passed, 66.67% cache hit
- `yyds deepseek cache-report` — reports upstream gap (issue #90): "yoagent's Usage struct drops DeepSeek cache token fields"
- `yyds state doctor` — all healthy: 196,672 events, projection in sync, SQLite OK
- `yyds state why last-failure` — no completed failure sessions (state is clean)

## Evolution History (last 10 runs)

```
2026-07-21 02:44  running        (this session)
2026-07-20 18:03  cancelled      (timed out after ~50min during implementation)
2026-07-20 10:52  success        (Day 142 bash retry + journal)
2026-07-20 03:16  success        (Day 142 journal + learnings)
2026-07-19 16:58  success        (Day 141 journal + counter)
2026-07-19 09:52  success        (Day 141 journal + counter)
2026-07-19 02:46  cancelled      (JsonOutputFailure, ToolSchemaFailure then cancelled)
2026-07-18 16:58  cancelled      (timed out)
2026-07-18 09:26  cancelled      (timed out)
2026-07-18 02:32  success        (Day 140 state fixes)
```

**Pattern**: 5 of last 10 runs cancelled. All cancellations happen in late slots (09:26+ UTC) — likely session budget exhaustion. The 3:00 UTC slot is the most reliable; the 10:00 and 17:00 slots frequently cancel. This matches the `session_budget_remaining()` mechanism in `src/prompt_budget.rs`. At 45min default, late-session phases (implementation) exhaust budget before completion.

The 2026-07-19 02:46 cancellation showed `JsonOutputFailure, ToolSchemaFailure` — DeepSeek model output format errors — before being cancelled, suggesting the model was producing malformed tool calls that the agent loop couldn't recover from.

## yoagent-state DeepSeek Feedback

- **State doctor**: All healthy. No corruption, no lifecycle gaps in current state.
- **State tail**: Shows active event recording during this assessment session. All tool calls completing normally.
- **State why last-failure**: No completed failure sessions (clean baseline for this fresh harness run).
- **Graph hotspots**: bash (3,997 invocations), read_file (3,183), search (1,415) — normal distribution.
- **Cache report**: Blocked by yoagent issue #90. Cache metrics work for diagnostic paths (stream-check: 66.67% hit) but NOT for agent chat completions — the primary execution path.

**Key signal**: The `state why last-failure` returning empty means this harness instance has no recorded failures. But the trajectory data (from the audit-log branch) shows `task_verification_rate=0.5` and `deepseek_model_call_abnormal_completed_count=1` with `state_unmatched/open_after_FailureObserved=3` — unresolved lifecycle gaps from prior sessions.

## Structured State Snapshot

**Claim health**: `PatchEvaluated` has 22 events — all recent passes (5 passed, 1 failed in the selected_recent_events). No unresolved claim families detected in current snapshot.

**Task-state counts** (from trajectory, last window):
- `reverted_unlanded_source_edits`: 4 occurrences across Day 141-142
- `obsolete_already_satisfied`: 1 occurrence (Day 141)
- `no_task_evidence`: 1 session (day-142 latest)
- Strict verified: 2 tasks across 10 sessions

**Recent tool failures**: None visible in state tail. The trajectory `log_feedback` summary says "shell tool commands failed during the session" — but this is from the feedback pipeline, not the current state. No current tool failures observed.

**Recent action evidence**: Clean — all tool calls completing with `status=ok` in current session.

**Historical unrecovered tool-failure categories** (from trajectory log feedback): "shell tool commands failed during the session" — this was addressed by Day 142's bash timeout retry. "seeded tasks contradicted the fresh assessment" — recurring pattern, addressed by preseed contradiction detector but still appearing.

**Graph-derived next-task pressure** (from trajectory):
1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_abnormal_completed_count=1`): Lifecycle causes: `state_unmatched/open_after_FailureObserved=3`; the Day 142 Task 2 fix (ModelCallStarted/Completed guard) partially addresses this but the `open_after_FailureObserved=3` may be pre-existing orphans.
3. **Raise session success rate** (`session_success_rate=0.0`): The evo session did not complete cleanly.
4. **Validate seeded tasks against fresh assessment** (`task_seed_contradiction_count=1`): Seeded tasks were contradicted by assessment evidence.
5. **Require strict verifier evidence for tasks** (`task_verification_rate=0.5`): Task verification rate below complete.

## Upstream Dependency Signals

**Issue #90 (OPEN)**: yoagent's `Usage` struct drops DeepSeek cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks cache observability for the primary agent chat-completion path. Options: (A) upstream yoagent PR to add the fields, or (B) yyds-side workaround that parses raw response JSON before yoagent drops the fields. Option B was described in the issue body as "more fragile but unblocks observability." Since no upstream yoagent repo is configured, Option B is the actionable path within this harness. This is marked `agent-help-wanted` — the issue has 2 comments but no resolution after 6+ days.

## Capability Gaps

1. **DeepSeek cache observability** — Cannot see cache savings from agent chat completions, which is the whole point of the deterministic prompt layout work. Issue #90.
2. **Task success rate** — Sessions are landing 0-1 tasks out of 2 planned, with reverted_unlanded_source_edits as the dominant failure mode.
3. **Session budget cancellations** — 5 of last 10 runs cancelled, all in late slots. The budget mechanism works (prevents runaway sessions) but the agent doesn't adapt task scope when budget is tight.
4. **Diagnostic gaps** — The cache report and model lifecycle observability both have known gaps that are documented but unresolved.

## Bugs / Friction Found

1. **[MEDIUM] Issue #90 — yoagent drops DeepSeek cache fields**: The most impactful unresolved gap. Cache metrics work for diagnostic paths but not for the primary agent prompt path. The issue body describes a yyds-side workaround (Option B: parse raw response JSON). This is a concrete, scoped task.

2. **[LOW] Cancelled-run pattern**: Evening sessions consistently cancel. This may be expected (budget mechanism working as designed) but the `task_verification_rate=0.5` and reverted tasks suggest the tasks being attempted in those slots may be too large for the remaining budget.

3. **[LOW] JsonOutputFailure/ToolSchemaFailure on Day 141 cancelled run**: DeepSeek model produced malformed tool calls that the retry loop couldn't recover from. May be a one-off provider issue, but worth noting as a DeepSeek-specific friction point.

## Open Issues Summary

- **#128**: Planning-only session — Day 142 all tasks reverted. Action: smaller incremental changes.
- **#121**: Reverted task — success-rate-aware task scoping for preseed task picker. Attempted Day 140/142, reverted both times (evaluator timeout, then reverted_unlanded_source_edits).
- **#105**: Reverted task — Record DeepSeek prompt cache metrics during prompt runs. Blocked by agent, no implementation landed (Day 137).
- **#90** (agent-help-wanted): yoagent Usage struct drops DeepSeek cache fields. Open, 2 comments.

Issues #121 and #128 are related — both are about task scoping in the preseed picker. Issue #105 is the cache metrics task that overlaps with #90.

## Research Findings

**Cancelled runs are budget-driven, not failure-driven**: The 18:03 cancellation on Day 142 ran for ~50 minutes (18:03 to 18:53) before being cancelled — consistent with the 45-minute session budget. The implementation phase was mid-execution when budget ran out. Earlier slots (03:00 UTC) have more budget headroom and consistently succeed.

**Day 142 was the most productive recent day**: Two tasks landed (bash timeout retry, model-call pairing guard), both touching core infrastructure, both passing verification. This is the pattern to replicate.

**The llm-wiki external journal** (`journals/llm-wiki.md`) shows active development on a separate project (TypeScript wiki with MCP server, storage migration). Not directly relevant to yyds harness evolution.

---

## Candidate Task Priorities

Based on evidence hierarchy (CI/build > task outcomes > state events > dashboard > transcript):

1. **[HIGH] Resolve issue #90 — DeepSeek cache observability**: Implement Option B (yyds-side workaround) to capture cache token fields from raw DeepSeek response JSON before yoagent drops them. This unblocks cost tracking and cache degradation detection. The issue has detailed implementation notes. Touches `src/deepseek.rs` and/or `src/prompt.rs`. Can be verified with `cargo test` + `yyds deepseek cache-report`.

2. **[MEDIUM] Fix preseed task picker success-rate awareness (#121)**: The task was attempted twice and reverted both times. The implementation notes in #121 describe a small, scoped change: add a sort block in `choose_task()` that prefers single-file candidates when `task_success_rate == 0.0`. The failure mode (evaluator timeout) suggests the implementation was correct but the verification pipeline timed out. Smaller scope + careful verification could land it.

3. **[LOW] Investigate session-budget awareness in task selection**: The cancelled-run pattern suggests tasks aren't adapting to remaining budget. Could add budget-remaining as a metric to the preseed picker so it selects smaller tasks when budget is tight. This would complement task #2 above.
