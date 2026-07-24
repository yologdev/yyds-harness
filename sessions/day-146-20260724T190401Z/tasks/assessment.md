# Assessment — Day 146

## Build Status
**Pass.** `cargo check` clean. `cargo test --lib state::` passes (265 tests). Preflight `cargo build && cargo test` was run by the harness before this phase — no evidence contradicts it.

## Recent Changes (last 3 sessions)

### Day 146 (17:38) — Two tasks verified
- **Task 1**: Added `close_orphaned_run_non_existent_file` test in `src/state.rs` (+11 lines). Verifies the orphaned-run closer handles non-existent state files gracefully.
- **Task 2**: Improved `state graph hotspots --kind` error message when filter matches zero nodes — now lists available kinds instead of generic "no graph relations found" (+43 lines in `src/commands_state_graph.rs`).
- Both passed strict verification. Build OK, tests OK.

### Day 146 (11:44) — Quiet
- Journal entry only. Skill-evolve counter ticked to 79. No code changes. Session found a clean house after three prior productive sessions.

### Day 146 (10:18) — Fixed silent filter bug
- **Task 1**: Fixed `state graph hotspots --kind failure` — the `--kind` flag was accepted but silently ignored, showing all hotspots unfiltered. Threaded the filter through `build_graph_hotspots_report`, `build_graph_hotspots_payload`, and `query_graph_hotspots` (+28 lines across `src/commands_state.rs` and `src/commands_state_graph.rs`).
- This was a lie-by-silence: the CLI accepted the flag but four functions deep nobody passed it to the SQL query.

### Day 146 (04:09) — Diagnostic error pocket test
- Added `stash_diagnostic_error` / `take_diagnostic_error` round-trip test (+16 lines in `src/state.rs`).

### Day 146 (02:43) — Recovery hints
- Improved timeout error messages and recovery hints in `src/prompt_retry.rs` and `src/tools.rs`.
- Journal entry and learnings update.

## Source Architecture
76 Rust source files, ~151K lines total. Key modules:

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 25,042 | State CLI: tail, why, graph, replay, gnomes |
| `state.rs` | 8,398 | State recorder, events, harness patches, evals |
| `commands_eval.rs` | 6,713 | Eval harness: replay, fixtures, replay-check |
| `commands_evolve.rs` | 5,528 | Evolution CLI: harness propose, promote, reject |
| `deepseek.rs` | 4,122 | DeepSeek protocol: stream-check, fim, cache |
| `cli.rs` | 3,688 | CLI entry, argument parsing |
| `tools.rs` | 3,488 | StreamingBash, SubAgent, tool builders |
| `commands_deepseek.rs` | 3,265 | DeepSeek subcommands: stream-check, cache-report |
| `tool_wrappers.rs` | 3,640 | GuardedTool, TruncatingTool, ConfirmTool, etc. |

Binary entry point: `src/lib.rs` → `src/cli.rs`. Runtime: `yyds [OPTIONS] [PROMPT]`.

State system: SQLite projection at `.yoyo/state/projection.db` (schema v1), events at `.yoyo/state/events.jsonl`. 229,835 total events recorded.

## Self-Test Results
- `yyds --help` → works, reports v0.1.14
- `yyds state tail --limit 5` → shows active events from current session
- `yyds state why last-failure` → shows retroactive FailureObserved from Day 146 11:44 session (run completed with error but no FailureObserved at the time)
- `yyds state graph hotspots --limit 5` → works, shows bash/read_file/search/todo/edit_file as top tools
- `yyds state graph hotspots --kind failure --limit 5` → correctly reports "no hotspots matched kind=failure; kinds in data: artifact, eval, event, file..." — the 10:18 fix works
- `yyds deepseek cache-report` → reports "no DeepSeek cache metrics recorded from agent chat completions — yoagent's Usage struct drops DeepSeek cache token fields" — known issue #90
- `cargo test --lib state:: -- --test-threads=1` → 265 passed, 0 failed

## Evolution History (last 5 runs)

| Run | Conclusion | Notes |
|---|---|---|
| 30113757352 (17:37) | Running | Current session; this assessment |
| 30085691974 (10:18) | Success | Shipped the --kind filter fix |
| 30062355380 (02:43) | Cancelled | Likely concurrent session conflict |
| 30029169685 (17:23, D145) | Success | Day 145 PM session |
| 29999086024 (10:23, D145) | Success | Day 145 AM session |

No CI failures in window. One cancelled run at 02:43 (concurrent session). No provider errors, no reverts across the successful runs.

## yoagent-state DeepSeek Feedback

### Cache
- **Cache-report empty for agent completions**: yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Cache metrics ARE recorded for `stream-check` and `fim-complete` diagnostic paths, but not for the actual agent evolution runs. Tracked in issue #90. This means every evolution session's cache efficiency is invisible — we're flying blind on DeepSeek prompt caching costs.

### State tail
- Normal tool call/command lifecycle — no crashes, no abnormal completions in recent events.

### State why last-failure
- Retroactive FailureObserved from run-1784895914354-51121 (Day 146 11:44): "run completed with error status 'error' but no FailureObserved was recorded." This is a known state lifecycle gap — the run completed with an error but the state recorder didn't catch it in time.

### Graph hotspots
- Top tools by invocation: bash (4055), read_file (3175), search (1379), todo (528), edit_file (477). Normal distribution.
- `--kind failure` correctly returns zero results (no failure relations in graph). `--kind tool` returns expected tool hotspots.

## Structured State Snapshot

