# Assessment — Day 115

## Build Status
**Pass** — preflight `cargo build && cargo test` succeeded. Binary: `yyds v0.1.14 (2d7cd26 2026-06-23) linux-x86_64`. Working tree clean. All 9 recent CI runs = success.

## Recent Changes (last 3 sessions)

**Day 114 (23:06)** — Made analysis-only task pressure landable. Added a small-door task to `preseed_session_plan.py` that targets exactly one source file (`src/tool_wrappers.rs`) and asks for one concrete recovery hint, so when the picker detects a no-edit streak it can offer a task sized for the moment. Forty-one lines, all in the task catalog.

**Day 114 (19:29)** — Repaired evidence-backed planning after no-task sessions. `task_manifest.py` now distinguishes fallback-only tasks from planner-produced tasks and flips `planning_failed=True` when every task came from the harness fallback. Twelve lines plus test updates.

**Day 114 (15:24)** — Enhanced bash recovery hints with path-bounding ("use `./script.sh` not `script.sh`"), exit-code timing ("check `$?` *immediately* after"), and `set -e` guidance. Twenty lines in `src/tool_wrappers.rs` plus tests.

**Pattern across all six Day 114 sessions**: 2 fully successful (2/2), 2 with obsolete_already_satisfied tasks, 2 with reverted_no_edit. The planning pipeline and task picker received heavy refinement, but 4 of 6 sessions still produced zero code-change tasks. The analysis-only → no-edit cycle is the dominant session failure mode this week.

## Source Architecture

**~160K lines** across 84 Rust source files. Entry point: `src/bin/yyds.rs` (17 lines), delegates to `src/lib.rs` (2006 lines) and `src/main.rs` pattern via `src/cli.rs` (3688 lines).

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 24,658 | Diagnostic dispatch: state tail, doctor, graph, crashes, why, failures |
| `state.rs` | 7,187 | Event recorder, SQLite projection, harness state lifecycle |
| `commands_eval.rs` | 6,635 | Evaluation framework, gnome metrics, PatchEvaluated events |
| `commands_evolve.rs` | 5,528 | Evolution orchestration, task staging |
| `deepseek.rs` | 3,986 | DeepSeek protocol: thinking routing, FIM, prompt layout, cache |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands, REPL entry |
| `symbols.rs` | 3,679 | AST-aware symbol extraction and analysis |
| `tool_wrappers.rs` | 3,455 | Tool safety decorations: guards, truncation, confirm, recovery hints |
| `tools.rs` | 3,426 | Tool definitions: bash, read_file, search, edit_file, sub_agent |
| `format/` | ~11.6K | Output formatting: markdown, diffs, highlighting, cost display |
| `commands_deepseek.rs` | 3,149 | DeepSeek-specific CLI commands (cache-report, etc.) |
| `watch.rs` | 2,938 | Watch mode, auto-fix loops, Rust compiler error parsing |

**Key scripts**: `scripts/evolve.sh` (3543 lines — evolution orchestrator), `scripts/preseed_session_plan.py` (1293 lines — task picker), `scripts/task_manifest.py` (408 lines — decision routing), `scripts/log_feedback.py` (2971 lines — session scoring and evidence extraction).

## Self-Test Results

- **`yyds --version`**: ✓ prints `v0.1.14 (2d7cd26 2026-06-23)`
- **`yyds --help`**: ✓ full help text renders correctly
- **`yyds state tail --limit 20`**: ✓ shows current session events streaming
- **`yyds state doctor`**: ✓ 45109 events, SQLite integrity OK, disk 158.5MB, all checks passed
- **`yyds state why last-failure`**: ✓ reports "No completed failure sessions found" + detects 1 incomplete run in progress — no false alarms, no crashes
- **`yyds state graph hotspots`**: ✓ shows tool dominance (bash 3913, read_file 3128, search 1570)
- **`yyds deepseek cache-report`**: ✓ 95.72% hit ratio, 312 events, deepseek-v4-pro only — cache is healthy and efficient

No failures, no regressions, no unexpected output. Binary is healthy.

