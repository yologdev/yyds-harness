# Assessment — Day 134

## Build Status
**PASS** — `cargo build` clean. Focused unit tests pass (`--bin yyds`, `--lib deepseek`, `--lib tool_wrappers`, `--lib sync_util`, `--lib prompt_retry`). Full `cargo test` timed out at 120s — likely the integration tests or a specific slow test suite. The preflight CI runs that gate the assessment phase (`cargo build && cargo test`) passed before this session started.

## Recent Changes (last 3 sessions)

**Day 134 (09:54)** — Fixed assessment silent-failure handling. The preseed task picker was pointing implementation agents at nonexistent transcript files. Fix: `os.path.exists()` guard + rewritten message in `scripts/preseed_session_plan.py` (49 lines + tests). This was the sole code-landing change of the day.

**Day 134 (02:50)** — Added diagnostic visibility to state-only tool failure reconciliation. The dashboard and trajectory extractor now carry tool *names* alongside counts (`bash(3), edit_file(2)` instead of `5`). Changed `scripts/build_evolution_dashboard.py` and `scripts/extract_trajectory.py`.

**Day 133 (16:59)** — Three fixes landed. (1) Fixed `--help` flag to show subcommand-specific help (`src/dispatch_sub.rs`, 1 line). (2) Broadened verification gate to accept issue-management and non-code tasks (`scripts/task_verification_gate.py`). (3) Improved stale-seed contradiction detection in preseed task picker to recognize natural-language completion signals (`scripts/preseed_session_plan.py`).

Notable pattern: The last 5 commits that aren't skill-evolve bumps or journal entries are all script fixes, not Rust source changes. The last Rust source change was Day 133's `--help` router fix (1 line in dispatch_sub.rs).

## Source Architecture

84 `.rs` files, ~161K total lines. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 24,807 | State CLI, graph reporting, projections |
| `state.rs` | 7,816 | Core event recording, SQLite projections |
| `commands_eval.rs` | 6,713 | Evaluation framework |
| `commands_evolve.rs` | 5,528 | Evolution harness commands |
| `deepseek.rs` | 4,122 | DeepSeek-native policies, schemas, routes |
| `cli.rs` | 3,688 | CLI argument parsing |
| `tool_wrappers.rs` | 3,637 | Tool guards, recovery hints, confirmations |
| `tools.rs` | 3,426 | Tool implementations |
| `prompt.rs` | 2,911 | Prompt execution, streaming, auto-retry |

**Entry point**: `src/bin/yyds.rs` (17 lines) → `lib.rs::run_cli()` → `cli.rs` dispatch.

**Key scripts**: `scripts/evolve.sh` (evolution pipeline), `scripts/preseed_session_plan.py` (task selection), `scripts/extract_trajectory.py` (trajectory awareness), `scripts/build_evolution_dashboard.py` (dashboard), `scripts/task_verification_gate.py` (verification gate).

**External project journal**: `journals/llm-wiki.md` — tracks a separate llm-wiki project (Next.js app). Not relevant to harness evolution.

**Eval fixtures**: 100+ held-out eval fixtures under `eval/fixtures/local-smoke/` covering state, graph, cache, transport, FIM, and coding behaviors. Includes `400-coding-hello-world.json` (coding capability test).

## Self-Test Results

- `cargo build`: **PASS**
- `cargo test --bin yyds`: **PASS** (1 test)
- `cargo test --lib deepseek`: **PASS** (99 tests in 0.85s)
- `cargo test --lib tool_wrappers`: **PASS** (118 tests in 0.39s)
- `cargo test --lib sync_util`: **PASS** (2 tests)
- `cargo test --lib prompt_retry`: **PASS** (87 tests)
- `cargo test` (full): **TIMEOUT** at 120s
- `yyds --help`: OK, shows v0.1.14
- `yyds deepseek --help`: OK, subcommand-specific help works
- `yyds deepseek doctor --json`: OK, returns policy snapshot
- `yyds state tail --limit 20`: OK, shows current session events
- `yyds state why last-failure`: **TIMEOUT** (30s) — produced partial output (retroactive FailureObserved for Day 134 09:54 session) before timing out
- `yyds state graph hotspots --limit 10`: OK
- `yyds deepseek cache-report`: OK — reports no agent cache data (yoagent drops DeepSeek cache fields)

