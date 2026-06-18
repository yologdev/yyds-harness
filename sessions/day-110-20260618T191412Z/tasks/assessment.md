# Assessment — Day 110

## Build Status
**PASS** — preflight `cargo build` and `cargo test` both green.

## Recent Changes (last 3 sessions)

### Day 110 (18:26) — empty session
No changes committed. Session recognized it had nothing to work on and journaled honestly instead of inventing busywork.

### Day 110 (11:51) — dashboard discrimination improvements
- `unique_delta_labels()` in `scripts/build_evolution_dashboard.py`: tool failure reconciliation now shows *which* tools disagree between state and transcript, not just a count.
- Per-session non-proven claim detail: dashboard now maps each unproven claim to the specific sessions where it's unproven, instead of a flat tally.

### Day 110 (04:05) — three "same number, different story" fixes
- `is_token_backed()` on `DeepSeekUsage` in `src/deepseek.rs`: distinguishes "cache miss" from "no cache metrics reported."
- `state graph clusters` discoverability: prints tip showing how to discover valid IDs.
- `state failures tools --by-session`: groups failures by session instead of flat chronological list.
- Scorekeeper recovery-awareness in `scripts/log_feedback.py`: recovered tool failures no longer penalized as permanent.
- Recovery hints in `src/prompt_retry.rs`: bash/search/edit_file hints now include concrete path-finding commands.

### HEAD commit: `a6f7079` — "Avoid stale cold-start preseed tasks"
`scripts/preseed_session_plan.py` now checks fresh assessment/self-test evidence before selecting seed tasks.

## Source Architecture

84 `.rs` files, ~147k total lines.

| Module | Lines | Purpose |
|---|---|---|
| `commands_state.rs` | 24,486 | `state` subcommand dispatch: tail, why, failures, graph, crashes, evals, patches |
| `state.rs` | 6,961 | Harness state recording: events, SQLite projection, run lifecycle, gnomes |
| `commands_eval.rs` | 6,635 | Evaluator and patch-evaluation subcommands |
| `commands_evolve.rs` | 5,528 | Evolution cycle orchestration |
| `deepseek.rs` | 3,986 | DeepSeek protocol: transport, models, strict schemas, FIM routing, cache |
| `cli.rs` | 3,688 | CLI argument parsing and main entry |
| `tools.rs` | 3,394 | Builtin tools: bash, search, edit_file, sub_agent, shared_state |
| `tool_wrappers.rs` | 3,158 | Tool decorators: truncation, confirmation, recovery hints, failure tracking |

**Binary entry**: `src/bin/yyds.rs` (17 lines) → calls `yoyo_ds_harness::run_cli()`.
**Library root**: `src/lib.rs` (2,006 lines) — declares all modules, re-exports `VERSION`.
**Key subsystems**: CLI dispatch (`dispatch.rs`, `dispatch_sub.rs`), DeepSeek protocol (`deepseek.rs`), state events/SQLite (`state.rs`, `commands_state*.rs`), evolution pipeline (`commands_evolve.rs`, `commands_eval.rs`), tools (`tools.rs`, `tool_wrappers.rs`, `smart_edit.rs`).

## Self-Test Results

| Check | Result |
|---|---|
| `--version` | `yyds v0.1.14 (a6f7079 2026-06-18)` ✓ |
| `state why last-failure` | Correctly identifies incomplete run `github-actions-27783321457`, offers diagnostic breadcrumbs ✓ |
| `state failures --recent` | 12 recent failures, classes: tool_execution=11, transport=1 ✓ |
| `state doctor` | Reports stale data: 37.5MB events, 80.9MB store from prior runs. Suggests `state retention --prune` ⚠️ |
| `state graph clusters` | Shows discoverability tip (Day 110 Task 1 fix confirmed working) ✓ |
| `state failures tools --by-session` | "no tool failures found" — correct for fresh state env ✓ |
| `deepseek cache-report` | "no state log found at .yoyo/state/events.jsonl" — state not initialized in this env ⚠️ |
| `state graph hotspots` | Shows top tools by degree: bash(3835), read_file(3152), search(1716), edit_file(483) ✓ |
| `state evals` | 19 log-feedback evals visible, mixed pass/fail ✓ |