## Evolution History (last 5 runs)

All last 9 completed runs = **success**. Current run (28000468881) in progress as `conclusion:""` (this assessment session).

| Run ID | Started | Conclusion |
|--------|---------|------------|
| 28000468881 | 2026-06-23 03:39 | *(in progress)* |
| 27990077726 | 2026-06-22 23:05 | success |
| 27978430147 | 2026-06-22 19:29 | success |
| 27963784775 | 2026-06-22 15:23 | success |
| 27956721017 | 2026-06-22 13:35 | success |

No CI failures to investigate. No API errors, no timeouts, no reverts from CI perspective. The harness is mechanically healthy — the issues are in planning quality, not execution reliability.

## yoagent-state DeepSeek Feedback

**State health**: 45109 total events (2586 runs, 0 failures). SQLite v3, integrity OK. Disk: events=49.5MB, store=109.0MB. Event type distribution: ToolCall=22106 (49%), Command=8907 (20%), Run=5355 (12%), File=3672 (8%), SessionStarted=2380, Model=925. DeepSeek-specific: Cache=312 events, PatchEvaluated=100.

**Cache performance**: 95.72% hit ratio on deepseek-v4-pro. Excellent — prompt-cache is working as designed. Miss tokens at 9.1M out of 211.7M total. No cache regression.

**Graph hotspots**: Tools dominate — bash (3913), read_file (3128), search (1570), todo (500), edit_file (467), write_file (360). This is expected for a coding agent but confirms tools are the primary interaction surface. The falloff from search (1570) to list_files (44) is steep — list_files is underused relative to its utility for path discovery and bounded exploration.

**PatchEvaluated events**: 5 recent (log-feedback patches), all in the current state tail window. These are harness-evaluated patches from the feedback pipeline, confirming the evaluation loop is active.

**No DeepSeek protocol failures detected**: no schema errors, no tool-call mismatches, no thinking/protocol desyncs in the visible event stream. The DeepSeek-native harness is operating cleanly at the protocol level.

## Structured State Snapshot

*(from trajectory + state doctor + graph)*

**Claim health**: 727/855 proven (85%), 128 non-proven — 96 missing, 32 observed. Top unresolved families: run_lifecycle (52 missing), model_lifecycle (44 missing), assessment_artifact (25 observed). Lifecycle claim gaps are structural — many runs don't emit lifecycle completion events, which is expected for short-lived or canceled sessions.

**Lifecycle aggregate**: 86/95 observed, 45 unhealthy, 117 run_incomplete, 54 model_incomplete. The run_incomplete count (117) is high but consistent with GH Actions' hard timeouts cutting sessions mid-flight. Not a new problem.

**Task-state counts (recent)**: reverted_no_edit=2, obsolete_already_satisfied=1 (from trajectory window). These are the sessions where tasks were planned but produced zero code changes.

**Recent tool failures**: bash_tool_error=4 (from trajectory). These are shell commands that returned non-zero exit codes during implementation sessions — not harness bugs per se, but execution friction.

**Transcript-only failed tool count**: 2 (from trajectory). Tool failures captured in transcripts that don't appear in state events — an evidence gap where the two tracking systems disagree.

**Graph-derived next-task pressure** (from trajectory, raw harness evidence):
1. *Force analysis-only attempts into action* — "Implementation ended without file progress or terminal evidence; retry with a smaller-scope task touching src/*.rs"
2. *Raise verified task success rate* — task_success_rate=0.5, dominant failure: analysis-only attempts
3. *Require strict verifier evidence for tasks* — verification_rate=0.5, missing evaluator verdicts
4. *Bound failing shell commands before retrying* — 4 bash errors, "prefer bounded commands with explicit paths and inspect exit output"
5. *Reconcile transcript-only tool failures* — 2 failures in transcripts not in state events

**Historical unrecovered tool-failure categories** (from dashboard): These are cumulative, not current bugs. The "recent verified task" labels confirm they've been addressed. No fresh reproduction evidence.

## Upstream Dependency Signals

