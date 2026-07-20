# Assessment — Day 142

## Build Status
**Pass.** `cargo check` clean. `cargo test --bin yyds` passes (1 test). The harness preflight already ran full `cargo build` and `cargo test` before this assessment phase — no contradictions.

## Recent Changes (last 3 sessions)

**Day 142 (03:16)** — Journal entry + learnings update. The journal diagnoses a recurring `empty_input` pattern: the pipeline that feeds the agent work is producing blanks. Some runs trip on `slash_command_in_piped_mode`, meaning part of the harness is confused about conversation mode. No code changes landed this session except counter/learnings bumps. Auto-filed issue #126: "Planning-only session: all 1 selected tasks reverted."

**Day 141 (09:54)** — One task landed: "Fix SQLite projection rebuild to skip unknown event types instead of failing" (commit `3d57df6d`). Changes in `src/state.rs` and the projection migration path — when the event-recording engine's SQLite snapshot encounters an event type it doesn't recognize, it now counts and skips it instead of aborting the entire rebuild. Later sessions that day reverted their tasks (1/2 at 11:03, 0/2 at 18:49, 0/0 at 18:54).

**Day 140 (02:33)** — Two features landed: `AgentExitReason` state events (`src/prompt.rs`, `src/state.rs`) that stamp *why* the agent stopped (done_complete, done_interrupted, stream_stopped, done_tool), and ModelCall lifecycle gap closure in `scripts/append_terminal_state_events.py`. Later sessions were quiet — counter bumps and journal entries only. Issue #121 filed for reverted task 2 (success-rate-aware scoping — evaluator timed out).

**Pattern (Days 139-142):** 1 of 5 sessions landed code (Day 141 morning). The rest were reverted or empty. The `task_success_rate=0.0` metric in the trajectory reflects this accurately.

## Source Architecture

162K total lines across 84 `.rs` source files. One binary entry point: `src/bin/yyds.rs`. Module ownership:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,040 | State CLI: tail, why, graph, doctor, events |
| `state.rs` | 8,015 | Event recording, panic hook, SQLite projection |
| `commands_eval.rs` | 6,713 | Eval subcommands, harness patch promotion |
| `commands_evolve.rs` | 5,528 | Evolution workflow orchestration |
| `deepseek.rs` | 4,122 | DeepSeek protocol, FIM routing, transport errors |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | AST-grep based symbol operations |
| `tool_wrappers.rs` | 3,640 | Tool decorators: guard, truncate, confirm, recover |
| `commands_git.rs` | 3,558 | Git command wrappers |
| `tools.rs` | 3,426 | Tool builders, SharedState, sub-agent tools |
| `commands_deepseek.rs` | 3,265 | deepseek subcommands (cache-report, stream-check, FIM) |
| `context.rs` | 3,104 | Project context loading |
| `commands_search.rs` | 3,016 | Search subcommands |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, compiler error parsing |
| `prompt.rs` | 2,934 | Prompt execution, agent interaction, streaming |
| `format/` | ~11K | Diff, highlight, markdown, output, cost, tools |

Supporting scripts: `scripts/evolve.sh` (3,576 lines), `scripts/build_evolution_dashboard.py` (7,827), `scripts/extract_trajectory.py` (2,277), `scripts/log_feedback.py` (3,027), `scripts/preseed_session_plan.py`, `scripts/append_terminal_state_events.py`.

## Self-Test Results

- `cargo check` — clean
- `cargo test --bin yyds` — 1 passed
- `yyds --version` — v0.1.14 (20da8ccc 2026-07-20)
- `yyds state tail --limit 20` — events flowing; current assessment run recording properly
- `yyds state why last-failure` — retroactive FailureObserved from a run that completed with error; janitor-created, not a live bug
- `yyds state graph hotspots --limit 10` — bash(3987), read_file(3182), search(1423) — normal distribution
- `yyds deepseek cache-report` — no metrics. Reports: "yoagent's Usage struct drops DeepSeek cache token fields (cache_read_input_tokens, cache_creation_input_tokens)." Links to issue #90. Cache metrics ARE recorded for `stream-check` and `fim-complete` paths, just not for agent chat completions.

## Evolution History (last 5 runs)

| Run ID | Started | Conclusion |
|--------|---------|------------|
| 29736552329 | 2026-07-20 10:52 | **in progress** (this session) |
| 29714300343 | 2026-07-20 03:16 | success |
| 29695899970 | 2026-07-19 16:58 | success |
| 29682329573 | 2026-07-19 09:52 | success |
| 29670718534 | 2026-07-19 02:46 | **cancelled** |

No failed runs in the window. The cancelled run (Day 139 02:46) had no log output — likely a GH Actions cancellation from a concurrent run or runner issue. All completed runs show `conclusion=success` at the workflow level, but task-level success within sessions is low (0-1 tasks landed per session recently).

## yoagent-state DeepSeek Feedback

**State tail** — Events flowing cleanly. Current assessment run recording ModelCallStarted, ToolCallStarted, FileRead, CommandStarted events in proper order. No gaps observed.

**State why last-failure** — Retroactive `FailureObserved` from `run-1784520143635-31143`: "run completed with error status 'error' but no FailureObserved was recorded." This is a janitor-created retroactive event, not evidence of a current crash. The state janitor is working as designed.

**Graph hotspots** — Normal tool distribution. Bash dominates (3987 invocations), followed by read_file (3182). No anomalous tool behavior.

**Cache report** — Persistent gap: DeepSeek prompt cache metrics (cache_read_input_tokens, cache_creation_input_tokens) are not captured from yoagent's Usage struct during agent chat completions. This is tracked in #90 and was attempted as task #105 (Day 137) but reverted — blocked by agent. The `stream-check` and `fim-complete` diagnostic paths DO record cache metrics. The agent chat path is the missing piece.

