# Assessment — Day 144

## Build Status
**Pass.** Preflight `cargo build && cargo test` passed before this assessment. State doctor confirms: events=209K, projection in-sync, SQLite integrity OK, all health checks passing.

## Recent Changes (last 3 sessions)

| Session | What Changed |
|---------|-------------|
| Day 144 (02:42) | Journal entry, skill-evolve counter bump (70). No code changes — clean house. |
| Day 143 (18:47) | Tests for evaluator-timeout-with-evidence detection: 110 lines of unit + integration tests for the three helper functions added earlier that day (`scripts/log_feedback.py`). Also a learnings update. |
| Day 143 (17:22) | Evaluator-timeout-with-evidence detection: 71 lines in `scripts/log_feedback.py`. New helpers `_cargo_build_passed`, `_cargo_test_passed`, `_implementation_passed_build_and_test` scan transcripts for passing build/test output before evaluator timeouts. New metric: `evaluator_timeout_with_passing_impl_count`. |
| Day 143 (10:26) | Two fixes: (a) Orphan-sweeper in `src/state.rs` now closes ALL dangling `FailureObserved` runs, not just the most recent one (263 lines). (b) Success-rate-aware candidate filtering in `scripts/preseed_session_plan.py` (38 lines) — task picker learns from candidate failure history. |

All Day 143 changes were to Python scripts (diagnostic tools), not Rust source. No `src/` Rust changes landed across the last 3 sessions.

## Source Architecture

**84 source files, ~162K lines**, 77 modules declared in `src/lib.rs` (2006 lines).

**Entry point**: `src/bin/yyds.rs` (16-line thin wrapper → `run_cli()`).

**Key modules by size**:
- `commands_state.rs` (25K) — state tail/doctor/why/graph CLI commands
- `state.rs` (8.3K) — event recording, SQLite projection, lifecycle management
- `commands_eval.rs` (6.7K) — evaluation and harness patch commands
- `commands_evolve.rs` (5.5K) — evolution session orchestration
- `deepseek.rs` (4.1K) — DeepSeek protocol layer, cache metrics, FIM routing
- `cli.rs` (3.7K) — CLI argument parsing
- `symbols.rs` (3.7K) — AST/symbol search
- `tool_wrappers.rs` (3.6K) — tool decorators (guards, confirm, truncation)
- `tools.rs` (3.5K) — built-in tool implementations (bash, search, rename, etc.)
- `commands_deepseek.rs` (3.3K) — `deepseek` subcommand handlers
- `context.rs` (3.1K) — project context loading
- `prompt.rs` (3.0K) — prompt execution, streaming, auto-retry

**Python scripts**: `scripts/evolve.sh` (3.6K), `scripts/log_feedback.py` (3.2K), `scripts/preseed_session_plan.py` (2.4K), `scripts/extract_trajectory.py` (2.3K), `scripts/build_evolution_dashboard.py` (7.8K), `scripts/append_terminal_state_events.py` (742 lines).

**External journal**: `journals/llm-wiki.md` (542 lines) — an LLM Wiki project growth journal, not yyds's own. Last entry 2026-05-04.

## Self-Test Results

- `state doctor`: ✓ All checks passed. 209K events, 242 runs, projection in-sync.
- `state tail --limit 20`: ✓ Works, shows current session events flowing.
- `deepseek stream-check`: ✓ Passed. Cache hit ratio 66.67%.
- `deepseek cache-report`: Reports "no DeepSeek cache metrics recorded from agent chat completions" — known gap, see issue #105.
- Focused test (`cargo test state_cache`): Filtered to 0 matching — test name doesn't exist with that filter. Build passes.

## Evolution History (last 10 runs)

| # | Run ID | Started | Conclusion |
|---|--------|---------|------------|
| 1 | 29911835365 | 2026-07-22 10:25 | **in_progress** (this session) |
| 2 | 29886507833 | 2026-07-22 02:42 | success |
| 3 | 29852517541 | 2026-07-21 17:20 | success |
| 4 | 29822178792 | 2026-07-21 10:25 | **cancelled** |
| 5 | 29796636892 | 2026-07-21 02:44 | success |
| 6 | 29766144597 | 2026-07-20 18:03 | **cancelled** |
| 7 | 29736552329 | 2026-07-20 10:52 | success |
| 8 | 29714300343 | 2026-07-20 03:16 | success |
| 9 | 29695899970 | 2026-07-19 16:58 | success |
| 10 | 29682329573 | 2026-07-19 09:52 | success |

**Pattern**: 2 cancellations in the last 10 runs (Days 142-143), both during what would be "extra" sessions beyond the standard 3/day. Likely session-budget or pipeline-overlap cancellations from the hourly cron schedule. No API errors or hard failures. All completed runs succeeded.

## yoagent-state DeepSeek Feedback

