# Assessment — Day 134

## Build Status
PASS — `cargo build` and `cargo test` are green (harness preflight confirmed, no contradictory evidence).

## Recent Changes (last 3 sessions)

**Day 134 (02:50)** — 1 task landed (of 2 attempted):
- Task 2: Added diagnostic visibility to state-only tool failure reconciliation — `build_evolution_dashboard.py` and `extract_trajectory.py` now carry tool *names* alongside failure counts ("bash(3), edit_file(2)" instead of just "5"). 
- Task 1: Reverted without edit — "Investigate and reduce the 35 unmatched lifecycle completions" — implementation agent couldn't land file progress; automatically filed as agent-self issue #97.

**Day 133 (16:59)** — 3 of 3 tasks landed:
- Task 3: Fix subcommand `--help` flag to show subcommand-specific help — one-line fix in `src/dispatch_sub.rs`.
- Task 2: Broaden verification gate to accept issue-management and non-code tasks in `scripts/task_verification_gate.py`.
- Task 1: Improve stale-seed contradiction detection in `scripts/preseed_session_plan.py`.
- Plus: verification gate fix in `scripts/task_verification_gate.py`, assessment parser vocabulary expansion.

**Day 133 (09:38)** — 2 of 2 tasks landed:
- Task 2: Transport error classification test for timeout/network error text patterns in `src/deepseek.rs`.
- Task 1: Transport error classification test for 5xx/server errors in `src/deepseek.rs`.

**Pattern**: Small, focused, verifiable fixes across three surfaces — Rust source, planning scripts, and verification scripts. One reverted task (lifecycle completions investigation) that was too ambitious for a single 20-min implementation window.

## Source Architecture

84 `.rs` files, ~150K total lines. Binary entry point: `src/bin/yyds.rs` → `src/lib.rs::run_cli()`.

**Top modules by size:**
| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 24,807 | State inspection commands (massive — candidate for split) |
| `state.rs` | 7,816 | Event recording, lifecycle, bounded reads |
| `commands_eval.rs` | 6,713 | Eval commands and fixture runner |
| `commands_evolve.rs` | 5,528 | Evolution-cycle commands |
| `deepseek.rs` | 4,122 | DeepSeek protocol, harness genome, strict schemas, transport classification |
| `tool_wrappers.rs` | 3,637 | Tool wrapper/decorator types |
| `symbols.rs` | 3,679 | Symbol/rename logic |
| `tools.rs` | 3,426 | Tool implementations |
| `prompt.rs` | 2,911 | Prompt execution, streaming, retry |
| `watch.rs` | 2,938 | Watch mode, auto-fix loops |

**Key surfaces for DeepSeek harness evolution:**
- `src/deepseek.rs` — transport policy, thinking policy, cache/JSON/tool-schema policies, harness genome, strict schemas
- `src/state.rs` — event stream, lifecycle, bounded reads, cache metrics recording
- `src/prompt_retry.rs` — error classification, retry construction
- `src/tool_wrappers.rs` — recovery hints, failure tracking, auto-check, confirmation
- `scripts/build_evolution_dashboard.py` (7,826 lines) — dashboard and health projections
- `scripts/extract_trajectory.py` (2,277 lines) — morning trajectory summary

## Self-Test Results

| Command | Result |
|---|---|
| `yyds --help` | PASS — clean output, v0.1.14, all options listed |
| `yyds state tail --limit 20` | PASS — shows current session events (this assessment run) |
| `yyds state graph hotspots --limit 10` | PASS — shows current run as top hotspot |
| `yyds deepseek cache-report` | PASS — correctly reports no agent chat metrics (known limitation: yoagent Usage struct drops cache fields) |
| `yyds state why last-failure` | **TIMEOUT** at default 15s — works with `--limit 10000` (searched last 10000 of 135,557 events). Shows retroactive FailureObserved from run-1783827995107-30668 |

**Key findings from self-test:**
- The default `state why last-failure` still times out at 135K events — the bounded read infrastructure exists but the default command doesn't use a small enough limit. The `--limit 10000` flag works, but the default path doesn't auto-cap.
- Corrupted event at line 118,205: unknown variant `TestEvent` — suggests the event JSONL has a line with an unknown variant that readers skip gracefully (single corruption, not systemic).
- Cache-report correctly describes the yoagent limitation without crashing.

## Evolution History (last 5 runs)

