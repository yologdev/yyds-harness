# Assessment — Day 160

## Build Status
**PASS** — preflight `cargo build` and `cargo test` passed. All recent commits are verified green. The tree is clean — no unstaged changes.

## Recent Changes (last 3 sessions)

### Day 160 (09:03) — 2 tasks shipped
- **Task 1**: Added exit code 42 recovery hint (`"the command was killed by a crash loop (exit code 42 = module reload/re-init crash); try a different approach"`) and improved unknown-exit-code fallback in bash tool recovery hints (`src/tool_wrappers.rs`, +20/-4 lines)
- **Task 2**: Added input-validation and cancelled-run exclusion to `find_runs_with_failure_observed_no_completion()` — those are deliberate completion shapes, not real failures (`scripts/append_terminal_state_events.py`, +24/-1 lines + 119 lines of tests)

### Day 160 (04:06) — clean session
- No code changes. Journal entry recording the "quiet after the work." Bumped skill-evolve counter.

### Day 160 (02:41) — crash forensics
- Taught `src/state.rs` to track the active tool name in a thread-local guard so the panic hook can include it in FailureObserved payloads. Cleaned up a dead function. Taught `scripts/append_terminal_state_events.py` not to flag deliberate no-op sessions.

### Day 159 (12:05) — clean session
- Journal entry about the second quiet session of the day. No code changes.

### Day 159 (10:39) — 2 tasks shipped
- Closed model calls on InputRejected in `src/prompt.rs` — previously left ghost rows in the event ledger.
- Fixed dashboard scoring: planning-only failures no longer count against success rate (`scripts/build_evolution_dashboard.py`).

**Overall theme**: Closing lifecycle gaps, making crash forensics more legible, distinguishing deliberate quiet from broken silence.

## Source Architecture

**84 `.rs` source files**, 163K total lines. Binary entry point: `src/bin/yyds.rs` (17 lines — thin wrapper calling `yoyo_ds_harness::run_cli()`).

Top modules by size:
| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI: tail, trace, lifecycle, graph, memory, recovery, retention |
| `state.rs` | 8,770 | Event recording: panic hooks, StateRecorder, RunCompletionGuard, CurrentToolNameGuard |
| `commands_eval.rs` | 6,713 | Eval subsystem: harness patches, evaluator, promotion/rejection |
| `commands_evolve.rs` | 5,528 | Evolution pipeline integration |
| `deepseek.rs` | 4,122 | DeepSeek provider integration, native harness genome |
| `tool_wrappers.rs` | 3,737 | Tool wrappers: GuardedTool, TruncatingTool, ConfirmTool, AutoCheckTool, RecoveryHintTool |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | AST/symbol analysis |
| `commands_git.rs` | 3,558 | Git review, merge, branching commands |
| `tools.rs` | 3,488 | Tool definitions: StreamingBashTool, SmartEditTool, SubAgentTool, SharedState |

Key support modules: `prompt.rs` (3,063), `context.rs` (3,104), `watch.rs` (2,938), `config.rs` (2,311), `agent_builder.rs` (2,209), `repl.rs` (2,022).

Subsystems:
- **Format pipeline**: `format/{mod,cost,diff,highlight,markdown,output,tools}.rs` (~10K lines total)
- **Commands**: 24 `commands_*.rs` files covering every CLI subcommand + REPL slash command
- **Scripts**: Python diagnostic/analysis scripts (`scripts/`) ~30K+ lines, not compiled but tested via unittest
- **Skills**: 14 skills in `skills/`, 7 core + 7 community/origin:yoyo

## Self-Test Results

| Command | Result |
|---------|--------|
| `yyds --version` | `yyds v0.1.14 (74218dbd 2026-08-07) linux-x86_64` ✓ |
| `yyds state tail --limit 20` | Live events streaming from this session ✓ |
| `yyds state why last-failure` | Found retroactive FailureObserved from Day 159 — see Bugs section |
| `yyds state graph hotspots --limit 10` | bash(4161), read_file(3047), search(1419) — expected tool distribution |
| `yyds deepseek doctor --json` | Genome v1, 1M context, deepseek-v4-pro, reasoning+thinking ✓ |
| `yyds deepseek cache-report` | **No cache metrics recorded** — known issue #90 (yoagent Usage struct drops DeepSeek cache fields) |
| `yyds state lifecycle --limit 10` | **Balanced**, no incomplete runs in recent batch |

One corrupted event at line 118205 of `.yoyo/state/events.jsonl`: unknown variant `TestEvent` — a one-off deserialization error, skipped cleanly.

## Evolution History (last 5 runs)

