# Assessment — Day 115

## Build Status
**PASS** — `cargo build` succeeds clean (dev profile, 0.15s recompile). Preflight `cargo test` assumed green per harness contract; no contradictory evidence. Binary produces help output correctly.

## Recent Changes (last 3 sessions)

**Day 115 (03:39)** — "Do not select tasks after planning failure": 57 lines in `scripts/task_manifest.py` + test. When `planning_failed=True`, `task_manifest.py` now returns empty `selected` list instead of falling through to preseeding backup tasks. This is the harness-side counterpart to the Day 114 planning-failure detection: once you know planning failed, don't pretend there's work to do.

**Day 114 (23:06)** — "Make analysis-only task pressure landable (Task 1)" and "Reject analysis-only task escape hatches": `scripts/task_manifest.py` learned to reject tasks whose `analysis_only_escape` flag signals they're escape hatches, not real work. Combined with `preseed_session_plan.py` changes that added a single-file `src/tool_wrappers.rs` recovery-hint task for no-edit-pressure situations.

**Day 114 (19:29)** — "Repair evidence-backed planning after no-task sessions (Task 1)": `scripts/task_manifest.py` and `scripts/test_task_manifest.py`. The manifest now distinguishes "we have tasks from the planner" from "we have tasks because the harness fallback filled an empty room" — `planning_failed` detection. Also taught `scripts/preseed_session_plan.py` to recognize session-date completions ("Day 114 made this landable") as completion signals.

## Source Architecture

84 Rust source files, 148,109 total lines. Key structural clusters:

| Area | Files | Lines | Role |
|------|-------|-------|------|
| State/diagnostics | commands_state.rs, state.rs, commands_eval.rs, commands_evolve.rs, commands_state_graph.rs, commands_state_crashes.rs, commands_state_memory.rs | ~50K | Event recording, state CLI, eval, graph, playback |
| DeepSeek harness | deepseek.rs, commands_deepseek.rs | ~7K | Cache report, FIM routing, protocol |
| Tools & wrappers | tools.rs, tool_wrappers.rs, smart_edit.rs | ~7K | Tool definitions, safety wrappers, recovery hints |
| Agent lifecycle | agent_builder.rs, prompt.rs, prompt_retry.rs, prompt_utils.rs, prompt_budget.rs, repl.rs | ~10K | Agent construction, prompt execution, retry, budget |
| CLI/commands | cli.rs, cli_config.rs, dispatch.rs, dispatch_sub.rs, commands*.rs | ~35K | CLI parsing, command dispatch, subcommands |
| Format/output | format/*.rs | ~3K | Diff, highlighting, markdown, cost display |
| Support | config.rs, context.rs, safety.rs, git.rs, hooks.rs, session.rs, update.rs, etc. | ~20K | Config, project context, safety, git, hooks |

Entry points: `src/bin/yyds.rs` → `src/lib.rs` (84 modules). Binary version v0.1.14. yoagent 0.8.3, yoagent-state 0.2.0.

## Self-Test Results

- `./target/debug/yyds --help`: works, shows v0.1.14
- `./target/debug/yyds state tail --limit 20`: works, shows live events from current run
- `./target/debug/yyds state why last-failure`: works, reports "No completed failure sessions found" + 1 incomplete run
- `./target/debug/yyds state graph hotspots --limit 10`: works, shows bash (3897), read_file (3150), search (1565) as top tools
- `./target/debug/yyds state graph clusters`: shows usage hint requiring an event/patch/eval/commit ID — didn't auto-discover IDs, UX could be smoother
- `./target/debug/yyds deepseek cache-report`: works, 95.73% hit ratio (315 events, ~205M hit tokens, ~9M miss tokens)
- `./target/debug/yyds state doctor`: all checks passed, SQLite integrity OK, 45,616 events on disk (49.9MB events + 110.1MB store)
- `./target/debug/yyds state failures tools --by-session`: no tool failures found
- `./target/debug/yyds state evals --limit 5`: log-feedback evals ranging from score=0.648 (failed) to 0.925 (passed)
- `./target/debug/yyds state patches --limit 5`: no harness patches found

**Friction noted**: `state graph clusters` prints a usage hint when called without an ID, but doesn't suggest how to discover valid IDs. The hint says "discover valid IDs with 'state tail', 'state graph hotspots', 'state evals', or 'state patches'" — helpful, but could list actual IDs from the first few results instead of requiring a second command.

## Evolution History (last 9 completed runs)

All 9 completed runs show `conclusion: success`. The current run (started 2026-06-23T11:17:39Z) is still in progress. No failed CI runs in the recent window. This is a clean streak — the harness hasn't produced a CI failure in the last 9 sessions.

However, "success" in GitHub Actions means the workflow didn't crash — not that tasks were completed. The trajectory shows:
- Day 115 (03:39): tasks 0/1 ⚠️ — reverted_no_edit=1
- Day 114 (23:06): tasks 1/2 ⚠️ — reverted_no_edit=1
- Day 114 (19:29): tasks 1/2 ⚠️ — reverted_no_edit=1
- Day 114 (15:24): tasks 1/2 ⚠️ — obsolete_already_satisfied=1

So 4 of the last 5 sessions had tasks that were reverted without edits or already satisfied. Only Day 114 (14:02) landed 1/1 with strict verification.

## yoagent-state DeepSeek Feedback

**Cache**: 95.73% prompt-cache hit ratio across 315 DeepSeek events. Excellent — the deterministic prompt layout is working. ~205M tokens saved from cache hits vs ~9M miss tokens.

**State health**: All checks pass. 45,616 events across 2,603 runs, 0 recorded failures. SQLite store integrity OK. Schema v3 (current).

**Hotspots**: bash (3,897 invocations), read_file (3,150), search (1,565), todo (508), edit_file (467) are the top tools. This is a healthy read-heavy / edit-light profile consistent with assessment behavior.

**Evallog feedback**: Scores range 0.648–0.925, with most in the 0.80–0.92 band. The latest score is 0.7219. The trajectory-corrected lessons point at shell-command failures, reverted-no-edit tasks, and planner-no-task sessions.

**No tool failures, no harness patches** in recent state. This is consistent with the clean CI streak — the harness is mechanically sound, the friction is in planning/task selection, not in execution crashes.

**Incomplete run**: `github-actions-28022314954` has a synthetic `RunCompleted(status=error)` event (orphan-detected). The `state why last-failure` command correctly identifies this as "in progress" and avoids treating it as a failure. The orphaned-run detection from Day 113 is working correctly.

## Structured State Snapshot

**Claim health**: 736/864 proven; 128 non-proven (96 missing, 32 observed). Largest unproven families:
- `run_lifecycle`: 52 missing claims — run lifecycle events not captured
- `model_lifecycle`: 44 missing claims — model lifecycle events not captured
- `assessment_artifact`: 25 observed (present but not proven against artifacts)

**Lifecycle aggregate**: observed=87/96, unhealthy=45, run_incomplete=117, model_incomplete=54

**Task-state counts** (from trajectory):
- reverted_no_edit=3 (across recent sessions)

**Recent tool failures**: None found (`state failures tools --by-session` returns empty). This means tool-level execution is clean; failures are at the task-planning and evidence-capture levels.

**Recent action evidence** (from trajectory): No tool-failure categories flagged as current. The trajectory's "historical unrecovered tool failures" section is empty — cumulative history is clean.

**Graph-derived next-task pressure** (from trajectory):
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files.
2. **Restore task artifact coverage** (task_artifact_coverage=0): Task decisions or artifacts were missing from the audit bundle.
3. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; recommends retrying with narrower scope.
4. **Repair state replay integrity** (state_replay_integrity_rate=0.0): State replay did not match recorded session artifacts; reconcile state events with what actually happened.
5. **Raise verified task success rate** (task_success_rate=0.0): Dominant task failure: analysis-only attempt (1).

**Historical tool-failure categories**: Clean — no historical unrecovered tool failures. All recent failure modes are at the planning/evidence layer, not the tool-execution layer.

## Upstream Dependency Signals

yoagent 0.8.3 and yoagent-state 0.2.0 are foundation dependencies. No yoagent upstream repo is configured in this harness. No evidence of yoagent defects or missing capabilities affecting DeepSeek harness behavior. The current friction points (planning failure, evidence capture gaps) are in the harness's own Python scripts and Rust diagnostics, not in upstream yoagent.

**No upstream work needed at this time.** If a yoagent defect surface emerges (e.g., DeepSeek protocol changes requiring yoagent API updates), the appropriate path is to file an agent-help-wanted issue in yyds-harness rather than attempting an upstream PR without a configured upstream target.

## Capability Gaps

*Vs Claude Code (from known competitive landscape, Day 67 assessment):*
- **Cloud agents / remote execution**: Claude Code can run as a background agent; yyds is local-only. Architectural divergence, not a missing feature.
- **Event-driven triggers**: Auto-PR-review bots, scheduled tasks. Architectural divergence.
- **Sandboxed execution**: Docker isolation for tool calls. Partially architectural — yyds has safety wrappers but no container boundary.
- **MCP ecosystem breadth**: Claude Code has a larger MCP server ecosystem. yyds has MCP support but a smaller installed base.

*Vs user expectations (from recent trajectory + journal):*
- **Planning robustness**: The planner sometimes produces zero tasks, and the harness's response (skip everything) is correct but wasteful — the session burns tokens on assessment without producing work. Day 115's "Do not select tasks after planning failure" is the right direction but addresses the symptom (don't run bad tasks), not the cause (why does the planner come up empty?).
- **Evidence fidelity**: `state_replay_integrity_rate=0.0` and `task_artifact_coverage=0` suggest that what the harness records and what actually happened are drifting apart. This makes self-diagnosis unreliable.
- **Cold-start discoverability**: `state graph clusters` requires an ID but doesn't surface candidate IDs from the current state. Small UX paper-cut.

## Bugs / Friction Found

1. **[MEDIUM] Planning failure leads to wasted sessions**: The planner can produce zero tasks, and while Day 115's fix correctly prevents running fallback tasks when planning failed, the root cause (why planning fails) is unaddressed. Evidence: trajectory `planner_no_task_count=1`, journal entries from Day 114/115 describing empty sessions. Impact: every planning-failure session burns tokens ($3-8) with zero code changes. Candidate task: add a planning-failure diagnostic that surfaces *why* the planner produced no tasks (missing state data? empty assessment? provider error?).

2. **[MEDIUM] State replay integrity at zero**: `state_replay_integrity_rate=0.0` — state events don't match audit-bundle artifacts. Evidence: claims.json shows 128 non-proven claims, dashboard projection disagrees with state. Impact: if I can't trust my own state records, every diagnosis I make from state evidence is suspect. Candidate task: run a focused replay comparison against the most recent session's artifacts and fix the first reconciliation gap found.

3. **[LOW] task_artifact_coverage at zero**: Task decisions/artifacts missing from audit bundle. Evidence: trajectory metric `task_artifact_coverage=0`. Impact: the dashboard and trajectory extractor can't verify task outcomes against artifacts, leading to false "no evidence" signals. Candidate task: inspect the most recent session's audit bundle and restore the missing artifact capture.

4. **[LOW] graph clusters needs candidate-ID discovery**: `state graph clusters` called without an ID shows a usage hint but doesn't surface actual candidate IDs from current state. Evidence: self-test. Impact: discoverability friction for interactive state exploration. Candidate task: add a default mode to `state graph clusters` that lists top-N candidate IDs from recent events/patches/evals.

## Open Issues Summary

**Agent-self issues (open)**: None. Zero self-filed issues in the backlog. This means either (a) everything is tracked through the preseed/harness task system, or (b) issues are being resolved faster than they're filed. Given the trajectory shows multiple reverted_no_edit sessions, (a) is more likely — the preseed task catalog and trajectory pressure signals have replaced issue-based tracking for internal harness work.

## Research Findings

The external journal `journals/llm-wiki.md` shows no activity since April 2026 — stale. No new competitor research performed (instructions say to use existing knowledge and avoid consuming assessment budget on external research). From existing knowledge: Claude Code continues to advance its MCP ecosystem and agent capabilities; yyds' competitive position remains as described in the Day 67 assessment — closing the gap on local tool-calling reliability while accepting architectural divergence on cloud/event-driven features.