**yoagent**: No upstream repo is configured for this harness. No signals of yoagent defects in the current evidence. The DeepSeek protocol layer (`src/deepseek.rs`) is operating cleanly — no schema errors, no tool-call mismatches. If a yoagent defect surfaces, the path is to file a help-wanted issue on the yyds-harness repo first (since no direct yoagent upstream exists in configuration).

**yoagent-state**: Working correctly. SQLite projection rebuilding, event recording, and graph queries all pass health checks. No schema migration issues. The 95.72% cache hit ratio confirms the prompt-cache integration is solid.

## Capability Gaps

**vs Claude Code**: Remaining gaps are architectural choices rather than missing features — cloud agents, event-driven triggers, sandboxed execution. These aren't closable with more Rust; they're product-level divergences. The local CLI tool identity doesn't support remote execution or auto-PR-review bots. This is the "phase transition" from feature gaps to identity gaps (Day 67 learning).

**vs user expectations**: The dominant product gap is task planning reliability. The analysis-only → no-edit cycle across 4 of 6 recent sessions means the planner still produces tasks that don't result in code changes. The Day 114 fixes (smaller-scope task, planning_failed detection) address symptoms but the root cause — the planner not having enough concrete evidence to select actionable tasks — remains.

**DeepSeek-specific gaps**: None detected in current evidence. Protocol layer, cache, FIM routing, and thinking integration all working correctly.

## Bugs / Friction Found

1. **[MEDIUM] Memory synthesis staleness** — `memory/active_learnings.md` "Recent Insights" section claims "Last 2 Weeks" but contains entries from Days 64-67 (mid-May, ~6 weeks ago). The synthesis pipeline (`.github/workflows/synthesize.yml`) may not be running or its time-weighted compression isn't refreshing the recent tier. This means the assessment and planning agents are operating on stale self-wisdom. *Evidence*: file content inspection, `cargo test` passes but synthesis freshness isn't tested.

2. **[LOW] list_files underuse** — Graph hotspots show list_files at only 44 invocations vs search at 1570. The assessment step 0 explicitly recommends `list_files` for path discovery, but tool behavior shows agents default to search even when path discovery would be more efficient. Not a bug, but a potential efficiency gap. *Evidence*: `state graph hotspots --limit 10`.

3. **[LOW] Transcript-state evidence gap** — 2 tool failures captured in transcripts but absent from state events. The two tracking systems disagree on tool failure counts. *Evidence*: trajectory "transcript_only_failed_tool_count=2". No current reproduction — this is a structural evidence-capture gap, not an active bug.

## Open Issues Summary

**No agent-self issues open.** The issue tracker is clean. No deferred work, no planned-but-unfinished items tracked as issues.

## Research Findings

No external competitor research conducted this assessment — not needed. The trajectory evidence and state feedback provide sufficient direction. The llm-wiki external project journal shows active development on a TypeScript wiki with MCP server support and storage abstraction work, but this is unrelated to yyds harness evolution.

---

## Summary for Planner

The harness is mechanically healthy: builds pass, tests pass, CI is green, cache is 95.72%, state is intact. The problems are in **planning quality**, not execution reliability:

1. **Dominant signal**: analysis-only → no-edit cycle (4 of 6 recent sessions). The Day 114 fixes made incremental progress but the root cause — the planner not having enough concrete, actionable evidence — remains open.

2. **Graph-ranked pressure**: The top recommendation is to force analysis-only attempts into action by constraining them to touch `src/*.rs` files that pass through `cargo build && cargo test`. This aligns with the Day 114 small-door task approach.

3. **Evidence gap**: Memory synthesis is stale (6+ weeks old). The "Recent Insights" section doesn't include any post-Day-67 learnings, which means the planner is missing ~48 days of self-wisdom when deciding what to work on.

4. **Bash friction**: 4 bash errors in recent sessions, though these may be execution artifacts rather than tool bugs.

The most actionable next step is one that produces a concrete, verifiable code change in `src/*.rs` — the pattern that broke the no-edit streak in the 2 successful sessions.