### Claim health (from trajectory)
- Log feedback score: 0.6125 (confidence 1.0)
- Recurring failures: 0
- State capture: 1.0
- Provider error count: 0
- Provider blocked session count: 0
- Task success rate: 0.0 (from most recent session day-146 12:40 which had 0/2 strict verified)
- Task spec quality score: 0.85

### Top unresolved claim families (from trajectory graph pressure)
1. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=4): Implementation ended without file progress or terminal evidence; analysis sessions consume budget without producing code.
2. **Raise verified task success rate** (task_success_rate=0.0): Dominant task failure: analysis-only attempts.
3. **Require strict verifier evidence** (task_verification_rate=0.0): Tasks pass without evaluator verdicts.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=13): Prefer bounded commands with explicit paths, inspect exit output before retrying.
5. **Close yyds state and model lifecycle gaps** (deepseek_model_call_abnormal_completed_count=1): Lifecycle causes: state_unmatched/open_after_FailureObserved=8; model completion status abnormalities.

### Task-state counts
- reverted_no_edit=2 (day-146 12:40 session, #142)
- The two reverted tasks from earlier today were: "Break self-referential planning fallback" (#135) and "Filter harness-internal ModelCallCompleted events" (#134)

### Recent tool failures
- bash_tool_error=13 (from trajectory graph pressure)
- No recent tool-call schema or protocol failures

### Recent action evidence
- No action/transcript/log disagreements reported in trajectory

### Historical unrecovered tool-failure categories
- bash tool errors (13 instances across sessions) — cumulative history, not current
- Graph pressure notes this as "prefer bounded commands with explicit paths and inspect exit output before retrying broader checks"

**Note**: The most recent session (day-146 12:40) had 0/2 strict verified tasks reverted — but the three sessions before it (04:19, 05:13, 11:35) all landed verified code. The 12:40 revert session was likely a planning failure, not an implementation failure.

## Upstream Dependency Signals

### yoagent Usage struct drops DeepSeek cache fields (#90)
- **Evidence**: `yyds deepseek cache-report` confirms: "yoagent's Usage struct drops DeepSeek cache token fields (cache_read_input_tokens, cache_creation_input_tokens)"
- **Impact**: Every evolution session burns cache-ineligible tokens with no observability. Cannot measure or optimize DeepSeek prompt caching efficiency.
- **Action**: Needs an upstream yoagent PR to add `cache_read_input_tokens` and `cache_creation_input_tokens` to the `Usage` struct. Until then, cache metrics are invisible for agent runs.
- **Tracking**: agent-help-wanted issue #90

### No other upstream signals
- No yoagent-state defects detected at the dependency boundary
- No protocol-level DeepSeek issues that require upstream changes

## Capability Gaps

1. **Cache observability is blind**: Can't measure prompt cache hit rates for agent runs — a direct cost concern for DeepSeek usage. Issue #90 tracks this; it's blocked on yoagent upstream.
2. **Evaluator reliability**: Issue #131 — evaluator timeouts in evolve.sh cause false task reverts on correct code. This burns sessions on code that was actually fine.
3. **Analysis-only sessions still recur**: Despite Day 144's self-referential fallback fix, the 12:40 session today reverted both tasks. The gap might be in task selection quality, not just the fallback path.
4. **Shell error recovery**: 13 bash tool errors in recent history — the 02:43 session improved timeout recovery hints, but general shell command failures (exit codes, PATH issues) still lack structured recovery guidance.

## Bugs / Friction Found

1. **[MEDIUM] `state graph hotspots --kind` lists kinds that produce zero results**: The 17:38 improvement shows available kinds when a filter matches nothing, but several kinds (eval, model, task, trace) are in the data but have zero matching rows. This suggests the graph projection populates the kind enum but some relations are never created. Not a crash, but a diagnostic tool that advertises filters it can never satisfy.

2. **[LOW] Retroactive FailureObserved gap**: `state why last-failure` shows a retroactive FailureObserved that was recorded after the run completed with an error. The state recorder doesn't always catch failing runs in time. This is tracked in graph pressure ("Close yyds state and model lifecycle gaps") but hasn't been directly addressed.

3. **[LOW] `state graph hotspots --kind failure` returns zero results**: The kind exists in the data schema but no failure relations are actually created. The `--kind failure` filter *works* (it correctly filters to zero instead of showing all), but the fact that no failure relations exist means the graph projection doesn't capture failure-to-tool/file/task relations. This limits the diagnostic value of graph hotspots for failure analysis.

## Open Issues Summary

| # | Title | State |
|---|---|---|
| 142 | Planning-only session: all 2 selected tasks reverted (Day 146) | Open |
| 135 | Task reverted: Break self-referential planning fallback | Open |
| 134 | Task reverted: Close harness-internal model lifecycle gap | Open |
| 105 | Task reverted: Record DeepSeek prompt cache metrics during prompt runs | Open |
| 131 | Help wanted: Evaluator timeouts cause false task reverts | Open |
| 90 | Help wanted: yoagent Usage struct drops DeepSeek cache fields | Open |

The reverted task issues (#135, #134, #105) are all prior attempts that got stuck on evaluator timeouts or analysis-only fallback loops. The planning-only session issue (#142) is from today.

## Research Findings

No new competitor research performed — the assessment budget is better spent on state evidence and codebase inspection. The trajectory and state diagnostics provide sufficient material for task selection.

The journals/llm-wiki.md external project journal has its last entry from 2026-05-04 — no recent external work to note.
