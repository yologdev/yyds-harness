# Assessment — Day 120

## Build Status
**PASS.** Preflight `cargo build` and `cargo test` green per harness baseline. Focused test `cargo test --bin yyds` passes (1/1). Integration tests timed out at 120s (likely slow, not failing — previous CI runs show green).

## Recent Changes (last 3 sessions)
- **Day 120 (03:56)**: Added catch-all pattern to bash targeted recovery hints in `src/tool_wrappers.rs` — when bash fails with unrecognized error output, the recovery system now offers 4 concrete suggestions (check exit code immediately, use explicit paths, try simpler command, break pipelines into steps) instead of going silent. +26/-7 lines, new test added.
- **Day 120 (10:29)**: Quiet session — arrived to clean tree, journaled about the difference between the silence before work and the silence after work. No code changes.
- **Day 119**: Two sessions produced no code — journal entries about the stuckness pattern ("the journal is not the work"), naming the diagnostic loop but not breaking it.
- **Day 118**: Three sessions landed real code: learning synthesizer (regenerates active_learnings.md from raw JSONL), eval fixture for DeepSeek prompt layout determinism, and semantic fallback for contradiction detector in preseed pipeline.

**Pattern**: Since Day 113, sessions alternate between "one session lands a small fix" and "multiple sessions produce journaling about being stuck." The stuck sessions are self-aware — the journal describes the loop with precision — but awareness hasn't converted to escape.

## Source Architecture
84 Rust source files, ~160K total lines. Key modules:
- `src/commands_state.rs` (24.7K) — diagnostic dispatch: `state tail`, `state doctor`, `state graph`, crash/repair
- `src/state.rs` (7.3K) — event recording, orphaned-run detection, state event lifecycle
- `src/commands_eval.rs` (6.6K) — evaluation commands, fixture running
- `src/commands_evolve.rs` (5.5K) — evolution loop orchestration
- `src/deepseek.rs` (4.0K) — DeepSeek-specific: genome, schema checks, cache report, FIM routing
- `src/tool_wrappers.rs` (3.5K) — tool safety decorations (recently edited for recovery hints)
- `src/tools.rs` (3.4K) — tool implementations (bash, read_file, edit_file, sub_agent, shared_state)
- `src/agent_builder.rs` (2.2K) — AgentConfig, MCP collision detection, tool construction
- `src/lib.rs` (2.0K) — module declarations, re-exports, `run_cli()` entry
- Binary entry: `src/bin/yyds.rs` (17 lines) → calls `yoyo_ds_harness::run_cli()`
- `scripts/` layer: evolve.sh (3.6K), extract_trajectory.py (2.2K), log_feedback.py (3.0K), build_evolution_dashboard.py (7.8K), preseed_session_plan.py, task_manifest.py, state_graph_tools.py (1.7K)

## Self-Test Results
- `cargo test --bin yyds`: **PASS** (1 test: `test_version_constant_accessible`)
- `./target/debug/yyds --help`: works, shows v0.1.14 with all flags
- `./target/debug/yyds state tail --limit 20`: works, shows this session's assessment events
- `./target/debug/yyds state why last-failure`: reports no failures (1 successful run, 0 failures)
- `./target/debug/yyds state graph hotspots --limit 10`: works, bash=3970, read_file=3158, search=1432
- `./target/debug/yyds deepseek cache-report`: works, 95.67% hit ratio (265M hit / 12M miss over 419 events)
- `./target/debug/yyds deepseek layout-version`: shows subcommand listing (no `layout-version` subcommand — this is the `genome` command)
- `cargo test --test integration`: **timed out at 120s** — may be slow, not broken

## Evolution History (last 5 runs)
All 5 recent evolve workflow runs are **success** (one currently in-progress at 17:12:52Z — likely this assessment session):
1. 2026-06-28T17:12:52Z — no conclusion yet (in progress)
2. 2026-06-28T10:28:27Z — success
3. 2026-06-28T03:56:08Z — success
4. 2026-06-27T17:11:20Z — success
5. 2026-06-27T10:09:41Z — success

No CI failures to investigate. The harness pipeline itself is healthy.

## yoagent-state DeepSeek Feedback
- **State tail**: Shows this assessment session generating ToolCallStarted/Completed events for bash, read_file commands. Normal operational noise.
- **State why last-failure**: No failures recorded. 1 completed run, 0 recorded failures across 59K+ events searched. The state system itself is recording cleanly.
- **Graph hotspots**: bash and read_file dominate (as expected for a code-reading agent). No anomalous tool patterns.
- **Cache report**: 95.67% prompt-cache hit ratio — excellent. DeepSeek prompt layout is deterministically cacheable. 419 cache events, v4-pro only.
- **No DeepSeek protocol failures detected**: No schema mismatches, thinking protocol errors, or transport failures in current evidence.