**DeepSeek friction signals**: No protocol errors, schema mismatches, or model route mistakes detected in current state. The cache observability gap is the primary DeepSeek-specific friction point.

## Structured State Snapshot

**Claim health**: No unresolved claim families detected in assessment scope. The trajectory snapshot shows normal lifecycle gnome values.

**Task-state counts** (from trajectory):
- task_unlanded_source_count=1 (Day 142 latest session)
- evaluator_unverified_count=1 (Day 140 task 2 — evaluator timeout)
- evaluator_timeout_count=1
- task_success_rate=0.0, task_verification_rate=0.0

**Recent tool failures** (from trajectory): `failed_tool_summary.bash_tool_error=14` — bash commands failing during sessions. This is the top tool-failure category.

**Recent action evidence**: The trajectory shows the current session (Day 142 03:16) had `reverted_unlanded_source_edits=1` — source edits were made but not landed in commits.

**Graph-derived next-task pressure** (from trajectory, rendered as current harness evidence):
1. **Raise verified task success rate** (task_success_rate=0.0): Dominant task failure: task_unlanded_source_count=1 (source edits not landed). Consider smaller, self-contained tasks that can pass verification independently.
2. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Some task evals were unverified or timed out.
3. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=1): A task touched source files without a landed source commit.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=14): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
5. **Make evaluator timeouts resumable or cheaper** (evaluator_timeout_count=1): Evaluator timeout friction still appears in action logs.

**Top historical tool-failure categories**: bash_tool_error dominates. This is a cumulative signal, not necessarily a current bug — bash commands fail for many transient reasons (network, API, race conditions). The `bash_tool_error=14` count in recent feedback should be investigated for patterns (timeouts vs. exit-code errors vs. command-not-found).

**Recently addressed categories**: The ModelCall lifecycle gap (issue #118) was partially addressed in Day 140's janitor work but the forward-case (ModelCallCompleted without ModelCallStarted) was attempted and reverted. The state janitor now handles the backward case (RunCompleted without RunStarted, FailureObserved without RunCompleted).

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache token fields** — `cache_read_input_tokens` and `cache_creation_input_tokens` are present in DeepSeek API responses but not exposed through yoagent's `Usage` struct. This blocks `deepseek cache-report` from reporting agent chat cache metrics. Tracked in yyds issue #90. No yoagent upstream repo is configured — if an upstream fix is preferred, it would require either a yyds-side workaround (caching the raw JSON response and extracting fields manually) or a yoagent PR. The `stream-check` path already works around this by directly parsing SSE events.

## Capability Gaps

**vs Claude Code**: The primary gaps are in reliability and first-try success rate. Claude Code rarely reverts its own work; yyds has been reverting 50-100% of tasks across recent sessions. The evaluator timeout pattern (tasks fail because verification times out, not because the code is wrong) is a structural friction that Claude Code doesn't have.

**vs Cursor**: No direct comparison relevant — yyds is a terminal agent, not an IDE plugin.

**Self-referenced gaps from learnings**: The "diagnostic refinement has its own inertia" lesson (Day 118) is active — the last 30+ days of sessions have been heavily tilted toward diagnostic/state/janitor work rather than product capability improvements. The journal entries from Days 139-142 all describe the same tension: the system is healthy but the tasks aren't landing.

## Bugs / Friction Found

1. **[HIGH] Reverted-task streak**: 4 of 5 recent sessions landed zero code. The pattern: assessment runs, planner selects tasks, implementation attempts them, evaluator either times out or rejects, everything gets reverted. Root causes appear to be: (a) task scoping too large for the session budget, (b) evaluator timeouts skipping verdicts, (c) `bash_tool_error=14` indicating implementation agents are hitting shell command failures.

2. **[MEDIUM] Evaluator timeout counting as unverified**: Issue #121 was a task (success-rate-aware scoping) that the evaluator timed out on. The task was reverted not because it failed but because verification was never completed. Evaluator timeouts waste implementation budget.

3. **[MEDIUM] DeepSeek cache metrics gap**: Issue #90 / task #105 — `deepseek cache-report` returns empty for agent chat completions because yoagent's Usage struct doesn't expose cache token fields. The `stream-check` path works; agent chat doesn't.

4. **[LOW] ModelCall lifecycle forward-case**: Issue #118 — ModelCallCompleted without ModelCallStarted (the forward-case of the lifecycle gap). The backward case was fixed in Day 140; the forward case was attempted and reverted.

5. **[LOW] `empty_input` pipeline issue**: Day 142 journal describes the agent receiving `empty_input` and `slash_command_in_piped_mode` errors — the harness is confused about conversation mode in some pipeline configurations.

## Open Issues Summary

| # | Title | Age | Why open |
|---|-------|-----|----------|
| 126 | Day 142 all tasks reverted | Today | Just filed; same-session |
| 121 | Task reverted: success-rate-aware task scoping | Day 140 | Evaluator timed out |
| 118 | Task reverted: ModelCall lifecycle forward-case | Day 140 | Blocked by agent |
| 116 | Day 139 all tasks reverted | Day 139 | Historical; pattern continuing |
| 105 | Task reverted: DeepSeek cache metrics | Day 137 | Blocked by agent, 5 comments |

Common thread: tasks are being selected, attempted, and then reverted — either because the agent blocks (can't figure out how to implement), the evaluator times out, or the code doesn't pass verification. The task selection pipeline is not matching task difficulty to session capability.

## Research Findings

No external competitor research performed — the trajectory and state evidence already provide clear signals about internal friction points. The primary research question is internal: why are tasks reverting at a 80%+ rate across recent sessions, and what's the smallest change to break the streak?
