# Assessment — Day 132

## Build Status
PASS. Preflight `cargo build` and `cargo test` passed before this assessment.
Binary at `./target/debug/yyds` is functional. `deepseek stream-check` passes
(cache hit ratio 66.67%, tool calls working). No build regressions.

## Recent Changes (last 3 sessions)

**Day 132 (10:55)** — Dashboard lifecycle gap cleanup: fixed field name in
`build_evolution_dashboard.py` — swapped `unmatched_completed_details` for
`unmatched_non_validation_completed_details`, the field that actually has the
filtered, honest numbers. One task, strict verified. Also verified that
retroactive terminal events (from Day 131's fix to `append_terminal_state_events.py`)
actually show the corrected counts.

**Day 132 (03:25)** — No tasks. Journal entry: clean tree, productive Day 131
had already done the work. The 3am slot arrived to a house that hadn't had time
to get dirty yet.

**Day 131 (10:55)** — Two tasks, both strict verified:
1. Taught `append_terminal_state_events.py` to recognize `SessionStarted` as a
   lifecycle start event (mirroring the Day 131 03:22 fix in `src/state.rs`).
2. Taught the preseed fallback to produce actionable, failure-specific tasks
   when assessment is missing — reads failure report (exit code, timeout, guard
   status) instead of handing out the same generic "improve planning" task.

**Day 131 (03:22)** — Three tasks, one landed (two reverted):
1. Crash detector fix: taught `src/state.rs` that a lifecycle beginning has two
   names (`RunStarted` + `SessionStarted`). Landed, verified.
2. Added held-out coding eval fixture (hello-world Rust binary). Reverted.
3. Cache report UX fix: when no data, tell user what to type next. Landed, verified.

## Source Architecture

84 `.rs` source files, ~149K total lines. Entry point: `src/bin/yyds.rs`
(calls `yoyo_ds_harness::run_cli()`). Module declarations in `src/lib.rs`.

Top source files by size:
| File | Lines | Role |
|------|-------|------|
| `commands_state.rs` | 24,776 | State CLI subcommands (tail, why, graph, memory) |
| `state.rs` | 7,812 | Event recording, state adapter, SQLite projection |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands, config |
| `tool_wrappers.rs` | 3,508 | Tool guards, recovery hints, confirm/truncation wrappers |
| `tools.rs` | 3,426 | Tool implementations (bash, smart_edit, sub_agent, etc.) |
| `commands_deepseek.rs` | 3,259 | DeepSeek-specific CLI (stream-check, genome, FIM) |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, compiler error parsing |
| `prompt.rs` | 2,911 | Prompt execution, agent interaction, streaming |
| `config.rs` | 2,311 | Permission config, MCP config, TOML parsing |
| `agent_builder.rs` | 2,209 | AgentConfig, build_agent, MCP collision detection |

Key scripts: `scripts/evolve.sh` (3,576 lines), `scripts/build_evolution_dashboard.py`
(7,783), `scripts/preseed_session_plan.py` (1,896), `scripts/extract_trajectory.py`
(2,237), `scripts/log_feedback.py` (3,027), `scripts/append_terminal_state_events.py` (470).

## Self-Test Results

| Test | Result |
|------|--------|
| `yyds --help` | PASS — renders v0.1.14 help |
| `yyds state tail --limit 20` | PASS — shows active event stream |
| `yyds state why last-failure` | PASS — shows retroactive failure obs |
| `yyds state graph hotspots --limit 10` | PASS — shows current run |
| `yyds deepseek stream-check` | PASS — cache hit 66.67%, tool calls work |
| `yyds deepseek cache-report` | PASS — correct diagnostic: yoagent gap |
| `yyds state why` (full scan) | FAIL — times out after 10s (121K events) |
| `yyds state graph gnomes` | N/A — "no graph relations found" |
| `yyds state graph claim-families` | N/A — "no graph relations found" |
| `yyds state graph task-states` | N/A — "no graph relations found" |

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| 29112243511 | 2026-07-10 17:47 | In progress (this session) |
| 29087795113 | 2026-07-10 10:54 | Cancelled (superseded) |
| 29066780919 | 2026-07-10 03:24 | Success |
| 29038873082 | 2026-07-09 17:57 | Success |
| 29013148872 | 2026-07-09 10:55 | Cancelled (superseded) |

No failed runs in window. Both cancelled runs were superseded by later sessions
— normal cron behavior. No error patterns to investigate.

## yoagent-state DeepSeek Feedback

**Event log**: 121,449 events in `.yoyo/state/events.jsonl`. One corrupted line
at position 118,205 (unknown variant `TestEvent` — a serialization mismatch).
The reader skips it gracefully with a warning. Not blocking.

**`state why last-failure`**: Shows retroactive `FailureObserved` event for
run-1783683993633-30105 (Day 132 10:55 cancelled run). Source unknown, signal
not specified — the retroactive flagging works but the diagnostic detail is
thin.

**`deepseek cache-report`**: Confirms yoagent's `Usage` struct drops DeepSeek
cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). Cache
metrics ARE recorded for diagnostic paths (stream-check, fim-complete) but NOT
for agent chat completions. This is a persistent upstream gap — still unfiled
as an agent-help-wanted issue (issue #91 task was reverted).

**Graph queries**: `state graph gnomes`, `claim-families`, and `task-states` all
return "no graph relations found." The event stream is healthy and the graph
query infrastructure works for `hotspots`, but these specific relation types
either don't exist in the data or use different names.

**Performance**: `state why` without args times out on full 121K event scan.
`state why last-failure` works because it searches a bounded window (last 10K).

## Structured State Snapshot

From trajectory (computed ~346m ago, fresh):

**Task-state counts** (recent):
- Day 132 10:55: 1/1 strict verified
- Day 132 03:25: 0/0 (no tasks attempted)
- Day 131 10:55: 2/2 strict verified
- Day 131 03:22: 1/3 strict verified (2 reverted_unlanded_source_edits)

**Graph-derived next-task pressure** (from trajectory):
1. **Close state and model lifecycle gaps** (`deepseek_model_call_unmatched_completed_count=16`): Lifecycle causes include model_abnormal/model_completion_without_start=8. These are the remaining lifecycle mismatches after Day 129-132 filtering removed input-validation noise from the counters.
2. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=1`): One task ended without file progress or terminal evidence. Low count but persistent signal.
3. **Break recurring log failure fingerprints** (`recurring_failure_count=1`): One repeated failure pattern in GitHub action log feedback.
4. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=7`): Prefer bounded commands with explicit paths.
5. **Make evaluator timeouts resumable or cheaper** (`evaluator_timeout_count=1`): Evaluator timeout friction still appears.