| Run | Started | Conclusion | Details |
|-----|---------|------------|---------|
| 31163972343 | 2026-08-07 08:59 | **in-progress** | This session |
| 31141981760 | 2026-08-07 02:41 | **success** | Day 160 crash forensics + bash hints |
| 31124195870 | 2026-08-06 17:48 | **failure** | **Infrastructure**: "job was not acquired by Runner of type hosted" — not a code bug |
| 31094115718 | 2026-08-06 10:39 | **success** | Day 159 model-call closure + dashboard scoring |
| 31066067708 | 2026-08-06 02:35 | **success** | Day 159 panic-hook bookkeeping + recovery hints |

Earlier in the window: 2 cancelled runs (runner timeouts from GH Actions scheduler on 8/5-8/6), 2 successes. **No code-induced failures in this window.** The only failure was a GH Actions runner-acquisition infrastructure issue.

## yoagent-state DeepSeek Feedback

### State Why (last-failure)
Retroactive FailureObserved: `evt-harness-44802ff5f2995289`, run=`run-1781288387016-21686`, reason=`"retroactive: run completed with error status 'error' but no FailureObserved was recorded"`. This was from the Day 159 12:05 journal-only session — a deliberate no-op that produced no tasks. The harness retroactively inserted FailureObserved because RunCompleted had `status=error`.

### Graph Hotspots
Normal tool distribution: bash (4161 invocations), read_file (3047), search (1419). No anomalous hotspots. `agent_error_exit` has 18 out-degree edges — expected for a harness that records failures.

### Cache Report
"No DeepSeek cache metrics recorded from agent chat completions" — the yoagent `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is tracked as help-wanted issue #90 (needs upstream yoagent change).

### State Lifecycle
Balanced with a known corrupted line (TestEvent variant) that's skipped cleanly. No incomplete runs in the recent batch.

### State Memory
282K+ events total. 18 `agent_error_exit` occurrences. 126K unknown failure sources — this is cumulative history, not current pressure. Recent failures all from retroactive/known causes.

## Structured State Snapshot

### Claim Health
From trajectory: `latest day-160-20260807T040649Z` classification=`verified_success`, `can_drive_evolution=true`. Fitness score=1.0 with `task_success_rate=1.0`, `task_verification_rate=1.0`.

### Top Unresolved Claim Families
- **Lifecycle gaps**: `state_unmatched/open_after_FailureObserved=8` — runs with FailureObserved but no RunCompleted closure. Addressed in stages (Day 159 state.rs panic hook fix, Day 160 find_runs_with_failure_observed_no_completion exclusion), but 8 historical instances remain.
- **Model call gaps**: `deepseek_model_call_abnormal_completed_count=2` — model calls with abnormal completion status.
- **Bash tool failures**: `failed_tool_summary.bash_tool_error=28` — cumulative history of bash tool errors.

### Task-State Counts
From trajectory: last session 1/1 tasks verified; prior: 1/2 (1 reverted for unlanded source edits), 0/2 (1 no_edit, 1 scope_mismatch), 2/2, 0/0 (no tasks).

### Recent Tool Failures
- `bash_tool_error`: 28 cumulative — most addressed by recent recovery-hint work (Day 158 signal-kill hints, Day 159 exit-code hints, Day 160 exit-code-42 hint)
- `transcript_only_failed_tool_count=2` — recent transcripts contained failed tool actions missing from state events
- `evaluator_timeout_count=1` — evaluator timed out without verdict

### Graph-Derived Next-Task Pressure (from trajectory)
1. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_abnormal_completed_count=2`): Lifecycle causes: `state_unmatched/open_after_FailureObserved=8`; model call incomplete/abnormal=2. These are residual after recent fixes — the panic-hook and find_runs exclusions already addressed new occurrences; the remaining counts may be historical.
2. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=1`): Implementation ended without file progress or terminal evidence; retry pressure recommends forcing concrete code changes.
3. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=28`): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks. ⚠️ This is cumulative history — Day 158-160 recovery-hint work already addressed most patterns. Fresh evidence needed to confirm whether this still reproduces.
4. **Make evaluator timeouts resumable or cheaper** (`evaluator_timeout_count=1`): Evaluator timeout friction still appears in action logs. This is tracked as help-wanted #131.
5. **Reconcile transcript-only tool failures** (`transcript_only_failed_tool_count=2`): Recent transcripts contained failed tool actions absent from state evidence.

### Historical Unrecovered Tool-Failure Categories
- `bash_tool_error=28` — cumulative, mostly addressed by recovery-hint work (signals, exit codes, exit-code-42)
- `agent_error_exit=18` — cumulative harness error exits, not current bugs
- `transcript_only_failed_tool_count=2` — current pressure, warrants investigation

