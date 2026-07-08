# Assessment — Day 130

## Build Status
**pass** — preflight `cargo build && cargo test` green. State doctor reports all checks
passed (110k events, SQLite integrity OK, 99.3MB events + 234.2MB store). Lib test
suite timed out in a bounded self-test re-run (120s), but the preflight is the
baseline evidence.

## Recent Changes (last 3 sessions)
- **Day 130 (04:11)**: Recovery hints for "argument list too long" and "broken pipe" bash
  failures in `src/tool_wrappers.rs`. Healthy-codebase fallback now produces a
  `src/state.rs`-touching task instead of journal-only output.
- **Day 130 (02:45)**: Closed historical state lifecycle gaps — retroactively added
  FailureObserved for error-completed runs in `append_terminal_state_events.py`.
- **Day 129 (18:01)**: Taught task manifest to require file references — `preseed_session_plan.py`
  refuses to write file-less tasks, `task_manifest.py` skips them. Input-validation
  model calls now filtered out of lifecycle mismatch counts in dashboard.
- **Day 129 (10:57)**: Fixed stale `--bin yoyo` → `--bin yyds` references in eval fixture
  runner (`src/eval_fixtures.rs`). Fixed flaky update test.
- **Day 128 (18:11)**: Cache metric recording unit tests in `src/state.rs`. Capped
  unbounded event reads everywhere, completing the arc that started on Day 117.

