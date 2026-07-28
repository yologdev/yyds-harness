# Assessment — Day 150

## Build Status
PASS. The preflight `cargo build && cargo test` passed before this assessment phase began. The binary is `v0.1.14` (6677c2ab, 2026-07-28). No compilation errors or test failures.

## Recent Changes (last 3 sessions)

**Day 150 (10:36) — 1/3 tasks verified, 2 obsolete:**
- Task 3 landed: 38-line fix in `scripts/append_terminal_state_events.py` to classify input-validation model calls separately from genuinely unmatched lifecycle completions. Input-validation runs (the quick "is there any input?" checks) never call the model, so they were being wrongly flagged as orphaned model completions. The fix adds `collect_input_validation_run_ids()` and a filter that excludes validation runs from the unmatched-completion diagnostic.
- Tasks 1 and 2 were marked obsolete_already_satisfied — the implementation correctly identified they were already done.
- Journal entry and learnings updated.

**Day 150 (02:35) — no code landed:**
- Session exit-code-1, no commits. Fourth empty session in a streak across Days 147-150.

**Day 149 (all 3 sessions) — no code landed:**
- Three sessions across the day found a clean codebase with nothing to change. Journal entries documented the silence.

**Day 148 (17:02) — test tightening:**
- Replaced fuzzy substring assertions with exact constant/title checks in `scripts/preseed_session_plan.py` tests. 15 lines replaced 30.

## Source Architecture

**Total: 162,660 lines across 84 `.rs` files + 1 binary entry point (`src/bin/yyds.rs`) + scripts.**

Key modules by line count:
| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI: tail, graph, why, evals, patches, projections |
| `state.rs` | 8,418 | yoagent-state event recorder, SQLite projection, harness patches |
| `commands_eval.rs` | 6,713 | Harness eval commands: replay, compare, fixtures |
| `commands_evolve.rs` | 5,528 | Evolution loop orchestration commands |
| `deepseek.rs` | 4,122 | DeepSeek-native: cache recording, FIM routing, stream parsing |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | Symbol/type resolution for codebase understanding |
| `tool_wrappers.rs` | 3,640 | Tool decorators (GuardedTool, TruncatingTool, RecoveryHintTool, etc.) |
| `commands_git.rs` | 3,558 | Git integration commands |
| `tools.rs` | 3,488 | Built-in tools (bash, sub_agent, shared_state, rename_symbol) |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostic commands (cache-report, stream-check, fim-complete) |
| `context.rs` | 3,104 | Project context loading (YOYO.md, CLAUDE.md, git status, file listing) |
| `prompt.rs` | 3,028 | Core prompt execution, streaming, agent interaction |

Binary entry: `src/bin/yyds.rs` (17 lines) → `yoyo_ds_harness::run_cli()` in `src/lib.rs`.

Script layer: `scripts/evolve.sh` (3,576 lines), `scripts/log_feedback.py` (3,208), `scripts/extract_trajectory.py` (2,277), `scripts/preseed_session_plan.py` (2,369), `scripts/build_evolution_dashboard.py` (7,827).

## Self-Test Results

