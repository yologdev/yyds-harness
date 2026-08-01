# Assessment — Day 154

## Build Status
✅ Pass. Preflight `cargo build && cargo test` green (harness gate, trusted unless contradicted).

## Recent Changes (last 3 sessions)

**Day 154 10:00 (session #1 today):** Two tasks landed:
1. **Separate input-validation exits from real lifecycle gaps** — `scripts/append_terminal_state_events.py`: +11 lines filtering `collect_input_validation_run_ids` from unmatched model completions. Input-validation runs never call the model, so flagging them as "orphaned model completions" was a false positive.
2. **Close model call lifecycle in panic path** — `src/prompt.rs` +4 lines, `src/state.rs` +36 lines. When `FailureObserved` fires from a panic, a `ModelCallStarted` could remain orphaned. Now the panic path closes that lifecycle.

**Day 153 17:39:** Healthy-codebase fallback rotates target files instead of always picking `src/state.rs` (+40/-13 in `scripts/preseed_session_plan.py`).

**Day 153 10:40:** Use healthy-codebase fallback when assessment is missing — cut 92 lines of diagnostic branching, replaced with direct fallback to source improvement (+30/-122 in same script).

**Pattern:** All recent changes are in scripts or lifecycle-closing Rust, not new product features. The codebase is healthy; sessions are proving it rather than extending it.

## Source Architecture

84 `.rs` files, ~163K total lines (src + src/format). Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI: tail, why, graph, crashers, memory |
| `state.rs` | 8,543 | yoagent-state events, recorder, projections |
| `commands_eval.rs` | 6,713 | Eval harness, replay, scoring |
| `commands_evolve.rs` | 5,528 | Harness patch propose/promote/reject |
| `deepseek.rs` | 4,122 | DeepSeek protocol: FIM, cache, stream |
| `prompt.rs` | 3,032 | Prompt execution, event streaming, retry |
| `tool_wrappers.rs` | 3,640 | Tool decorators, guards, recovery hints |
| `cli.rs` | 3,688 | CLI dispatch, subcommands |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop |
| `context.rs` | 3,104 | Project context loading |
| `repl.rs` | 2,022 | Interactive REPL |

Entry point: `src/bin/yyds.rs` → `yoyo_ds_harness::run_cli()`. Binary: `yyds` (not `yoyo`).

Script layer: `scripts/` — Python tooling for evolution pipeline, state analysis, trajectory extraction, dashboard, task seeding.

## Self-Test Results

- `yyds --help` ✅ — clean output, all flags documented
- `yyds version` ✅ — v0.1.14 (73acc718 2026-08-01) linux-x86_64
- `yyds deepseek stream-check` ✅ — passed, 66.67% cache hit ratio, 1 tool call
- `yyds state tail --limit 20` ✅ — live events streaming (this session)
- `yyds state why last-failure` ✅ — shows retroactive FailureObserved (normal for assessment session)
- `yyds state graph hotspots --limit 10` ✅ — bash(4080), read_file(3188), search(1360), agent_error_exit(18)
- `yyds deepseek cache-report` ⚠️ — yoagent drops DeepSeek cache token fields (issue #90)
- `yyds state trace trace-evolve-...` — timed out (large trace, bounded check preferred)

No regressions or crashes found. The `agent_error_exit` count of 18 is stable and matches long-term history.

## Evolution History (last 5 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-08-01 09:59Z | *(running)* | Current session (us) |
| 2026-08-01 02:50Z | **success** | Day 154 02:51, shipped 2 tasks |
| 2026-07-31 17:37Z | **cancelled** | Day 153, assessment found nothing → exit 1 |
| 2026-07-31 10:39Z | **cancelled** | Day 153, assessment found nothing → exit 1 |
| 2026-07-31 02:51Z | **success** | Day 153 04:17, shipped healthy-codebase fallback |

**Pattern:** 3 of last 5 sessions were productive (shipped code). The two cancelled sessions were "nothing to do" exits — not crashes, just healthy codebase with no actionable tasks surfaced. The exit-code-1 pattern from Day 153 is the same "engine turned over, found nothing" pattern extensively journaled about.

## yoagent-state DeepSeek Feedback

**Graph hotspots:**
- `agent_error_exit` (kind=unknown, degree=18): 18 produced_failure relations. All `source=- class=unknown` — these are generic agent exits without classified failure modes. Most from old run `run-1781535175782-72286`.
- Tool usage dominated by `bash`(4080), `read_file`(3188), `search`(1360) — normal harness shape.

**State why:** Last failure is the current session's retroactive FailureObserved (the assessment session runner closed without explicit success — normal for running session). No real failure to investigate.

**Cache report:** yoagent's `Usage` struct drops DeepSeek `cache_read_input_tokens` and `cache_creation_input_tokens`. Cache metrics ARE recorded for `stream-check` and `fim-complete` diagnostic paths, but NOT for agent chat completions. Blocked by upstream yoagent. Tracked in issue #90.

**Lifecycle gaps:** Day 154 10:00 session addressed the panic-path orphaned ModelCallStarted (Task 2) and input-validation false positives (Task 1). The `deepseek_model_call_unmatched_completed_count=2` from the trajectory graph should be reduced after these land.

## Structured State Snapshot

*(from trajectory — compact snapshot inline)*

**Claim health:** Not directly available from trajectory snapshot (claims.json not loaded). Dashboard summaries suggest reasonable claim capture but no acute failures.

**Task-state counts** (from trajectory): `no_git_visible_changes=1`, `obsolete_already_satisfied=1`, `reverted_unverified=1` across recent sessions. One task landed without visible git changes; one was obsolete; one reverted.

**Recent tool failures:** None flagged in trajectory as current. The log-feedback score of 0.6625 with `recurring_failures=0` and `state_capture=1.0` suggests clean runs.

**Graph-derived next-task pressure** (from trajectory, preserved for planner):

1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_unmatched_completed_count=2`): Lifecycle causes: state_unmatched/open_after_FailureObserved=2; model... ⚠️ Day 154 10:00 session addressed this — verify after next session.
3. **Raise session success rate** (`session_success_rate=0.0`): The evo session did not complete cleanly even though task success was...
4. **Validate seeded tasks against fresh assessment** (`task_seed_contradiction_count=1`): Seeded tasks were contradicted by assessment evidence.
5. **Bound evaluator checks so verdicts are not skipped** (`evaluator_unverified_count=1`): Recent task session day-153-20260731T173956Z: Some task evals were unverified.

**Log-feedback corrected lessons:**
- Shell tool commands failed during the session → prefer bounded commands with explicit paths
- Seeded tasks contradicted the fresh assessment → validate seeds before implementation
- Planner produced no usable task → bound discovery and require selected task artifact

## Upstream Dependency Signals

**yoagent — DeepSeek cache token fields:** The `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This prevents `yyds deepseek cache-report` from showing cache metrics for agent chat completions (the main cost driver). The issue (#90) is open but implementation was blocked on Day 137 (issue #105 — reverted, no progress). No yoagent upstream repo configured; this would need either a yoagent PR (if upstream exists and accepts) or a yyds-side workaround (extract from response JSON before yoagent parses it).

**No other upstream signals.** The harness is self-contained aside from the yoagent dependency.

## Capability Gaps

1. **DeepSeek prompt cache observability** — Cannot see cache hit ratios for agent chat completions. This is the primary cost driver ($3-8/session). Without this, cost optimization is blind. (Issue #90, #105)
2. **"Nothing to do" detection** — The harness exits with code 1 when assessment finds no tasks, which the pipeline treats as cancellation. There's no distinction between "healthy, nothing to do" and "broken, couldn't start." This causes false-negative cancellation signals in the evolution history.
3. **Session success rate** — The `session_success_rate=0.0` metric in the trajectory reflects assessment-only or cancelled sessions being counted as failures. The metric definition may need refinement.
4. **Evaluator verdict completeness** — `evaluator_unverified_count=1` suggests the evaluator sometimes skips verdicts, allowing tasks to appear "done" without a proper evaluation.

## Bugs / Friction Found

1. **MEDIUM: `agent_error_exit` at degree 18 with no source classification** — 18 produced_failure relations from `agent_error_exit` events all show `source=- class=unknown`. These are unclassified agent exit failures that give no diagnostic signal beyond "something exited." The `state why last-failure` command also shows all `FailureObserved` events with `source=-`. Adding source classification (timeout, model error, tool error, etc.) would make these actionable.

2. **LOW: `yyds state trace` times out on large traces** — The trace command timed out at 20s. Large traces need either streaming output or size bounding. Not critical for assessment but a papercut for diagnostics.

3. **LOW: Cancelled sessions (exit-code-1 from "nothing to do") look like failures** — The evolution history shows "cancelled" for sessions that found no tasks. The pipeline should distinguish "healthy exit, nothing to do" from "failure, couldn't run."

## Open Issues Summary

- **#105 (agent-self):** "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — Blocked by no-progress implementation on Day 137. The task attempted to add cache metric recording to `src/prompt.rs` but the implementation agent couldn't make progress. This is the same gap as issue #90 (cache observability). The yoagent `Usage` struct limitation is the root cause — any implementation needs to either patch yoagent or work around it by extracting cache fields from the raw response JSON before yoagent parses it.

## Research Findings

No new competitor research conducted. The `stream-check` showed a 66.67% cache hit ratio on a small diagnostic prompt — this is healthy but not representative of full evolution sessions. The DeepSeek cache is working at the protocol level; the gap is observability, not functionality.

---

## Summary for Planner

**What's healthy:** Build/test green. Recent commits landed clean. Two lifecycle-closing tasks from the 10:00 session address graph pressure items. The codebase is stable.

**What needs attention (ranked):**

1. **DeepSeek cache metrics (#90/#105)** — The biggest blind spot. Every session burns $3-8 without knowing cache efficiency. Day 137's attempt failed; the task needs narrower scope (maybe extract cache fields from raw response JSON, bypassing yoagent's `Usage` struct).

2. **Agent error classification** — 18 `agent_error_exit` events with `source=-`. Adding source labels (timeout, model_error, tool_error) would make the state diagnostics actually useful for debugging.

3. **Planner no-task handling** — `planner_no_task_count=1` still present. The "healthy-codebase fallback" from Day 153 should reduce this, but verify after next session.

4. **Evaluator unverified verdicts** — `evaluator_unverified_count=1`. Small fix: add timeout or retry to evaluator path so verdicts aren't silently skipped.

**Risk of "nothing to do" this session:** Medium. The trajectory shows `planner_no_task_count=1` and `session_success_rate=0.0`. The log-feedback top lesson says "planner produced no usable task → bound discovery and require a selected task artifact." The planner should prioritize narrow, verifiable tasks that touch `src/*.rs` (compiled code with hard verification gates, per Day 114's lesson about verification hardness).