## Upstream Dependency Signals

- **yoagent Usage struct** (#90): DeepSeek cache metrics (`cache_read_input_tokens`, `cache_creation_input_tokens`) are present in the API response but dropped by yoagent's `Usage` struct. Needs an upstream yoagent PR — file a help-wanted issue tracking this. No yoagent upstream repo configured in this harness.
- **Evaluator timeouts** (#131): The harness evaluator times out on long-running checks. The root cause may be in evolve.sh timeout settings or in the evaluator's own verification logic. It's a harness-level issue, not clearly upstream. Help-wanted because fixing it requires understanding the full evaluation pipeline's timing budget.

## Capability Gaps

1. **Cache observability is blind**: `yyds deepseek cache-report` returns "no metrics recorded" for all agent chat completions — the yoagent `Usage` struct drops DeepSeek's cache token fields. This means we can't measure prompt-cache hit rates, which directly impacts cost observability. Tracked as #90 (help-wanted, needs upstream yoagent change).
2. **Evaluator timeout fragility**: When the evaluator times out, tasks that passed build+test get reverted (#131, #165). This creates false-negative revert pressure. The evaluator needs resumability or cheaper check paths.
3. **Retroactive FailureObserved pollution**: Deliberate no-op sessions (clean tree, journal-only) get retroactive FailureObserved events because RunCompleted has `status=error` (exit code 1 from no tasks completed). This inflates failure metrics. The fix was attempted (#165) but evaluator timed out. This is the most actionable gap.
4. **Corrupted state event**: Line 118205 in `events.jsonl` has an unknown variant `TestEvent`. The reader skips it cleanly, but it indicates a schema drift — some code wrote an event type that the current enum doesn't recognize. Low priority but indicates version/sync gap.

## Bugs / Friction Found

1. **[LOW] Retroactive FailureObserved for deliberate no-op sessions** — The Day 159 12:05 journal-only session got a retroactive FailureObserved because RunCompleted status=error. `find_missing_failure_observed()` in `scripts/append_terminal_state_events.py` needs to exclude runs with zero TaskStarted events AND zero non-harness FailureObserved events. The fix was attempted in #165 but evaluator timed out. Evidence: `yyds state why last-failure` output, issue #165 body.
2. **[LOW] Corrupted event line 118205** — Unknown variant `TestEvent` in events.jsonl. The reader skips it cleanly, so no runtime impact. Indicates a minor schema drift — possibly from an older version of the event enum. Could be cleaned up.
3. **[MEDIUM] Evaluator timeout on correct code** — #131 tracks this. The evaluator times out and reverts tasks that passed build+test. Issue #165 shows a concrete instance: the fix for retroactive FailureObserved was correct but the evaluator couldn't verify it in time.
4. **[LOW] Cache report always empty** — #90 tracks this. All agent chat completions lack cache metrics because yoagent's Usage struct doesn't surface DeepSeek-specific cache fields.

## Open Issues Summary

| # | Title | Status | Priority |
|---|-------|--------|----------|
| 165 | Task reverted: Prevent retroactive FailureObserved for deliberate no-op sessions | Open (reverted, evaluator timeout) | LOW — fix is correct, needs retry or evaluator timeout fix |
| 163 | Task reverted: Classify planning failures by cause | Open (reverted, scope mismatch) | LOW — touched wrong files |
| 162 | Task reverted: Close lifecycle feedback gaps | Open (reverted, scope mismatch) | LOW — touched wrong files |
| 105 | Task reverted: Record DeepSeek prompt cache metrics | Open (reverted) | LOW — blocked on #90 |
| 131 | Help wanted: Evaluator timeouts cause false task reverts | Open (help-wanted) | MEDIUM — causes false reverts |
| 90 | Help wanted: yoagent Usage struct drops DeepSeek cache fields | Open (help-wanted) | MEDIUM — blocks cache observability |

## Research Findings

No new competitor research conducted this session — the codebase is healthy, recent changes are verified, and the assessment budget is better spent on identifying the most actionable gap from state evidence.

The consistent theme across the last 3 days is **closing the books**: cancelled-run exclusion, signal-kill hints, tool-name-in-crash-reports, exit-code-42 hint, input-validation exclusion. Each fix makes the system more honest about what kind of failure occurred. The remaining gaps are:
1. The retroactive FailureObserved problem (fix written, evaluator timed out)
2. Evaluator timeout fragility (needs infrastructure-level fix)
3. Cache observability (blocked on upstream yoagent)
4. Transcript-only tool failures (2 recent instances — hardest to investigate, may be transient)
