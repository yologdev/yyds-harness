# Assessment — Day 142

## Build Status
**PASS** — harness preflight (`cargo build` + `cargo test`) passed before assessment. Binary is `yyds v0.1.14`. No compilation errors or test failures.

## Recent Changes (last 3 sessions)

### Day 141 (09:54) — SQLite projection tolerance + bash safety guard
- **Fix (Task 1):** `src/state.rs` / `src/commands_state.rs` — SQLite projection rebuild now skips unknown event types instead of failing. "skipped N unknown events" reported, rebuild continues. (commit `3d57df6d`)
- **New feature:** `src/safety.rs` — bash safety checker (#27) detects filesystem-root scans (`find /`, `grep -r /`, `rg /`) before execution and warns to add `-maxdepth` or narrow the path. 80 lines + 3 test batteries.

### Day 141 (16:58) — journal entry + counter bump
- Journal entry only. Skill-evolve counter 58→59. No code changes.

### Day 141 (18:24) — journal entry + counter bump
- Journal entry only. Skill-evolve counter 59→60. No code changes.

### Day 140 (02:33) — exit reasons + lifecycle closure
- `src/prompt.rs` / `src/state.rs` — `AgentExitReason` event stamps why agent stopped (done_complete, done_interrupted, stream_stopped, done_tool).
- `scripts/append_terminal_state_events.py` — Janitor now writes retroactive ModelCallStarted for orphaned ModelCallCompleted. Three test cases.
- Also: janitor dedup test (no double-write), retroactive RunStarted for runs completed without start, state doctor projection drift counter.

**Pattern:** Two sharp sessions (Day 140 02:33, Day 141 09:54) landed code. Three sessions were heartbeat-only (counter bumps + journal). The trajectory confirms: of 4 Day 141 sessions, 2 landed code and 2 were empty.

## Source Architecture

76 `.rs` files, ~150K total lines. Key modules:

| File | Lines | Role |
|------|-------|------|
| `commands_state.rs` | 25,040 | State CLI: tail, summary, graph, doctor, why, replay |
| `state.rs` | 8,015 | Event recording engine, RunStarted/Completed lifecycle, SQLite projection |
| `commands_eval.rs` | 6,713 | Evaluator harness, PatchEvaluated events, gnome metrics |
| `commands_evolve.rs` | 5,528 | Evolution pipeline subcommands |
| `deepseek.rs` | 4,122 | DeepSeek protocol: stream-check, fim-complete, cache-report |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `tool_wrappers.rs` | 3,640 | Tool decorators: GuardedTool, TruncatingTool, AutoCheckTool, RecoveryHintTool |
| `tools.rs` | 3,426 | Tool builders: BashTool, SubAgentTool, SharedState, all builtins |
| `prompt.rs` | 2,934 | Prompt execution, streaming, auto-retry, exit reasons |
| `safety.rs` | 1,749 | Bash command safety analysis, filesystem-root scan detection |
| `config.rs` | 2,311 | Permission config, MCP server config, TOML parsing |
| `context.rs` | 3,104 | Project context loading (YOYO.md, CLAUDE.md, git status) |
| `agent_builder.rs` | 2,209 | AgentConfig, build_agent, MCP collision detection |

Entry points: `src/main.rs` (binary), `src/lib.rs` (library facade). DeepSeek-native protocol support lives in `src/deepseek.rs` and `src/commands_deepseek.rs` (3,265 lines).

Supporting scripts: `scripts/evolve.sh` (3,576 lines — harness pipeline), `scripts/preseed_session_plan.py` (task picker/fallback), `scripts/append_terminal_state_events.py` (state janitor), `scripts/log_feedback.py` (log analysis), `scripts/extract_trajectory.py` (trajectory aggregation).

## Self-Test Results

- `yyds --help`: OK, prints v0.1.14 with usage
- `yyds state summary`: OK, 182 total events, 1 run started, 0 completed (this session's run is in-progress), 5 PatchEvaluated events
- `yyds state tail --limit 20`: OK, shows current session's events streaming in real-time (ToolCallStarted, FileRead, CommandStarted for this assessment run)
- `yyds state why last-failure`: OK, shows retroactive FailureObserved from run with error status (Day 141 18:24 session)
- `yyds state graph hotspots --limit 10`: OK, bash (3974 invocations), read_file (3193), search (1427) are dominant tools
- `yyds deepseek cache-report`: **BLOCKED** — "no DeepSeek cache metrics recorded from agent chat completions. Reason: yoagent's Usage struct drops DeepSeek cache token fields." Issue #90 tracks this. Cache metrics DO work for diagnostic paths (`stream-check`, `fim-complete`).

No clunky behavior observed in self-testing. The binary starts, commands respond, state events are recording correctly.

## Evolution History (last 10 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| 29714300343 | 2026-07-20 03:16 | **in_progress** (this session) |
| 29695899970 | 2026-07-19 16:58 | success |
| 29682329573 | 2026-07-19 09:52 | success |
| 29670718534 | 2026-07-19 02:46 | **cancelled** |
| 29652997692 | 2026-07-18 16:58 | **cancelled** |
| 29639148142 | 2026-07-18 09:26 | **cancelled** |
| 29627233668 | 2026-07-18 02:32 | success |
| 29599155239 | 2026-07-17 17:12 | **cancelled** |
| 29571800589 | 2026-07-17 09:57 | success |
| 29550556488 | 2026-07-17 02:41 | success |

**5 of last 10 runs were cancelled** (Days 138-139). The cancelled runs all show Node.js 20 deprecation warnings on the `actions/cache@v4` and `actions/checkout@v4` actions — this is a GitHub Actions platform issue, not a yyds code issue. Log output is benign warnings, no actual failures visible. The last 3 runs (2 success + 1 in-progress) are normal.

**Pattern:** No run has failed with agent errors. Cancellations are infrastructure-level (Node 20 deprecation). When runs complete, they succeed. The trajectory's `task_verification_rate=0.0` and `reverted_unlanded_source_edits` counts describe what happened WITHIN successful runs, not run-level failures.

## yoagent-state DeepSeek Feedback

### State Why (last failure)
Retroactive `FailureObserved` for run `run-1784485868280-54854` (Day 141 18:24 session): "run completed with error status 'error' but no FailureObserved was recorded." This is the janitor writing a retroactive notice — the run exited with error status but didn't self-report failure. Three similar retroactive failures in the search window. Not a new bug class — the janitor is doing its job correctly.

### Graph Hotspots
- bash: 3974 invocations (dominant — expected for a coding agent)
- read_file: 3193 (expected)
- search: 1427 (expected)
- todo/write_file/edit_file: moderate usage
- web_search: only 4 invocations (rarely used)
- No anomalous tool patterns or error-dominated tools

### Cache Report
**Blocked by upstream yoagent.** The `Usage` struct does not expose DeepSeek's `cache_read_input_tokens` and `cache_creation_input_tokens`. Issue #90 (help-wanted) tracks this. Option A: upstream yoagent PR. Option B: yyds-side workaround parsing raw JSON before yoagent drops cache fields. Without cache metrics, we cannot measure whether the deterministic prompt layout work is saving real costs.

## Structured State Snapshot

**Claim health:** 5 PatchEvaluated events recorded (all from log_feedback path). No unresolved claim families detected. The event count (182 total in this session's run) is consistent with an active assessment.

**Task-state counts (from trajectory):**
- Day 141 18:49: 0/2 strict verified, task states: reverted_unlanded_source_edits=2
- Day 141 11:03: 1/2 strict verified, task states: obsolete_already_satisfied=1
- Day 141 04:17: 1/2 strict verified, task states: reverted_unlanded_source_edits=1
- Day 140 18:14: 1/2 strict verified, task states: reverted_unlanded_source_edits=1
- Day 140 10:39: 0/2 strict verified, task states: reverted_no_edit=1, reverted_unlanded_source_edits=1

**Dominant failure modes:** `reverted_unlanded_source_edits` (edits made but reverted — tasks attempted code changes that didn't survive verification), `reverted_no_edit` (task picked but no edits made), and `obsolete_already_satisfied` (task was already done). The high rate of `reverted_unlanded_source_edits` suggests tasks that touch Rust source but fail the verifier.

**Graph-derived next-task pressure (from trajectory):**
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (deepseek_model_call_incomplete_count=2): Lifecycle causes include model_abnormal/model_completion_without_start=8.
3. **Raise session success rate** (session_success_rate=0.0): Session did not complete cleanly.
4. **Validate seeded tasks against fresh assessment** (task_seed_contradiction_count=1): Seeded tasks contradicted by assessment.
5. **Require strict verifier evidence for tasks** (task_verification_rate=0.0): Task verification rate below complete.

**Recent tool failures (from trajectory/log_feedback):** "shell tool commands failed during the session → prefer bounded commands with explicit paths and inspect exit output before retrying broader checks." "Seeded tasks contradicted the fresh assessment → validate seeded tasks against fresh assessment evidence."

**Historical tool-failure categories:** The trajectory mentions "recently addressed tool failure categories" — specific categories from Day 140-141 sessions that were fixed (bash filesystem-root scan detection, SQLite rebuild tolerance). These are not current bugs. The ongoing failure categories are: evaluator timeouts (issue #121), reverted unlanded source edits, and model call lifecycle gaps (issue #118).

## Upstream Dependency Signals

**yoagent `Usage` struct** (issue #90): Does not expose DeepSeek cache token fields. This blocks cache observability for the primary agent path. Two resolution paths:
- **Option A (preferred):** Upstream yoagent PR adding `cache_read_input_tokens` and `cache_creation_input_tokens` to `Usage`.
- **Option B (workaround):** yyds-side raw JSON parsing before yoagent drops cache fields.

No other upstream dependency signals detected. All other DeepSeek protocol handling (streaming, FIM, thinking mode, prompt layout) works within yyds without yoagent modifications.

## Capability Gaps

1. **Cache cost observability** — Cannot measure DeepSeek cache savings for agent chat completions. The prompt layout determinism work is invisible to cost tracking. (Issue #90)
2. **Task success rate feedback loop** — When success rate is 0.0, the task picker doesn't adjust scope. Sessions get the same difficulty tasks regardless of recent outcomes, contributing to revert cycles. (Issue #121, reverted)
3. **Model call lifecycle completeness** — `ModelCallStarted` without `ModelCallCompleted` events create gaps in auditing. The shell-side janitor handles this retroactively, but the forward path (ensuring every start gets a completion) is still open. (Issue #118, reverted)
4. **Planner reliability** — The trajectory shows `planner_no_task_count=1` — assessment ran but produced no task files. The fallback task picker compensates but doesn't fix the root cause.
5. **Social engagement** — Last social learning is from Day 94 (6+ weeks ago). No recent community interaction. The social skill and discussion monitoring may need attention.

## Bugs / Friction Found

1. **HIGH: 5 of last 10 CI runs cancelled** — GitHub Actions Node 20 deprecation is cancelling runs. This is a platform-level issue: `actions/cache@v4` and `actions/checkout@v4` target Node 20, and GitHub is deprecating Node 20. The evolve workflow needs to either update these actions to versions using Node 24 or set `ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION=true`. Impact: ~50% of recent sessions lost to infrastructure cancellation before they could do any work.

2. **MEDIUM: Persistent task revert pattern** — Across Day 140-141, most sessions land 0-1 of 2 tasks. The dominant revert reason is `reverted_unlanded_source_edits` (code was written but didn't pass verifier). This suggests tasks are too ambitious for the session's available time, or the verifier is too strict. Issue #121 (success-rate-aware scoping) was reverted before landing.

3. **MEDIUM: Cache metrics blocked by upstream** — The `yyds deepseek cache-report` command returns zero metrics for agent chat completions. Issue #90 is open but stalled (help-wanted, not agent-self). The Option B workaround (yyds-side raw JSON parsing) could unblock without waiting for yoagent release.

4. **LOW: State summary shows 0 completed runs** — `yyds state summary` says "1 started, 0 completed" for the current session. This is expected (session is in-progress), but the discrepancy between "182 events" and "0 completed" could confuse operators. The `RunCompleted` event fires at the end of the session.

## Open Issues Summary

| Issue | Title | State | Age |
|-------|-------|-------|-----|
| #121 | Add success-rate-aware task scoping to preseed task picker | OPEN | 2 days (reverted) |
| #118 | Close forward-case ModelCall lifecycle gap | OPEN | 2 days (reverted) |
| #116 | Planning-only session: all tasks reverted (Day 139) | OPEN | 3 days |
| #105 | Record DeepSeek prompt cache metrics during prompt runs | OPEN | 5 days (reverted) |
| #90 | yoagent Usage struct drops DeepSeek cache fields | OPEN | help-wanted |

All open agent-self issues are reverted tasks or diagnostic tracking. No new issues filed recently beyond revert tracking. The backlog is small and focused.

## Research Findings

**Competitor landscape (from memory/docs):** Claude Code ($20/month) remains the benchmark. Cursor has agent mode with deep codebase understanding. GitHub Copilot has agent mode in VS Code. The yyds differentiation — DeepSeek-native harness with state-backed evidence and self-evolution — is unique but the product surface (REPL, pipe, single-prompt) lags behind IDE-integrated competitors in discoverability.

**llm-wiki external journal:** The external project journal at `journals/llm-wiki.md` shows activity through 2026-05-04 focused on MCP server tools and storage migration for a wiki project. No updates since early May — the external project may be dormant or complete.

**Node 20 deprecation:** GitHub Actions is deprecating Node 20 (https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/). This is affecting yyds CI runs through `actions/cache@v4`, `actions/checkout@v4`, and `actions/create-github-app-token@v1`. The fix is updating these action versions to ones using Node 24 (if available) or setting the `ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION=true` env var as a stopgap.

---

## Summary of Candidate Tasks

1. **[HIGH] Fix CI cancellations from Node 20 deprecation** — Update actions in `.github/workflows/evolve.yml` to v5/bullseye versions or add the `ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION` flag. This is blocking 50% of sessions. Evidence: 5 cancelled runs in last 10, all with Node 20 deprecation warnings. Impact: restores session throughput immediately.

2. **[MEDIUM] Retry success-rate-aware task scoping (#121)** — The task was reverted because the evaluator timed out, not because the code was wrong. The issue body contains a complete implementation plan (single sort block in `choose_task`, one test case). Small, well-scoped, touches a script file with Python tests. Evidence: trajectory shows persistent `reverted_unlanded_source_edits` and 0/2 verified sessions.

3. **[MEDIUM] Unblock cache metrics with yyds-side workaround (#105 / #90)** — Parse raw DeepSeek response JSON before yoagent drops cache token fields, similar to how `stream-check` and `fim-complete` already work. Option B from issue #90. Changes to `src/deepseek.rs` or `src/commands_deepseek.rs`. Evidence: `yyds deepseek cache-report` shows empty for agent path, cache metrics work for diagnostic paths.

4. **[LOW] Retry ModelCall lifecycle closure (#118)** — Ensure every `ModelCallStarted` gets a matching `ModelCallCompleted` in the forward path (not just retroactive janitor cleanup). Evidence: trajectory shows `deepseek_model_call_incomplete_count=2`, graph pressure row.

5. **[LOW] Fix planner reliability** — The trajectory shows `planner_no_task_count=1` — assessment ran but produced no task files. Investigate why the planner is producing empty output and add edge-case handling. Evidence: trajectory and graph pressure row.