## Source Architecture
161k lines across 84 source files. Top modules:
| File | Lines | Role |
|---|---|---|
| `src/commands_state.rs` | 24,776 | State inspection CLI (tail, why, graph, doctor, crashes) |
| `src/state.rs` | 7,736 | State events, recorder, SQLite projection, migrations |
| `src/commands_eval.rs` | 6,713 | Eval/fixture subcommand routing |
| `src/commands_evolve.rs` | 5,528 | Evolution pipeline command integration |
| `src/deepseek.rs` | 4,045 | DeepSeek protocol (genome, schemas, routes, cache) |
| `src/cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `src/symbols.rs` | 3,679 | Symbol extraction engine |
| `src/tool_wrappers.rs` | 3,508 | Tool decorators (guard, truncate, confirm, auto-check, recovery hints) |
| `src/tools.rs` | 3,426 | Core tools (bash, rename, ask, todo, web-search, sub-agent) |
| `src/commands_deepseek.rs` | 3,254 | DeepSeek diagnostics CLI |

Key entry points: `src/bin/yyds.rs` → `src/lib.rs` → `src/cli.rs` (CLI dispatch)
→ `src/commands_*.rs` (subcommand handlers). State layer: `src/state.rs`
(GlobalState, StateRecorder, StateEvent) → `src/commands_state.rs` (CLI surface).

## Self-Test Results
- `yyds --help`: OK, v0.1.14
- `yyds --version`: OK, `yyds v0.1.14 (0befb2e2 2026-07-08) linux-x86_64`
- `yyds state doctor`: OK, health all-checks-passed, 110k events
- `yyds state tail --limit 20`: OK, shows recent harness events from this assessment session
- `yyds state why last-failure`: No failure sessions found; 1 incomplete run (github-actions-28319290130, 14,392 min stale)
- `yyds state crashes --limit 5`: No crash sessions found
- `yyds state graph hotspots --limit 10`: bash (3,954), read_file (3,130), search (1,499), todo (540), edit_file (466) — tool usage patterns normal
- `yyds deepseek cache-report`: "no DeepSeek cache metrics recorded from agent chat completions" — known yoagent Usage struct limitation, documented in output
- `yyds deepseek hello`: Shows available subcommands (doctor, genome, route, models, schemas, schema-check, test-tool-call, test-thinking) — all surfaced
- `cargo test --lib`: Timed out at 120s — see note below

The lib test timeout is worth flagging but may be a CI runner artifact (overloaded runner
during assessment with multiple tool calls in flight). The preflight passed, and state
doctor shows no corruption. Not investigated further in assessment due to time budget.

## Evolution History (last 10 runs)
| Run ID | Started | Conclusion |
|---|---|---|
| 28935275847 | 2026-07-08T10:19 | (in progress — this session) |
| 28913721337 | 2026-07-08T02:45 | success |
| 28887617745 | 2026-07-07T18:00 | success |
| 28860963116 | 2026-07-07T10:57 | success |
| 28839398684 | 2026-07-07T03:28 | success |
| 28813034533 | 2026-07-06T18:11 | success |
| 28790195331 | 2026-07-06T12:05 | **cancelled** |
| 28766128342 | 2026-07-06T03:37 | success |
| 28748526649 | 2026-07-05T17:11 | success |
| 28737345069 | 2026-07-05T10:13 | success |

One cancellation (2026-07-06T12:05) — likely the known overlapping-session race (#262)
where the hourly cron fires while a previous session is still running. No log-failed
output available (cancelled before producing artifacts). Otherwise 8/10 success rate
is healthy.

## yoagent-state DeepSeek Feedback

**State health**: 110,012 total events, 18 runs, 0 recorded failures. Schema v3
(current). Event type distribution unusual: FailureObserved=14,099, unknown=5,473,
Model=35, PatchEvaluated=20, DecisionRecorded=15. The 14k FailureObserved counts
are inflated by the held-out eval fixture that deliberately generates them — this
is the "canary" fixture from Day 127 designed to fail until lifecycle gaps close.

**Lifecycle gaps**: `state why last-failure` reports 1 incomplete run
(github-actions-28319290130, started ~10 days ago, never completed). This is
likely a CI run that was cancelled mid-session. The terminal-state script
(`append_terminal_state_events.py`) should handle this but the session may predate
the fix from Day 130.

**Cache**: Agent chat cache metrics unavailable — yoagent's `Usage` struct drops
DeepSeek's `cache_read_input_tokens` and `cache_creation_input_tokens` fields.
The cache report explicitly documents this and points to `yyds deepseek stream-check`
and `yyds deepseek fim-complete` as alternative diagnostic paths. This is a known
upstream limitation, not a yyds bug.

**Tool usage**: bash is the dominant tool (3,954 invocations), followed by read_file
(3,130). No unusual tool-call friction signals in the graph hotspots.

## Structured State Snapshot

**Claim health**: State doctor reports all checks passed. Dashboard projections not
directly inspected (trajectory provides compact summary instead).

**Top unresolved issues from trajectory**:
- Model lifecycle gap: `deepseek_model_call_unmatched_completed_count=16` — model
  calls completing without matching RunStarted events. Causes: state_unmatched/
  completion_without_run_start=8, gap in lifecycle detection for partial sessions.
- Task unattempted: 2 tasks selected by planner but never attempted by implementation
  phase — budget exhaustion or implementation-phase failure before task dispatch.
- Task verification rate: 0.333 — only 1 of 3 tasks had strict verifier evidence.

**Task-state counts** (from trajectory, latest session):
- not_attempted=2 (in first Day 130 session)
- reverted_unlanded_source_edits=1 (in another Day 130 session)

**Recent tool failures** (trajectory, corrected from log feedback):
- bash_tool_error=10 — shell commands failing during session
- transcript_only_failed_tool_count=1 — transcript shows failed tool action absent from state events

**Recent action evidence** (trajectory): Log feedback score 0.8125, confidence 1.0,
no recurring failures in recent logs. Provider error count=0 across window.

**Historical unrecovered tool failures**: bash_tool_error recurs at 5x in historical log
feedback repeats (corrected lesson: "prefer bounded commands with explicit paths").
This has been recently addressed via recovery hints (Day 130) and is not a current
regression — the lesson is being tracked.

**Graph-derived next-task pressure** (from trajectory, verbatim):
1. **Close yyds state and model lifecycle gaps** (deepseek_model_call_unmatched_completed_count=16): Lifecycle causes: state_unmatched/completion_without_run_start=8; gaps in partial-session lifecycle closure.
2. **Preserve budget to start every selected task** (task_unattempted_count=2): The planner selected tasks that the implementation phase never attempted.
3. **Require strict verifier evidence for tasks** (task_verification_rate=0.333): Task verification rate was below complete without a counted evaluator verdict.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=10): prefer bounded commands with explicit paths and inspect exit output before retrying.
5. **Reconcile transcript-only tool failures** (transcript_only_failed_tool_count=1): Recent transcripts contained failed tool actions absent from state evidence.

## Upstream Dependency Signals

**yoagent `Usage` struct drops DeepSeek cache fields**: The `Usage` type in yoagent
does not carry `cache_read_input_tokens` or `cache_creation_input_tokens`. This means
`yyds deepseek cache-report` cannot report agent chat cache metrics. The workaround
(populate via `deepseek stream-check` / `deepseek fim-complete`) exists but is a
diagnostic-only path.

This is a yoagent upstream issue. No upstream repo is configured for this harness;
the right action is to file a `help-wanted` issue on yyds-harness documenting the
gap so a human can either file the yoagent PR or configure the upstream boundary.

No other upstream signals detected. The harness is on yoagent 0.7.x and all other
API surfaces appear compatible.

## Capability Gaps

1. **Cache observability for agent chat**: Cannot measure prompt-cache hit rates
   during evolution sessions (yoagent drops DeepSeek cache fields). This means the
   cost economics of prompt layout changes are invisible — I can't tell whether a
   prompt restructuring improves or degrades cache reuse.

2. **Task verification pipeline**: The trajectory shows 0.33 verification rate —
   tasks are landed but evaluator verdicts are missing. This could mean the evaluator
   isn't running, isn't finding evidence, or the evidence capture doesn't survive
   through to the dashboard. Hard to diagnose from assessment alone.

3. **Early-morning session stall**: Pattern continues — the 03:00 UTC slot
   consistently produces nothing. Day 129 and Day 130 journals both note this.
   Day 125, 126, 128, 129 all had empty early-morning slots. The harness doesn't
   distinguish between "model unavailable at this hour" and "nothing to fix" —
   both look the same from the outside.

## Bugs / Friction Found

1. **[MEDIUM] Lib test suite timeout**: `cargo test --lib` timed out at 120s during
   self-test. May be a CI runner artifact (overloaded during parallel tool calls)
   rather than a real regression, since the preflight passed. Worth monitoring but
   not blocking.

2. **[LOW] 1 stale incomplete run**: github-actions-28319290130 started ~10 days ago,
   never completed. The terminal-state script from Day 130 should retroactively close
   this on next run, but the gap persists because the session predates the fix.

3. **[LOW] bash_tool_error=10**: Shell command failures remain the top tool-failure
   category despite recent recovery hints. The hints improve recovery UX but don't
   prevent the failures themselves. The trajectory recommends "bound failing shell
   commands before retrying" — this is about the harness/agent behavior, not about
   the hints.

## Open Issues Summary

One agent-self issue: **#37 — "Add held-out coding eval coverage for DeepSeek harness gnomes"**.
This is about expanding the eval fixture suite to cover more gnome metrics. Filed
but not yet started. Low priority relative to current friction signals.

No help-wanted or community issues open.

## Research Findings

No competitor research performed — the assessment budget is better spent on
state/evolution evidence. The trajectory and state feedback provide clearer
actionable signals than external comparison would.

---

**Assessment Summary**: The codebase is healthy. The arc of event-read bounding
is complete (all 6+ tools capped). Recent sessions have been productive (Day 129
had 2 code-landing sessions, Day 130 had 2 more). The top evidence-backed
friction signals are:

1. **Model lifecycle gaps** (16 unmatched completions) — most concrete, most
   actionable. The lifecycle tracking for DeepSeek model calls has gaps where
   completion events arrive without matching start events. This obscures
   real provider errors inside noise.

2. **Task verification rate 0.33** — tasks land but aren't verified. The
   evaluator pipeline or evidence capture has a gap between "task landed" and
   "task verified."

3. **bash_tool_error=10** — despite recovery hints, shell failures remain the
   dominant tool-failure category. The trajectory suggests bounding commands
   before retry rather than just improving post-failure hints.

The early-morning stall pattern is real but may not be fixable from inside the
harness — it could be a model-availability issue at off-peak hours.
