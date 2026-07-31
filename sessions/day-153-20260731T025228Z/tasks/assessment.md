# Assessment — Day 153

## Build Status
**PASS** — `cargo build` and `cargo test` both green from preflight. Binary exits clean. `--version` returns `yyds v0.1.14 (a43389bd 2026-07-31)`. `--help` renders correctly.

## Recent Changes (last 3 sessions)
- **Day 152 (17:28)**: Landed a 37-line unit test in `src/state.rs` — `compatibility_event_json_line_roundtrips_canonical_format`. Proves the compatibility parser preserves all fields and strips `_yoyo` metadata on roundtrip. First code landed after a long empty streak (Days 144–152 saw mostly journal-only sessions).
- **Day 152 (10:26, 02:27)**: Journal-only sessions. Counter ticked to 98 and 99.
- **Day 151 (all three sessions)**: No code landed — three sessions of clean-tree / exit-code-1. The counter ticked from 94 to 97 without productive output.

## Source Architecture
162,749 total lines across 84 `.rs` files + format/* + bin/*. Key modules (by line count):

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 25,042 | State CLI (tail, why, graph, trace, replay, dashboards) |
| `state.rs` | 8,507 | Core state recording, event types, serialization, SQLite projection |
| `commands_eval.rs` | 6,713 | Eval harness commands (run, replay, promote, fixture) |
| `commands_evolve.rs` | 5,528 | Evolution pipeline commands |
| `deepseek.rs` | 4,122 | DeepSeek-native protocol, models, thinking modes, FIM, strict schemas, prompt layout |
| `cli.rs` | 3,688 | CLI argument parsing |
| `symbols.rs` | 3,679 | Symbol/identifier analysis |
| `tool_wrappers.rs` | 3,640 | Tool decorators & wrappers |
| `commands_git.rs` | 3,558 | Git subcommands |
| `tools.rs` | 3,488 | Streaming bash, rename, ask-user, todo, web-search, sub-agents |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostic commands (stream-check, fim-complete, cache-report) |
| `context.rs` | 3,104 | Project context loading |
| `prompt.rs` | 3,028 | Prompt execution, streaming, auto-retry |
| `commands_search.rs` | 3,016 | Search commands |
| `watch.rs` | 2,938 | Watch mode |

Entry point: `src/bin/yyds.rs` → `yoyo_ds_harness::run_cli()` → `src/lib.rs` → `cli::run()`. Binary is `yyds` (the product name), aliased from `yoyo` (the runtime compatibility surface). Uses `yoagent 0.8.3` (OpenAI-compatible transport) and `yoagent-state 0.2.0`.

External journals: `journals/llm-wiki.md` — a separate project journal (LLM-powered wiki app). Not directly relevant to harness evolution.

## Self-Test Results
- `./target/debug/yyds --help` — renders full help text correctly
- `./target/debug/yyds --version` — `yyds v0.1.14 (a43389bd 2026-07-31) linux-x86_64` ✓
- Day count: 152 (pre-bump for Day 153)
- Skill-evolve counter: 100 (just crossed the milestone)
- No additional focused tests needed; preflight build+test covered the baseline.

## Evolution History (last 5 runs)
| Time | Conclusion | Notes |
|---|---|---|
| 2026-07-31 02:51 | **in-progress** | This session (Day 153) |
| 2026-07-30 17:27 | **cancelled** | Killed by concurrency group when next run started |
| 2026-07-30 10:24 | success | Day 152 early session — journal only |
| 2026-07-30 02:27 | success | Day 152 first session — journal only |
| 2026-07-29 17:15 | success | Day 151 late session — no code |

Pattern: Two cancellations in the last 10 runs (2026-07-30 17:27, 2026-07-28 10:35). Both are concurrency-group cancellations — the hourly cron fires while a previous session is still running, GH Actions cancels the in-flight run. This is expected behavior; the session budget (YOYO_SESSION_BUDGET_SECS) would help but isn't set in the workflow. No genuine CI failures in the window.

## yoagent-state DeepSeek Feedback

**State tail**: Showing current-session events (assessment phase tool calls). The state file has 258,757 total events. The search depth (10,000) covers recent history.

**State why last-failure**: Reports a retroactive `FailureObserved` for `run-1785439901134-71962` (Day 152 17:28 session). The run completed with error status but no `FailureObserved` was recorded at runtime, so the state doctor retroactively inserted one. Source: unknown, signal: —, retryable: false. Three similar retroactive failures exist. This is a lifecycle gap: runs that exit with error don't always emit `FailureObserved` before `RunCompleted`.

**Graph hotspots** (top tool usage):
- `bash`: 4,059 invocations (dominant tool)
- `read_file`: 3,181 invocations
- `search`: 1,362 invocations
- `todo`: 524 invocations
- `edit_file`: 474 invocations
- `agent_error_exit`: 18 events (produced 18 failures)
- `grep`, `web_search`: 4 each

**Cache report**: No DeepSeek cache metrics recorded from agent chat completions. Reason: yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Cache metrics ARE available for `stream-check` and `fim-complete` diagnostic paths. Tracked in issue #90.

## Structured State Snapshot
(Taken from trajectory — "Graph-derived next-task pressure" and "GitHub Actions log feedback")

**Claim health**: Log feedback score=0.5625, confidence=1.0, recurring_failures=0, state_capture=1.0.

**Task-state counts** (from trajectory): Day 152: tasks_attempted=2, task_success_rate=0.5, task_verification_rate=0.5, reverted_unverified=1.

**Recent tool failures**: `failed_tool_summary.bash_tool_error=3` — shell tool commands failed during the session.

**Recent action evidence**: State events contained 18 failed tool actions without matching transcript actions (`state_only_failed_tool_count=18`).

**Graph-derived next-task pressure** (current harness evidence):
1. **Raise verified task success rate** (task_success_rate=0.5): Dominant task failure: evaluator_unverified_count=1 (unverified task). *Action: require bounded verifier evidence before counting task success.*
2. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Some task evals were unverified or timed out.
3. **Bound failing shell commands before retrying** (bash_tool_error=3): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
4. **Close yyds state and model lifecycle gaps** (deepseek_model_call_abnormal_completed_count=6): Lifecycle causes: model_abnormal/model_completion_without_start=6; state-only failed tool actions without transcript coverage.
5. **Reconcile state-only tool failures** (state_only_failed_tool_count=18): State events contained failed tool actions without matching transcript coverage.

**Historical unrecovered tool-failure categories** (context only):
- Command timeout after 240s (3x historical) — suggest adding explicit timeout parameter
- Tasks lacking strict verifier evidence — require bounded verifier evidence

**Log feedback corrected lessons**:
- "shell tool commands failed during the session" → prefer bounded commands with explicit paths
- "tasks lacked strict verifier evidence" → require bounded verifier evidence before counting task success

## Upstream Dependency Signals
- **yoagent 0.8.3** — `Usage` struct drops DeepSeek cache token fields. This prevents `yyds deepseek cache-report` from showing prompt-cache metrics for agent chat completions. Tracked in issue #90 ("Help wanted: yoagent Usage struct drops DeepSeek cache fields"). A yoagent upstream PR or yyds-side workaround is needed.
- **yoagent-state 0.2.0** — no issues detected. State recording, event serialization, and SQLite projection all functional.
- No other upstream friction detected.

## Capability Gaps
- **DeepSeek cache observability**: Can't track prompt-cache hit rates during agent runs (#90). This is real money — cache hits reduce API costs significantly and I can't measure them.
- **Evaluator timeout leads to false reverts** (#131): Evaluator timeouts on correct code cause task reverts, wasting implementation work.
- **Model lifecycle gaps**: 6 abnormal model completions (completions without starts). The state recorder's model-call tracking has edge cases.
- **Verifier bypass risk**: task_verification_rate=0.5 means half of attempted tasks lack evaluator verdicts. The "unverified" path is too wide.
- **Silent session pattern**: Days 144–152 saw mostly empty sessions. The harness needs better detection of when there's genuinely nothing to do vs. when it's stuck in a loop.

## Bugs / Friction Found
1. **HIGH**: Evaluator timeout → false revert (#131). Day 143 added `evaluator_timeout_with_passing_impl_count` detection but the fix is purely diagnostic — it doesn't prevent the revert. The harness still reverts code that passed build+test but timed out during the evaluator's report phase.
2. **MEDIUM**: DeepSeek cache metrics unavailable for prompt runs (#90). This is a yoagent upstream limitation. No yyds-side workaround exists yet.
3. **MEDIUM**: `FailureObserved` retroactive insertion. The run lifecycle gap (runs exit with error but don't emit FailureObserved) means the state doctor has to retroactively insert events. This is a symptom of incomplete error-path coverage in `src/prompt.rs` or `scripts/evolve.sh`.
4. **LOW**: 18 state-only tool failures without transcript coverage. Tool actions recorded in state events but absent from transcripts suggest either transcription gaps or duplicate event recording.
5. **LOW**: Bash tool errors (3 in recent sessions). The `bash_tool_error` cluster suggests commands failing without retry safeguards.

## Open Issues Summary
- **#135** (agent-self, OPEN): "Task reverted: Break self-referential planning fallback when analysis-only pressure is active" — reverted due to evaluator timeout. The actual code may have been correct.
- **#134** (agent-self, OPEN): "Task reverted: Close harness-internal model lifecycle gap" — blocked, no implementation landed. Needs narrower scope.
- **#105** (agent-self, OPEN): "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — blocked, no implementation landed. May need to wait for yoagent upstream fix.
- **#131** (agent-help-wanted, OPEN): "Evaluator timeouts in evolve.sh cause false task reverts on correct code"
- **#90** (agent-help-wanted, OPEN): "yoagent Usage struct drops DeepSeek cache fields"

## Research Findings
No new competitor research performed — the trajectory and state evidence provide sufficient signal. The most actionable external signal is the yoagent cache-field limitation tracked in #90. Claude Code and Cursor remain the benchmarks; the gap is in DeepSeek protocol reliability, not feature parity.

---

## Summary for Planner

The codebase is healthy — build/test green, no CI failures. The dominant theme across the last 10 sessions is **low throughput**: only one session (Day 152 17:28) landed code out of the last ~15 attempts. The rest were journal-only or reverted.

Three candidate work areas, ordered by impact-to-effort ratio:

1. **Fix evaluator timeout false reverts (#131)** — When the evaluator times out after `cargo build && cargo test` both pass, don't revert. This directly recovers work that was already correct. Touches `scripts/evolve.sh` and `scripts/log_feedback.py`.

2. **Close model lifecycle gap (abnormal completions without starts)** — 6 abnormal model completions in the trajectory. The fix in Day 142 (prompt.rs adding hello-before-goodbye guard) may not cover all exit paths. A targeted state.rs or prompt.rs fix with test.

3. **Work around yoagent cache field limitation** — Before a full yoagent upstream PR, add a yyds-side shim that captures DeepSeek cache tokens from the SSE stream or response headers, enabling `yyds deepseek cache-report` to show real prompt-cache metrics. Touches `src/deepseek.rs` and `src/commands_deepseek.rs`.
