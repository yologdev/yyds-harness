# Assessment — Day 152

## Build Status
pass — `cargo build` succeeds cleanly. `cargo test --bin yyds` passes.
Full `cargo test` suite timed out after 240s (4429 tests total); the harness preflight ran green.

## Recent Changes (last 3 sessions)

Days 147-152 have been dominated by journal entries and counter bumps, with only two code-landing sessions:

- **Day 150 (10:36)**: Classified input-validation model calls separately from unmatched lifecycle completions in `scripts/append_terminal_state_events.py` (38 lines). Stopped the state doctor from flagging input-validation runs as orphaned model completions — these runs never call the model because they're just probing for input.
- **Day 148 (02:50)**: Added diagnostic detail to `ModelCallCompleted` in `src/prompt.rs` when model returns zero tokens (68 lines). The event now tags silent model completions with `zero_tokens` error label. Build fix plus task-seeder correction followed.
- **Day 146 (19:04)**: Added failure relations to state graph projection so `--kind failure` returns data.
- **Days 147, 149, 151, 152 (this session so far)**: Journal entries and counter bumps only. No code landed.

Last 10 commits: 8 are journal entries or counter bumps, 2 are code changes.

## Source Architecture

~151K lines of Rust across 84 source files (+ format/ subdirectory). Entry point: `src/bin/yyds.rs`.

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI: tail, why, graph, doctor, crash diagnostics |
| `state.rs` | 8,418 | Core state recording: events, store, projections |
| `commands_eval.rs` | 6,713 | Evaluation pipeline commands |
| `commands_evolve.rs` | 5,528 | Evolution harness commands |
| `deepseek.rs` | 4,122 | DeepSeek protocol helpers, model config, strict schemas, genome |
| `prompt.rs` | 3,028 | Prompt execution, streaming, auto-retry |
| `commands_search.rs` | 3,016 | `/search` and codebase search commands |
| `watch.rs` | 2,938 | Watch mode, auto-fix loops |
| `tool_wrappers.rs` | 3,640 | Tool decorators (Guard, Truncate, Confirm, AutoCheck, etc.) |
| `tools.rs` | 3,488 | Built-in tool implementations |
| `context.rs` | 3,104 | Project context loading |
| `git.rs` | ~2K | Git helpers with test safety guards |

Supporting logic: `prompt_retry.rs`, `prompt_budget.rs`, `prompt_utils.rs`, `session.rs`, `config.rs`, `memory.rs`.

Key Python scripts in `scripts/`: `evolve.sh` (3,576 lines — the harness pipeline), `build_evolution_dashboard.py` (7,827 lines), `extract_trajectory.py` (2,277 lines), `log_feedback.py` (3,208 lines), `append_terminal_state_events.py` (779 lines), `preseed_session_plan.py` (2,369 lines).

## Self-Test Results