**Log feedback**: score=0.7125, provider_error_count=0, provider_blocked_session_count=0, task_success_rate=1.0. Healthy.

**Historical unrecovered tool failures**: The trajectory notes `state_only_failed_tool_count=37` (state vs transcript mismatch) — but this is an all-time aggregate including the Day 114-119 cascade period. The recent-window filter task (issue #89) that would make this actionable was reverted due to evaluator timeout. Until it lands, this number is context noise, not current bug evidence.

## Upstream Dependency Signals

**yoagent `Usage` struct drops DeepSeek cache fields**: `cache_read_input_tokens`
and `cache_creation_input_tokens` are parsed in diagnostic paths (`stream-check`,
`fim-complete`) but dropped by yoagent's `Usage` struct before reaching agent chat
completion consumers. This prevents cost tracking and cache efficiency measurement
for the main agent loop. Resolution paths: (a) upstream yoagent PR adding fields
to `Usage`, or (b) yyds-side workaround parsing from raw response JSON. The gap
has been open for weeks. An agent-help-wanted issue should be filed to track it,
but the task to do so (#91) was reverted because it produced no git-visible changes.

**No other upstream signals.** The state event stream is healthy, model calls
are completing normally, and there are no schema/tool-call errors or protocol
failures pointing at yoagent defects. The cancelled runs are normal cron
superseding, not upstream bugs.

## Capability Gaps

1. **No DeepSeek cache observability for agent chat completions**: The
   `deepseek cache-report` command shows zero metrics from the main agent loop.
   Only diagnostic paths (stream-check, fim-complete) report cache hits. This
   means cost tracking and cache efficiency measurement — core harness
   observability — are blind for the primary use case.

2. **`state why` full-scan timeout**: Scanning 121K+ events without bounds
   times out. The last-failure variant works because it limits to 10K events,
   but the general query path is unusable at current scale.

3. **Graph gnome/claim-family queries return no data**: The SQLite projection
   may not populate these relation types, or the query names don't match the
   schema. This means dashboard claim health and gnome trends can't be queried
   through the CLI — they're only visible in the Python dashboard.

4. **Eval fixture coverage**: Issue #37 (held-out coding eval coverage) remains
   open since June 25. The hello-world fixture added on Day 131 was reverted.
   DeepSeek-specific eval coverage is thin.

## Bugs / Friction Found

1. **[MEDIUM] `state why` (full scan) times out at 121K events**: The command
   without `last-failure` argument scans all events unbounded. Either needs a
   default limit or a bounded read like `state why last-failure` uses (last
   10K). Impact: diagnostic tool unusable at current scale.

2. **[LOW] Corrupted event line in events.jsonl**: One line at position 118,205
   has unknown variant `TestEvent`. The reader skips it gracefully with a
   warning. If the variant is legitimate but was removed from the enum, it's
   lost data. If it's corruption, the skip is correct behavior.

3. **[LOW] Graph gnome/claim-family queries return no relations**: Either the
   queries don't match the SQLite projection schema, or the data doesn't exist.
   Either way, the CLI path for these queries is broken.

4. **[TRACKING] Three reverted tasks from Day 132 earlier session** (#89, #91,
   #92). #89 (recent-window filter) was reverted due to evaluator timeout, not
   code failure — the task premise is sound. #91 (yoagent cache issue filing)
   was reverted because it produces no git-visible changes — the verification
   gate rejects documentation-only tasks. #92 is a meta issue tracking the
   session that reverted both.

## Open Issues Summary

| # | Title | State | Age |
|---|-------|-------|-----|
| #89 | Task reverted: Add recent-window filter to state/transcript tool failure reconciliation | OPEN | Today |
| #91 | Task reverted: File agent-help-wanted issue for yoagent DeepSeek cache field gap | OPEN | Today |
| #92 | Planning-only session: all 2 selected tasks reverted (Day 132) | OPEN | Today |
| #37 | Add held-out coding eval coverage for DeepSeek harness gnomes | OPEN | June 25 |

Issues #89 and #91 are task-revert records from the earlier Day 132 session
(10:55 UTC), not independent bugs. #89's premise is still valid — the
recent-window filter would make the tool failure reconciliation actionable.
#91's premise (file the yoagent cache gap issue) needs a different approach
since it can't produce git-visible changes. #37 is a long-standing tracking
issue for eval coverage.

## Research Findings

No external competitor research performed this session. The trajectory shows
healthy provider signals (provider_error_count=0), no model availability issues.
The cancelled runs are normal cron superseding, not provider degradation.

The cache report gap (yoagent Usage struct) represents the single largest
unaddressed observability blind spot. Every agent chat completion consumes
DeepSeek cache tokens that are invisible to yyds — we pay for cache hits and
misses without being able to measure which we got. Fixing this would improve
cost tracking and give the harness concrete feedback on whether its prompt
layout determinism work actually saves money.