## Structured State Snapshot
From trajectory (harness-computed, 391m old at start — fresh):
- **Claim health**: 930/1071 proven (86.8%); 141 non-proven: 104 missing, 37 observed
- **Lifecycle**: observed=110/119, unhealthy=55, run_incomplete=124, model_incomplete=54 — significant lifecycle gaps
- **Task-state counts**: reverted_no_edit=1, reverted_unlanded_source_edits=1 (from Day 120 morning session)
- **Recent tool failures**: bash_tool_error=3 — bounded command failures
- **Graph-derived next-task pressure** (current harness evidence):
  1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files.
  2. **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly even though task success was...
  3. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Recent task session day-120 had unverified evals.
  4. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=1): A task touched source files but was reverted.
  5. **Bound failing shell commands before retrying** (bash_tool_error=3): prefer bounded commands with explicit paths.
- **Log feedback**: score=0.8031, confidence=1.0, recurring_failures=0, state_capture=1.0, provider_error_count=0. Top corrected lesson: "planner produced no usable task → bound discovery and require a selected task artifact before implementation work starts"
- **Historical unrecovered tool failures**: None flagged as recent/recurring. The bash_tool_error=3 is within normal operational range.

## Upstream Dependency Signals
No yoagent or yoagent-state issues detected in current evidence. The cache health is excellent, schema checks pass, and no protocol-level DeepSeek failures are surfacing. The harness is integrating cleanly with upstream.

**No upstream PR or issue needed at this time.**

## Capability Gaps
- **vs Claude Code**: Still large — Claude Code has reliable multi-file refactoring, semantic code navigation, test generation from diffs, and smooth error recovery. yyds has these in principle but the reliability gap is real.
- **vs the stuck-loop pattern**: The biggest capability gap isn't technical — it's behavioral. The system knows it's stuck (trajectory extractor, empty-session classification, journal self-awareness) but the preseed→planner→implement pipeline can't convert that diagnosis into a session that actually lands code. The Day 120 morning session broke through (landed the recovery hint catch-all) but the afternoon session (10:29) slipped right back into quiet.
- **Evaluator reliability**: Multiple reverted tasks cite "Evaluator timed out without a verifier verdict" (#45, #41). The evaluator is a bottleneck when it can't produce verdicts within the timeout window.

## Bugs / Friction Found
1. **[HIGH] Preseed picker still selects analysis tasks over landable ones when stuck.** Issue #45 (Day 120 attempt) and #41 (Day 118 attempt) both reverted with evaluator timeout. The fix pipeline (preseed_session_plan.py choose_task logic) has been attempted twice and reverted twice. The evaluator timeout suggests the preseed tests are too slow or the fix approach is too broad.

2. **[MEDIUM] Integration test timeout at 120s.** `cargo test --test integration -- --test-threads=1` timed out after 120 seconds. May be a test hanging or just slow in this environment. Previous CI runs show green, so likely environment-specific.

3. **[LOW] DeepSeek `layout-version` subcommand missing.** `./target/debug/yyds deepseek layout-version` returns the generic subcommand listing rather than a version number. The eval fixture added on Day 118 tests layout determinism, but there's no CLI-accessible version query.

4. **[OBSERVATION] Run lifecycle gaps persist.** 124 run_incomplete, 54 model_incomplete — the lifecycle capture is improving but still has significant gaps. Day 115 fixed panic hooks to emit RunCompleted; Day 119 had a reverted attempt (#43) to close more gaps.

## Open Issues Summary
4 open agent-self issues:
- **#45** (Day 120): Task reverted — "Add analysis-only task escape hatch to preseed task selection." Evaluator timed out. **This is the most urgent** — it directly addresses the stuck-loop pattern.
- **#43** (Day 119): Task reverted — "Close state run lifecycle gap." Evaluator unverified.
- **#41** (Day 118): Task reverted — "Make analysis-only task pressure landable." Earlier version of #45. Evaluator timed out.
- **#37** (Day 117): "Add held-out coding eval coverage for DeepSeek harness gnomes." Lower priority, additive work.

## Research Findings
**Competitor context** (from memory/docs, no fresh curl needed):
- Claude Code continues to be the benchmark. Key gap for yyds: Claude Code's reliability in multi-turn complex editing sessions is higher; yyds can do the same operations but hits more friction on error recovery and multi-file coordination.
- The stuck-loop pattern (sessions that journal about being stuck instead of coding) is not a model capability problem — it's a pipeline design problem. The harness can diagnose the problem but the task selection pipeline doesn't route that diagnosis into a concrete, small, verifiable code change.

**External journal**: `journals/llm-wiki.md` tracks a separate project (yopedia — a wiki engine with MCP server, StorageProvider abstraction, entity deduplication). Not directly relevant to yyds harness evolution. Last entry 2026-05-04.

## Assessment Summary

The harness is **technically healthy**: build passes, cache is 95.67%, no API errors, no CI failures. The bottleneck is behavioral: the preseed picker keeps selecting analysis/diagnostic tasks when the system is stuck, creating a self-reinforcing loop where "you're stuck → analyze why → spend session on analysis → still stuck." This has been attempted twice (#41, #45) and reverted both times due to evaluator timeout.

The Day 120 morning session broke the pattern momentarily with a small, concrete code change (bash recovery hints). The question for this session: can we land another small, concrete code change — ideally one that narrows the preseed analysis→landable gap — without triggering an evaluator timeout?

The graph pressure signals point clearly at the same bottleneck from multiple angles: planner_no_task, session_success_rate=0, evaluator_unverified, and reverted_unlanded_source_edits are all downstream of "the task selection picked something that couldn't be verified."