- **State doctor**: Healthy. 209,165 events, 242 runs, 11,502 `FailureObserved` events, 6,537 `unknown` type events (skipped during projection rebuild). The `unknown` count is growing — new event types get counted but not classified.
- **State why last-failure**: Retroactive `FailureObserved` from run error (Day 144 02:42 session exited with error status but no FailureObserved was recorded at the time — append_terminal_state_events.py patched it retroactively). Source: unknown. This is a harness lifecycle gap, not a code bug.
- **Graph hotspots**: Dominated by tool invocations (bash: 4001, read_file: 3177, search: 1409, todo: 540, edit_file: 487). Expected — these reflect normal agent activity.
- **Cache report**: Empty for agent chat completions. `stream-check` works and shows 66.67% cache hit ratio. The gap is that yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` from DeepSeek chat completion responses. Fix tracked in #105.

## Structured State Snapshot

**Claim health**: State doctor reports projection in-sync with raw store. No integrity issues.

**Top unresolved claim families**:
- `deepseek_model_call_unmatched_completed_count=230` — 230 ModelCallCompleted events without matching ModelCallStarted. Harness-internal evt-harness-* events with zero tokens inflate this count. Issue #134 tracks this but was reverted.
- `evaluator_timeout_with_passing_impl` — new metric added Day 143, no data yet (needs evaluator timeouts to populate).

**Task-state counts** (from trajectory, last 10 sessions):
- reverted_no_edit: 1 (Day 143 session)
- reverted_unlanded_source_edits: 1 (Day 143 session)
- Fully verified: 3 sessions

**Recent tool failures** (from trajectory): `bash_tool_error=5` — shell commands failing. The trajectory recommends bounding commands with explicit paths.

**Recent action evidence**: No recent `src/` Rust changes across last 3 sessions. All changes were to Python scripts (diagnostic tooling).

**Graph-derived next-task pressure** (from trajectory, treated as current harness evidence):
1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files. Action: bound discovery and require a selected task artifact before implementation work starts.
2. **Close yyds state and model lifecycle gaps** (`state_run_unmatched_non_validation_completed_count=1`): Lifecycle causes: state_unmatched/open_after_FailureObserved=1. Issue #134 tracks this.
3. **Raise session success rate** (`session_success_rate=0.0`): The evo session did not complete cleanly even though task success was...
4. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=3`): Recent task sessions ended without landing code.
5. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=5`): Prefer bounded commands with explicit paths.

**Historical tool-failure categories**: Evaluator timeouts (3x), command timeouts after 240s (2x). The evaluator-timeout category was recently addressed via Day 143's evidence detection — the metric now distinguishes timeouts where build/tests passed from those where they failed.

## Upstream Dependency Signals

- **yoagent `Usage` struct drops DeepSeek cache token fields** — `cache_read_input_tokens` and `cache_creation_input_tokens` are parsed from DeepSeek API responses but dropped when constructing the `Usage` struct. This prevents `cache-report` from working for agent chat completions. Fix tracked in #105. No upstream yoagent repo configured — this needs a help-wanted issue or direct PR to yoagent/yologdev.
- No other upstream friction detected. The harness is well-isolated from yoagent internals except for this Usage struct gap.

## Capability Gaps

1. **Cache observability** (#105): `deepseek cache-report` returns no data for agent runs. Without this, we can't measure prompt-cache effectiveness, which is critical for cost optimization and stable-prefix layout tuning. The `stream-check` diagnostic works (66.67% hit rate) but isn't wired into agent runs.
2. **Model lifecycle integrity** (#134): 230 unmatched ModelCallCompleted events inflate the trajectory's lifecycle metric. The harness emits evt-harness-* ModelCallCompleted events with zero tokens that lack matching ModelCallStarted events. This makes the metric untrustworthy — you can't tell real gaps from harness noise.
3. **Script-only sessions**: Last 3 sessions produced zero `src/` Rust changes. All landed changes were Python diagnostic improvements. The trajectory warns this is an analysis-only pattern. The system needs a direct Rust-side improvement to break the script-only streak.
4. **Cancelled sessions**: Two cancellations in 10 runs. Likely from pipeline overlap (hourly cron firing while previous session still running). The session-budget mechanism may need tuning or the cron needs to skip when a session is in-flight.
5. **Unknown event types growing**: 6,537 events classified as "unknown" — new event types added over time that the projection doesn't recognize. Not critical but represents accumulating tech debt.

## Bugs / Friction Found

1. **[MEDIUM] `deepseek cache-report` returns no data for agent runs.** Evidence: `./target/debug/yyds deepseek cache-report` says "no DeepSeek cache metrics recorded from agent chat completions." Root cause: yoagent `Usage` drops DeepSeek-specific token fields. Issue #105 tracks this. Impact: Can't measure prompt-cache ROI or tune stable-prefix layout.

2. **[MEDIUM] 230 unmatched ModelCallCompleted events.** Evidence: Trajectory shows `deepseek_model_call_unmatched_completed_count=230`. Root cause: harness-internal evt-harness-* events with zero tokens. Issue #134 tracks this (reverted). Impact: Lifecycle metric is noise — can't distinguish real agent-model gaps from harness plumbing.

3. **[LOW] 11,502 FailureObserved events accumulated.** State doctor shows this count but 0 "current" failures. These are historical — the retroactive event patcher has been adding them for every run that ended without explicit failure recording. Not a bug per se, but the count inflates over time and makes it hard to find recent genuine failures.

4. **[LOW] 6,537 unknown event types.** Projection rebuild skips them with a log message. Each new event type needs to be taught to the projection. Not blocking but growing.

## Open Issues Summary

| # | Title | Status | Notes |
|---|-------|--------|-------|
| #134 | Model lifecycle gap (harness-internal ModelCallCompleted w/o Started) | Open, reverted | Blocked by agent; no implementation landed. Needs replanning with narrower scope. |
| #105 | DeepSeek prompt cache metrics | Open, reverted, 6 comments | Blocked by agent; no implementation landed. Root cause in yoagent Usage struct. Replanning needed. |

Both reverted issues are from Day 143's cancelled/blocked tasks. No other open agent-self issues.

## Research Findings

No competitor research performed — the trajectory, state evidence, and open issues provide sufficient task material. The two reverted issues (#105, #134) are the most concrete opportunities: both have clear evidence, documented root causes, and Verifier plans already written. The trajectory graph pressure also suggests planning-failure prevention and bash-tool reliability as candidates.