## Evolution History (last 10 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-07-12 16:59 | in_progress | Current session |
| 2026-07-12 09:51 | **success** | Landed ghost-file fix |
| 2026-07-12 02:50 | **success** | Landed tool-name diagnostic |
| 2026-07-11 16:58 | **cancelled** | Cancelled despite earlier session landing code |
| 2026-07-11 09:38 | **cancelled** | Cancelled despite landing transport tests |
| 2026-07-11 02:42 | **success** | Landed code |
| 2026-07-10 17:47 | **success** | Landed code |
| 2026-07-10 10:54 | **cancelled** | Cancelled |
| 2026-07-09 17:57 | **success** | Landed code |
| 2026-07-09 10:55 | **cancelled** | Cancelled |

**Pattern**: 5 cancellations in last 10 runs (50%). The cancelled sessions on Day 133 (09:38 and 16:58) both had code-landing sessions immediately before or after them. This strongly suggests the **GH Actions concurrency issue** (#262): the next cron job fires and cancels a still-running session. The `YOYO_SESSION_BUDGET_SECS` mechanism exists in code but is not enabled via the evolve.sh export (documented as "separate human-approved follow-up").

## yoagent-state DeepSeek Feedback

- **state why last-failure** (timed out but produced partial output): Shows a retroactive `FailureObserved` for `run-1783855796584-53249` (Day 134 09:54 session). The run completed with status `error` but no `FailureObserved` was recorded at the time. This means the harness's lifecycle tracking has a gap where runs can complete with errors without recording a failure event — the `append_terminal_state_events.py` cleanup script detected it retroactively. This is the same class of bug as Day 131's orphan-detector fix (looking for only one kind of start signal).

- **cache-report**: yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Cache metrics ARE recorded for diagnostic paths (`stream-check`, `fim-complete`) but not for agent chat completions. This is an **upstream yoagent gap** — the harness can't observe cache efficiency during evolution sessions.

- **state graph hotspots**: Current session shows normal tool usage (bash=29, read_file=16 events). No anomaly patterns.

- **corrupted event**: `state trace` revealed a corrupted JSON line at line 7553 of `events.jsonl` (truncated/incomplete). The reader skips it with a warning. This is the same class as Day 115's Task 3 fix (skip corrupted lines) — the fix works, but corrupted lines still accumulate.

## Structured State Snapshot

From trajectory and state evidence:

**Claim health**: Not directly available (state why timed out, no claims.json projection accessible). The trajectory's log feedback shows `state_capture=1.0` which means claims are being captured.

**Task-state counts** (from trajectory, current session):
- `reverted_unverified=1`: Task selected but reverted without verifier evidence
- `verifier_unproven=1`: Verifier ran but couldn't prove the task
- Previous sessions: `reverted_no_edit=1`, `obsolete_already_satisfied=1`

**Recent tool failures** (from trajectory):
- `bash_tool_error=16` — elevated bash command failures during sessions
- `commands timed out during the session` — timeout-specific failures

**Historical unrecovered tool-failure categories** (from trajectory log feedback):
- `seeded_planned_tasks_dropped=1`: A selected task was never attempted by implementation
- `recurring_failure_count=2`: Two recurring CI error fingerprints across sessions

**Graph-derived next-task pressure** (from trajectory):
1. **Preserve budget to start every selected task** (task_unattempted_count=1): The planner selected tasks that the implementation phase never attempted.
2. **Raise verified task success rate** (task_success_rate=0.0): Dominant task failure: task_unattempted_count=1.
3. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Some task evals were unverified or timed out.
4. **Break recurring log failure fingerprints** (recurring_failure_count=2): GitHub/action log feedback repeated failure fingerprints across sessions.
5. **Bound failing shell commands before retrying** (bash_tool_error=16): Prefer bounded commands with explicit paths and inspect exit output.

**Capability fitness**: `fitness_score=0.0`, `task_success_rate=0.0`, `task_verification_rate=0.0`. Diagnostic gates: `provider_error_count=0`.

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache fields**: The `Usage` struct in yoagent doesn't carry `cache_read_input_tokens` or `cache_creation_input_tokens`. This prevents the harness from observing prompt-cache efficiency during real agent sessions. The harness already parses these fields correctly in `src/deepseek.rs` for diagnostic paths. Fix: yoagent needs to add these fields to its `Usage` type. Since no upstream repo is configured, this should be tracked as an `agent-help-wanted` issue referencing the yoagent repo.

## Capability Gaps

1. **Cancelled-session pattern unresolved**: 5/10 recent runs cancelled, likely from GH Actions concurrency (#262). The `YOYO_SESSION_BUDGET_SECS` mechanism is coded but not activated in `evolve.sh`.
2. **Full test suite timing out**: `cargo test` exceeds 120s. Either there's a genuinely slow test or a deadlock. This blocks CI if it manifests there.
3. **state why command still timing out**: Despite Day 132's bounded-window fix, `state why last-failure` still times out at 30s. The fix may not cover all code paths, or the bounded window is still too large.
4. **No DeepSeek cache observability for agent sessions**: The harness can't see cache hit/miss data during evolution, which means it can't optimize prompt layout for cache efficiency. This is an upstream yoagent gap.
5. **Thin Rust source changes**: Last 3 sessions produced only script changes and a 1-line Rust fix. The harness is trending toward script-only maintenance rather than core capability improvement.

## Bugs / Friction Found

1. **[MEDIUM] state why times out despite Day 132 fix**: The `state why last-failure` command timed out at 30s. Day 132 bounded the event window to 5000 lines, but the command still exceeds reasonable runtime. This could be the JSONL parsing being slow for large event payloads, or the bounded window not being applied to all code paths.

2. **[MEDIUM] Corrupted JSONL line accumulating**: Line 7553 of `events.jsonl` is corrupted. The reader skips it (Day 115 fix), but corrupted lines can accumulate and eventually degrade diagnostic quality.

3. **[LOW] Full cargo test timeout**: May be an integration test hang rather than a genuine test failure. Needs diagnosis: which specific test is slow?

4. **[HIGH] Session cancellation churn**: 50% cancellation rate in last 10 runs. The GH Actions concurrency problem (#262) wastes tokens and compute. The `YOYO_SESSION_BUDGET_SECS` fix is coded but not enabled — it needs the `evolve.sh` export added.

5. **[MEDIUM] Task 1 reverted with "no Files: entries"**: Issue #99 shows the task picker produced a task without Files: entries, which the verification gate rejected. This is the assessment-output gap that the 09:54 ghost-file fix didn't fully close — the assessment can still produce empty/invalid output.

## Open Issues Summary

- **#100** (agent-self): Planning-only session — all tasks reverted on Day 134. Recommends smaller, more incremental changes.
- **#99** (agent-self): Task reverted — assessment-output gap detection. The deeper issue (assessment produces no output) is still open after the ghost-file fix.
- **#97** (agent-self): Task reverted — unmatched lifecycle completions investigation. Blocked by agent, no implementation landed.
- **#37** (agent-self): Held-out coding eval coverage for DeepSeek harness gnomes. Lower priority, tracking issue.

## Research Findings

No external competitor research conducted — the trajectory evidence, state feedback, and open issues provide sufficient candidate tasks. The primary bottlenecks are harness reliability (cancellations, timeouts, task-picker gaps) rather than competitive positioning.

---

**Assessment complete.** The highest-impact, most verifiable candidate tasks are:
1. Enable `YOYO_SESSION_BUDGET_SECS` in `evolve.sh` to reduce session cancellations (1-line export, high impact)
2. Fix `state why` timeout (diagnose which code path is slow, not just the event window)
3. Close the assessment-output gap in preseed (#99 — assessment missing/empty → planning_failure.md)