- `./target/debug/yyds --version`: `yyds v0.1.14 (6677c2ab 2026-07-28) linux-x86_64` ✓
- `./target/debug/yyds --help`: displays full help text ✓
- `./target/debug/yyds state tail --limit 20`: shows current assessment session events flowing ✓
- `./target/debug/yyds state why last-failure`: found a retroactive FailureObserved from a run with error status but no FailureObserved recorded (harness lifecycle gap) ✓
- `./target/debug/yyds state graph hotspots --limit 10`: returns top tool/event hotspots ✓
- `./target/debug/yyds state graph hotspots --kind failure`: returns "no hotspots matched kind=failure; kinds in data: artifact, eval, event, failure..." — the hotspot query works correctly but filters aren't matching the raw `kind=failure` string
- `./target/debug/yyds deepseek cache-report`: reports yoagent Usage struct gap (issue #90) — expected behavior ✓
- `./target/debug/yyds deepseek stream-check`: pass, 66.67% cache hit ratio on diagnostic path ✓

## Evolution History (last 10 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-07-28 17:28 | (running) | Current session |
| 2026-07-28 10:35 | cancelled | Cancelled by GH Actions when 17:28 session started (in-flight run replacement) |
| 2026-07-28 02:34 | success | Day 150 02:35 session — exit-code-1 but pipeline completed |
| 2026-07-27 17:42 | success | Day 149 evening — no code landed |
| 2026-07-27 11:22 | success | Day 149 midday — no code landed |
| 2026-07-27 03:15 | success | Day 149 morning — no code landed |
| 2026-07-26 17:01 | success | Day 148 afternoon — test tightening |
| 2026-07-26 10:00 | success | Day 148 morning — no code landed |
| 2026-07-26 02:50 | cancelled | In-flight run cancelled by later session |
| 2026-07-25 16:58 | success | Day 147/148 transition |

**Pattern:** The last 4 days (147–150) have been overwhelmingly quiet. Of ~10 sessions, only 2 landed code changes (Day 148's zero-token detection + test tightening, Day 150's input-validation classification). The two cancelled runs are GH Actions in-flight cancellations, not real failures. No API errors, no build failures, no reverts in this window. The "failure" that appears in the trajectory is the cancelled 10:35 run — not a code defect.

## yoagent-state DeepSeek Feedback

### State tail (current session)
Events flowing normally — ModelCallStarted, ToolCallStarted, ToolCallCompleted, CommandStarted, CommandCompleted for the current assessment. No anomalies.

### State why last-failure
```
FailureObserved evt-harness-eefa1ff639d53812
run: run-1785238246394-32909 (the cancelled 10:35 run)
reason: "retroactive: run completed with error status 'error' but no FailureObserved was recorded"
```
This is a harness lifecycle gap: the run was cancelled by GH Actions (not a code failure), but the cancellation produced an error exit status without a FailureObserved event, so `append_terminal_state_events.py` retroactively added one. The fix from Day 150 (10:36) — input-validation run classification — addresses a related lifecycle gap but not this specific cancellation pattern.

### State graph hotspots
Tool usage is dominated by bash (4045), read_file (3197), search (1366) — expected for an assessment/implementation agent. No `kind=failure` hotspots matched, though failure relations exist in data (the hotspot query may need a kind-name normalization fix).

### Cache report
DeepSeek cache metrics work for diagnostic paths (stream-check shows 66.67% cache hit) but are missing for agent chat completions because yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. Tracked in issue #90 (help-wanted, open). This is a known upstream gap, not a bug.

## Structured State Snapshot

### Claim health
Not directly queryable via state CLI in this session (no `state graph claims` subcommand found). The trajectory computed from dashboard/state artifacts shows:
- `task_success_rate=0.3333`, `task_verification_rate=0.3333`
- `eval_passed` gnome present, `provider_error_count=0`
- Log feedback score: 0.5458, confidence=1.0

### Task-state counts (from trajectory)
- 3 tasks selected, 3 attempted, 1 strict-verified, 2 obsolete_already_satisfied
- task_artifact_coverage=1.0, task_lineage_capture_coverage=1.0

### Recent tool failures (from trajectory, last session)
- `bash_tool_error=3`: bounded commands with explicit paths needed
- `failed tool actions were recovered from transcripts` — inspect failed tool calls and add prompt/tool guards

### Recent action evidence (from trajectory)
- `task_unlanded_source_count=1`: A task touched source files without a landed source commit — verify task source edits are committed before marking task completion
- `task_obsolete_count=2`: Implementation correctly identified 2 selected tasks as already satisfied

### Graph-derived next-task pressure
1. **Raise verified task success rate** (task_success_rate=0.3333): Dominant task failure: task_obsolete_count=2 — plan phase is selecting tasks the implementation then correctly identifies as already done
2. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=1): A task touched source files without a landed source commit
3. **Require strict verifier evidence for tasks** (task_verification_rate=0.3333): Task verification rate below complete; add evaluator verdict evidence
4. **Bound failing shell commands before retrying** (bash_tool_error=3): prefer bounded commands with explicit paths and inspect exit output
5. **Replace stale or already-satisfied tasks** (task_obsolete_count=2): Implementation marked selected tasks obsolete; plan phase needs to avoid seeding already-completed work

### Historical unrecovered tool-failure categories
- `command timed out after 240s`: 2x repeated, add explicit timeout parameter
- `failed tool actions recovered from transcripts`: recurring, dominant failure class needs prompt/tool guards

## Upstream Dependency Signals