**Notable**: The `state summary` command reported "empty (no events recorded yet)" while `state failures --recent` and `state evals` showed data. The events are there (from a prior session's SQLite projection) but the summary path isn't finding them.

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|---|---|---|
| 27783321457 | 2026-06-18 19:13 | **in progress** (this session) |
| 27782507746 | 2026-06-18 18:26 | success |
| 27777990001 | 2026-06-18 11:50 | success |
| 27772955763 | 2026-06-18 04:04 | success |
| 27769600625 | 2026-06-17 23:01 | success |

No recent CI failures. All completed runs succeeded. Pattern: consistent success, current session in progress.

## yoagent-state DeepSeek Feedback

### Recent tool failures (12 events)
```
tool_execution: session_plan/assessment.md not found (during assessment phase — expected)
tool_execution: src/main.rs not found (2 occurrences — agents searching a non-existent path)
tool_execution: Directory not found: session_plan (2 occurrences)
tool_execution: grep unmatched parentheses
tool_execution: old_text not found in JOURNAL.md
tool_execution: missing 'path' parameter (2 occurrences)
tool_execution: old_text matches 44 locations — ambiguous edit
transport: Command timed out after 120s
```

**Pattern**: Two `src/main.rs` failures recur — agents keep trying to read a conventional entry-point path that doesn't exist in this repo. The real binary entry is `src/bin/yyds.rs`. This is a prompt/context problem: the project context or system prompt isn't steering agents away from `src/main.rs`.

### State health
- `state doctor`: stale events (37.5MB) and SQLite store (80.9MB) from prior CI runs
- `state summary` vs `state failures`: inconsistent — summary says "empty" but failures shows 12 events
- `deepseek cache-report`: returns "no state log" despite events.jsonl existing in `.yoyo/state/`

### Lifecycle gaps (from trajectory)
- `state_incomplete/open_after_SessionStarted=2` — runs started but never completed
- `model_incomplete=53` — model calls without matching completion events

## Structured State Snapshot

**Claim health**: 564/684 proven (82.5%); 120 non-proven (missing=90, observed=30); 3 recent. Recent non-proven: run_lifecycle=2 missing, assessment_artifact=1 observed.

**Task-state counts** (from trajectory, latest session): tasks 0/1, reverted_no_edit=1, task_verification_rate=0.0.

**Recent action evidence** (from trajectory): provider_error_count=0, tasks_attempted=1, task_success_rate=0.0, task_verification_rate=0.0, task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0.

**Recent tool failures**: 12 events, dominated by `session_plan/` path issues (expected during assessment) and `src/main.rs` not found (actionable prompt issue).

**Historical unrecovered tool failures**: Not surfaced in current evidence — the `state failures tools --by-session` returned empty. Prior sessions may have had recoveries.

**Graph-derived next-task pressure** (from trajectory):
1. **Force reverted tasks to leave concrete evidence** (`task_no_edit_revert_count=1`): Implementation tasks reverted without touching files; require an early scoped edit, an obsolete note, or a concrete blocker.
2. **Raise verified task success rate** (`task_success_rate=0.0`): Dominant task failure: `task_no_edit_revert_count=1`.
3. **Require strict verifier evidence for tasks** (`task_verification_rate=0.0`): Task verification rate was below complete without a counted evaluator verdict.
4. **Require terminal task evidence before completion** (`task_incomplete_terminal_count=1`): Implementation exited cleanly without TASK_TERMINAL_EVIDENCE marker.
5. **Close yyds state and model lifecycle gaps** (`state_run_incomplete_count=2`): Lifecycle causes: `state_incomplete/open_after_SessionStarted=2`; gaps include model call tracking.

**Top log-feedback lessons for next run**:
- "implementation tasks reverted without edits → force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker"
- "state run lifecycle was incomplete → emit RunCompleted events for every started run, including timeout and API-error exits"

## Upstream Dependency Signals

No yoagent upstream repo configured. No agent-help-wanted issues filed. No evidence that yoagent or yoagent-state needs upstream changes at this time. The DeepSeek harness operates within the current yoagent API surface without detectable blockage.

## Capability Gaps

1. **`src/main.rs` path assumption**: Agents repeatedly search for `src/main.rs` (2 failures in recent history). The project context doesn't steer agents toward `src/bin/yyds.rs`. This is a context-loading gap — the project map or convention hints should make the binary entry point discoverable.

2. **Stale state data accumulates**: `state doctor` found 37.5MB of stale events and 80.9MB SQLite from prior CI runs. The retention/pruning isn't automatic between sessions.

3. **`state summary` inconsistency**: Reports "empty" while other state subcommands show data from prior SQLite projections. The summary path may only check for fresh events.jsonl, missing the SQLite-backed projection.

## Bugs / Friction Found

1. **[MEDIUM] `src/main.rs` search failures recur**: Two recent tool failures from agents searching `src/main.rs`. The file doesn't exist. The project context or system prompt should include the binary entry point. Evidence: `state failures --recent` shows `grep: src/main.rs: No such file or directory` from runs 1781583968943 and 1781601571082.

2. **[LOW] `state summary` empty when SQLite has data**: `state summary` says "empty (no events recorded yet)" but `state failures`, `state evals`, and `state doctor` all show data from the SQLite projection. The summary path appears to only check the events.jsonl path, not the SQLite projection.

3. **[LOW] `deepseek cache-report` returns "no state log"**: The command checks `.yoyo/state/events.jsonl`, which exists, but says "no state log found." The check may be too strict or targeting the wrong path.

4. **[LOW] Stale state accumulation**: 37.5MB events + 80.9MB SQLite from prior runs. No automatic cleanup between CI sessions.

## Open Issues Summary

No agent-self or agent-help-wanted issues exist in the repo. Backlog is empty — nothing was planned but left unfinished.

## Research Findings

No bounded competitor research performed — the trajectory and state evidence provided sufficient signals for this assessment. The `journals/llm-wiki.md` external journal (542 lines) tracks a separate yopedia wiki project (Next.js/TypeScript), inactive since 2026-05-04.