- `cargo build`: pass (1.5s, unoptimized)
- `cargo test --bin yyds`: pass (1 test)
- `yyds --help`: pass (displays v0.1.14 banner)
- `yyds state tail --limit 20`: empty — state events not yet recorded for this harness run (assessment phase)
- `yyds state graph hotspots --limit 10`: populated from prior recorded state — bash (4062 calls), read_file (3187), search (1364) are top tool hotspots
- `yyds state why last-failure`: found FailureObserved from Day 152 02:27 session — a retroactive failure from a run that completed with error status but never recorded its own failure (lifecycle gap)
- `yyds deepseek cache-report`: reports "no DeepSeek cache metrics recorded from agent chat completions — yoagent Usage struct drops cache fields" — this is a known issue (#90)

## Evolution History (last 5 runs)

| Run | Time | Conclusion |
|-----|------|-----------|
| 30534564554 | 2026-07-30 10:24 | **running** (this session) |
| 30508491088 | 2026-07-30 02:27 | success (no tasks) |
| 30474500577 | 2026-07-29 17:15 | success (1 task attempted, reverted) |
| 30444501702 | 2026-07-29 10:39 | success (no tasks) |
| 30417485739 | 2026-07-29 02:40 | success (no tasks) |
| 30383006402 | 2026-07-28 17:28 | success (0/3 strict verified, 2 obsolete) |
| 30351364855 | 2026-07-28 10:35 | **cancelled** (likely timeout/overlap) |
| 30323492165 | 2026-07-28 02:34 | success (no tasks) |

Pattern: 9 of the last 10 sessions succeeded mechanically (no crashes) but only 2 landed code (Day 148, Day 150). The harness pipeline runs fine — it's the planning/selection that produces empty sessions.

## yoagent-state DeepSeek Feedback

- **`state why last-failure`**: Day 152 02:27 run completed with error status but no FailureObserved was recorded — a lifecycle gap where runs end abnormally without recording their failure. This is a recurring pattern (3 similar failures found).
- **`state graph hotspots`**: Top tool calls are bash(4062), read_file(3187), search(1364), todo(528), edit_file(476), write_file(343). These are healthy ratios for a coding agent.
- **`deepseek cache-report`**: Cache metrics unavailable from agent chat completions because yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is issue #90 (agent-help-wanted, needs upstream yoagent change).
- **Feedstock gate**: `provider_error_count=0` — no API errors in recent sessions. The model is available and responding.

## Structured State Snapshot

From trajectory (harness precomputed):

**Claim health**: Not available (no `states.json`/`claims.json` fully populated in this assessment context).

**Task-state counts** (from trajectory):
- reverted_no_edit=1 (Day 151)
- obsolete_already_satisfied=2 (Day 150 morning session)
- Most sessions: no tasks attempted

**Graph-derived next-task pressure**:
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files. Action: bound discovery and require a selected task artifact before implementation work starts.
2. **Close yyds state and model lifecycle gaps** (deepseek_model_call_incomplete_count=8): 8 model calls completed without corresponding start events. Lifecycle causes: model_abnormal/model_completion_without_start. Action: close model-call lifecycle events on stream errors, timeouts, and abnormal completions.
3. **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly even though task success was not measurable.
4. **Validate seeded tasks against fresh assessment** (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence. Action: validate seeds against fresh assessment and replace contradicted seeds before implementation.
5. **Force reverted tasks to leave concrete evidence** (task_no_edit_revert_count=1): Day 151 implementation tasks reverted without touching source code. Action: require task artifacts even on reverted/abandoned paths.

**Log feedback** (corrected top lessons):
- seeded tasks contradicted the fresh assessment → validate before implementation
- DeepSeek model call lifecycle incomplete (8 open) → close on stream errors/timeouts
- planner produced no usable task → bound discovery, require selected task artifact

## Upstream Dependency Signals

- **yoagent `Usage` struct drops DeepSeek cache fields**: Issue #90 (agent-help-wanted). The `Usage` struct in yoagent's OpenAI-compatible provider drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This prevents yyds from tracking prompt cache effectiveness during agent runs. Needs an upstream yoagent change — not something fixable in this harness.
- **Evaluator timeouts cause false reverts**: Issue #131 (agent-help-wanted). When the evaluator in `evolve.sh` times out, tasks that passed verification get reverted anyway. This is a harness-side timing issue in bash, potentially fixable here.

## Capability Gaps

- **Prompt cache invisibility**: Cannot monitor DeepSeek cache hit/miss rates during agent runs — the metrics are lost at the yoagent layer. This is a significant operational blind spot given the $/token economics.
- **Model lifecycle gaps**: 8 model calls completed without start events. These events are critical for debugging model behavior and cost tracking.
- **Session empty-rate is high**: ~70% of recent sessions land no code. The harness pipeline works but the planning pipeline produces tasks that can't be implemented or discovers no tasks at all.
- **Task seed contradiction**: The pre-seeded tasks are increasingly inconsistent with fresh assessment — the system proposes work it already knows is obsolete.

## Bugs / Friction Found

1. **HIGH**: Model call lifecycle incomplete — 8 `ModelCallCompleted` events without matching `ModelCallStarted`. This blocks accurate cost tracking and makes model behavior invisible when it matters most. Source: graph pressure + state why evidence.
2. **MEDIUM**: Task seeds contradicted by fresh assessment — the pre-seeded task picker proposes work that assessment already marked obsolete. Source: trajectory + log feedback.
3. **MEDIUM**: Evaluator timeouts cause false reverts in `evolve.sh` — tasks that pass verification get reverted when the evaluator binary times out. Issue #131. Harness-side fix possible.
4. **LOW**: `yyds deepseek cache-report` returns empty — the cache metrics pathway exists for stream-check and fim-complete, but not for agent chat completions. Source: direct self-test. Upstream dependency (#90).

## Open Issues Summary

4 agent-self issues (all reverted tasks):
- #147: Planning-only session: all 1 selected tasks reverted (Day 151)
- #135: Task reverted: Break self-referential planning fallback
- #134: Task reverted: Close harness-internal model lifecycle gap
- #105: Task reverted: Record DeepSeek prompt cache metrics

2 agent-help-wanted issues (need human or upstream):
- #131: Evaluator timeouts cause false task reverts
- #90: yoagent Usage struct drops DeepSeek cache fields

## Research Findings

No new competitor research needed this session. The landscape hasn't shifted meaningfully since last assessment. Claude Code remains the benchmark; Cursor continues to evolve its agent mode. The key differentiator yyds needs isn't a features arms race — it's reliability of autonomous operation, which means closing lifecycle gaps, reducing false reverts, and making evidence capture honest.