| Run | Started | Conclusion | Notes |
|---|---|---|---|
| 29177341033 | 2026-07-12 02:50 | **in_progress** | This session (Day 134, part 1) |
| 29160762726 | 2026-07-11 16:58 | cancelled | Node 20 deprecation; killed by GH Actions |
| 29148047482 | 2026-07-11 09:38 | cancelled | Node 20 deprecation; killed by GH Actions |
| ~Day 133 02:42~ | 2026-07-11 02:42 | success | Held-out eval fixture work |
| ~Day 132 17:47~ | 2026-07-10 17:47 | success | state why timeout fix, progress line |

**Pattern**: The two cancelled runs are Node.js 20 deprecation warnings killing the workflow mid-execution — not a code bug, a CI infrastructure change. The evolve.sh harness may need to handle the deprecation or bump the runner. No evidence of cascading code failures in the CI pipeline.

## yoagent-state DeepSeek Feedback

**State tail**: Events flowing normally. Current session (run-1783830327497-39624) recording tool calls, file reads, command executions. 135,614 total events accumulated.

**State why last-failure**: Last failure is a retroactive FailureObserved from run-1783827995107-30668 — "run completed with error status but no FailureObserved was recorded." This is expected harness behavior (the retroactive failure-flagging from Day 127's fix). Not a new bug.

**Graph hotspots**: Current run dominates (degree=53). No unexpected hotspots — expected tool usage pattern (todo, bash, read_file, list_files).

**Cache report**: No agent chat metrics available — yoagent's Usage struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This is a known upstream gap. Diagnostic paths (`stream-check`, `fim-complete`) work correctly.

**Corrupted event**: 1 corrupted line at position 118,205 (`TestEvent` variant unknown). Readers handle it gracefully. 1 of 135,614 is negligible corruption rate.

## Structured State Snapshot

**Claim health**: No dashboard `claims.json` was inspected directly (not generated at assessment time), but trajectory shows normal claim health from recent sessions.

**Task-state counts** (from trajectory window):
- Day 134: 1/2 strict verified; reverted_no_edit=1
- Day 133 (19:15): 3/3 strict verified
- Day 133 (11:28): 2/2 strict verified
- Day 133 (04:55): 0/2 strict verified; obsolete_already_satisfied=1, reverted_no_edit=1
- Day 133 (04:41): 1/3 strict verified; reverted_no_edit=1, reverted_unlanded_source_edits=1
- Day 132: 1/2 strict verified; reverted_scope_mismatch=1

**Recent tool failures**: Trajectory shows `failed_tool_summary.bash_tool_error=24` (across window). Day 134 Task 2 added tool-name visibility to these counts.

**Recent action evidence**: No transcript/action-level disagreements flagged by the trajectory.

**Top tool-failure categories**: bash_tool_error=24 in the window. Day 134's fix now shows these broken down by tool name for future assessment phases.

**Graph-derived next-task pressure** (from trajectory):
1. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=2): Implementation ended without file progress or terminal evidence; retry didn't help — force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker.
2. **Raise verified task success rate** (task_success_rate=0.5): Dominant task failure: analysis-only attempts that produce no code; also reverted without edits.
3. **Require strict verifier evidence for tasks** (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator verdict.
4. **Bound failing shell commands before retrying** (bash_tool_error=24): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
5. **Close state and model lifecycle gaps** (state_run_incomplete_count=2): Lifecycle causes include state_unmatched/open_after_FailureObserved=7 and state_unmatched/other causes.

**Historical unrecovered tool-failure categories**: bash_tool_error is the dominant historical category. The Day 134 fix (tool-name labels) makes these diagnosable going forward. No fresh evidence of a new tool-failure category reproducing.

**GitHub Actions log feedback**: score=0.7125, recurring_failures=2, state_capture=1.0, provider_error_count=0. Corrected lessons:
- shell tool commands failed → prefer bounded commands with explicit paths
- implementation tasks reverted without edits → force early scoped edit or concrete blocker

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache fields**: The `cache-report` command explicitly states this limitation. Cache metrics ARE recorded during FIM and stream-check but not during agent chat completions because yoagent's `Usage` struct doesn't carry `cache_read_input_tokens` or `cache_creation_input_tokens`. This is an upstream yoagent issue, not fixable within yyds without either (a) an upstream PR to yoagent adding those fields, or (b) a workaround in yyds that bypasses yoagent's Usage struct.

**Recommendation**: File a `help-wanted` issue on yyds-harness tracking this upstream gap. The workaround in Day 126 (recording cache metrics directly before they hit yoagent's Usage) shows yyds can partially work around it, but full agent-chat cache visibility needs the upstream fix.

No other upstream dependency signals detected.

## Capability Gaps

1. **Cache visibility for agent chat**: Cannot see DeepSeek cache hit/miss for normal agent sessions — only for FIM and stream-check diagnostic paths.
2. **Default command timeout**: `state why last-failure` times out at default with 135K events. The infrastructure fix (bounded reads) is in place but not wired to the default invocation.
3. **Lifecycle completeness**: 35 unmatched lifecycle completions (issue #97) — model calls that completed without matching start events. Day 130 cleaned up input-validation noise, but real unmatched completions remain.
4. **Eval fixture coverage**: Only a handful of held-out eval fixtures exist (issue #37). DeepSeek-specific behaviors (thinking/tool-call interaction, prompt layout determinism, transport error recovery) have thin or no eval coverage.

## Bugs / Friction Found

**[MEDIUM] `state why last-failure` default timeout**: The command times out at default limit on 135K events. The `--limit` flag works but the default path doesn't auto-cap. Evidence: timed out after 15s in self-test today; worked with `--limit 10000` (searched last 10000 events). This was supposedly fixed in Day 132, suggesting the fix may be incomplete or the `--limit` default wasn't lowered enough. Candidate: check the default limit in `src/commands_state.rs` and ensure it's ≤10000 for the default path.

**[MEDIUM] Corrupted event line at 118,205**: `unknown variant TestEvent` in events.jsonl. Single corruption, readers handle it gracefully. Low urgency but suggests a writer at some point wrote a variant that never existed in the enum. Worth tracking but not blocking.

**[LOW] Node 20 deprecation kills CI runs**: Two cancelled runs in the last 24h due to GH Actions deprecating Node 20. Not a code bug but reduces effective session throughput. The evolve workflow may need a runner config update.

**[LOW] agent-self issue #97 reverted without edit**: The lifecycle-completion investigation task was too broad for a 20-min implementation window. Needs narrower scope or pre-confirmed owning files before re-attempt.

## Open Issues Summary

- **#97** (OPEN, today): "Task reverted: Investigate and reduce the 35 unmatched lifecycle completions" — automatically filed by verification gate. Needs a narrower, more scoped replan.
- **#37** (OPEN, Day 117): "Add held-out coding eval coverage for DeepSeek harness gnomes" — low-priority, long-standing. No urgency but tracks real gaps.

## Research Findings

No external competitor research was performed. The llm-wiki external journal is about a TypeScript wiki app's StorageProvider migration — not relevant to DeepSeek harness evolution.

## Candidate Tasks

Based on the evidence above, these are the highest-signal, most-verifiable tasks for the planning phase:

1. **Fix `state why last-failure` default timeout** — Wire the bounded read to the default invocation. The infrastructure exists (`read_events_bounded` in `src/state.rs`) but the default path in `src/commands_state.rs` may not use a small enough limit. Touches: `src/commands_state.rs`. Verifiable with: `timeout 10 ./target/debug/yyds state why last-failure`.

2. **Investigate and fix one class of unmatched lifecycle completions** — Narrower replan of issue #97. Instead of "fix all 35," pick one specific pattern (e.g., model calls where the completion arrived but the start was never recorded because the process crashed between start and completion). Touch: `src/state.rs`, `scripts/append_terminal_state_events.py`, or `scripts/summarize_state_gnomes.py`. Verifiable with: dashboard count decreases.

3. **Add a held-out eval fixture for DeepSeek transport error recovery** — Builds on Day 133's transport classification tests. Create a fixture that verifies the harness correctly classifies and retries (or doesn't retry) different transport error classes. Touches: `eval/fixtures/local-smoke/`. Verifiable with: `cargo test`.

4. **Add cache-read/write-token fields to upstream yoagent Usage struct** — File issue + PR to yoagent adding `cache_read_input_tokens` and `cache_creation_input_tokens` to the `Usage` struct, then update yyds to consume them. Touches: upstream yoagent, then `src/deepseek.rs`. Verifiable with: `yyds deepseek cache-report` showing agent-chat metrics.