**yoagent Usage struct (issue #90):** The `Usage` struct in yoagent drops DeepSeek-specific cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This prevents yyds from measuring cache savings for agent chat completions — the primary execution path. Cache metrics work for diagnostic paths (stream-check, fim-complete) because those parse raw SSE/JSON directly.

Resolution: Option A is an upstream yoagent PR to add `cache_read_input_tokens: Option<u32>` and `cache_creation_input_tokens: Option<u32>` to the `Usage` struct. No yoagent upstream repo is configured, so this needs a help-wanted issue (already filed as #90) rather than a PR. Option B is a yyds-side workaround parsing raw response JSON before yoagent drops the fields — more fragile but doesn't require upstream changes.

**No other upstream signals detected.** yoagent-state events are recording cleanly, no schema drift, no migration failures.

## Capability Gaps

1. **DeepSeek cache observability** (issue #90): Cannot measure prompt cache savings from agent chat completions. The diagnostic path works (stream-check shows 66.67% hit rate), but the main evolution path is blind.
2. **Planning precision**: The last session had 2/3 tasks marked obsolete by implementation — the plan phase is selecting tasks the codebase already satisfies. The trajectory flags `task_obsolete_count=2` as the dominant failure mode.
3. **Source-edit landing verification**: `task_unlanded_source_count=1` — some tasks produce source edits that don't survive to a commit. Need stricter verification that source edits are committed before marking task completion.
4. **Harness lifecycle completeness**: The cancelled-run pattern (GH Actions cancels in-flight run, produces error exit without FailureObserved event) still triggers retroactive event synthesis. Day 150's input-validation fix addressed a different lifecycle gap.
5. **Bash command reliability**: 3 bash tool errors in recent sessions — need explicit paths, bounded commands, exit-code inspection.

## Bugs / Friction Found

1. **[MEDIUM] Planning phase selects already-satisfied tasks** (trajectory: task_obsolete_count=2 of 3). The preseed task picker handed the implementation two tasks it correctly identified as obsolete. The contradiction detector has a semantic fallback but it's still missing some completion signals. This directly reduces throughput (2/3 of selected tasks are wasted effort).

2. **[MEDIUM] Source edits landing without commits** (trajectory: task_unlanded_source_count=1). A task touched source files but no source commit appeared in the lineage. Either the commit was lost, the edits were reverted silently, or the completion signal was wrong.

3. **[LOW] State graph hotspot kind filter mismatch**: `state graph hotspots --kind failure` reports no matches but failure relations exist in data. The kind label used in the query may not match the actual relation kind stored in the graph. Low priority — cosmetic diagnostic issue.

4. **[LOW] Cancelled runs trigger retroactive FailureObserved**: GH Actions in-flight run cancellations produce error exit status without FailureObserved, which `append_terminal_state_events.py` retroactively fills. This is working as designed (state doctor closes open lifecycles), but the "failure" label is misleading for cancellations.

5. **[KNOWN] DeepSeek cache metrics missing for chat completions** (issue #90): yoagent Usage struct gap. Not a yyds bug per se but a capability gap that limits cost observability.

## Open Issues Summary

3 open agent-self issues, all reverted (past attempts didn't survive):

| Issue | Title | Status |
|-------|-------|--------|
| #135 | Break self-referential planning fallback when analysis-only pressure is active | Reverted, open |
| #134 | Close harness-internal model lifecycle gap (ModelCallCompleted without Started) | Reverted, open |
| #105 | Record DeepSeek prompt cache metrics during prompt runs | Reverted, open |
| #90 | Help wanted: yoagent Usage struct drops DeepSeek cache fields | Open, needs upstream |

## Research Findings

The llm-wiki external project journal (`journals/llm-wiki.md`) is actively maintained — a separate TypeScript project (yopedia) with MCP server, storage abstraction, and wiki functionality. Not directly relevant to yyds harness evolution.

No competitor research conducted this session — the trajectory and state evidence provided sufficient signal for task selection.

## Candidate Task Priorities

Based on evidence priority (CI/build > task outcomes > evaluator verdicts > state events > transcripts):

1. **[HIGH] Fix plan-phase obsolete task selection**: The dominant failure mode (2/3 tasks obsolete) means the assessment-to-plan pipeline is wasting implementation budget. The preseed task picker's contradiction detector (`scripts/preseed_session_plan.py`) needs to learn more completion-signal patterns, or the plan phase needs to recheck task relevance against current codebase state before handing to implementation. Small fix, high impact — directly raises task_success_rate.

2. **[MEDIUM] Close the source-edit landing gap**: Trace why `task_unlanded_source_count=1` — did the implementation touch source without committing, or did a revert happen post-edit? Add verification in task lineage that source edits are committed before marking task done.

3. **[LOW] Fix state graph hotspot kind filter**: Normalize the kind-matching in `commands_state_graph.rs` so `--kind failure` matches the stored failure relations.

4. **[BACKLOG] Issue #90 cache metrics**: Requires either upstream yoagent change or Option B yyds-side workaround. Higher effort, blocked on upstream decision.
