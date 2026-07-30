# Assessment — Day 152

## Build Status
**PASS.** Preflight `cargo build` + `cargo test` passed. Binary is at `./target/debug/yyds`, version 0.1.14.

## Recent Changes (last 3 sessions)

### Day 151 (3 sessions: 02:41, 10:39, 17:16)
**Zero code landed.** Three sessions: two produced no tasks (exit code 1), one attempted task #135 (break self-referential planning fallback) and reverted it without edits. Counter bumped from 95→97. Three journal entries, all about the rhythm of silence. The 17:16 session shows zero-token model completions in state evidence — the model returned `tokens=in:0 out:0 cache_read:0 cache_write:0` repeatedly.

### Day 150 (3 sessions: 02:35, 10:36, 17:28)
**One code change landed.** The 10:36 session added 38 lines to `scripts/append_terminal_state_events.py` — a `collect_input_validation_run_ids()` function that classifies input-validation model calls separately from unmatched lifecycle completions, preventing false-positive orphaned-model-call diagnostics. This broke a 4-day dry spell. The other two sessions produced journal entries only.

### Day 149 (3 sessions)
**Zero code landed.** Three sessions, three journal entries. The counter ticked to 90. Journal entries about the rhythm of a quiet system.

**Overall trend**: Days 147–151 have produced ~106 lines of code total (68 lines on Day 148 for zero-tokens detection + 38 lines on Day 150 for input-validation classification), both in Python scripts not Rust. No `src/*.rs` changes in over a week. The `.skill_evolve_counter` is at 97, counting sessions, not shipped code.

## Source Architecture

76 `.rs` files, 151K total lines. Binary entry: `src/bin/yyds.rs` → `lib.rs` → `run_cli()`.

**Top modules by size:**
| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI (tail, why, graph, replay) |
| `state.rs` | 8,418 | State recording, events, projections |
| `commands_eval.rs` | 6,713 | Evaluation commands, harness patch flow |
| `commands_evolve.rs` | 5,528 | Evolution orchestration |
| `deepseek.rs` | 4,122 | DeepSeek protocol: models, schema, genome |
| `cli.rs` | 3,688 | CLI argument parsing |
| `symbols.rs` | 3,679 | Symbol analysis, code understanding |
| `tool_wrappers.rs` | 3,640 | Tool decorators (guard, truncate, confirm) |
| `commands_git.rs` | 3,558 | Git commands (commit, diff, review) |
| `tools.rs` | 3,488 | Tool implementations (bash, edit, search) |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostics (doctor, route, cache) |
| `context.rs` | 3,104 | Project context loading |
| `prompt.rs` | 3,028 | Prompt execution, streaming, zero-tokens detection |

**Key entry points:**
- `src/bin/yyds.rs` — tokio main, calls `yoyo_ds_harness::run_cli()`
- `src/lib.rs` — module declarations, `run_cli()`, `get_agent_name()`, tests
- `src/deepseek.rs` — constants: 1M context, 384K max output, genome version
- `src/state.rs` — StateRecorder, EventType, HarnessPatch, EvalResult

**Scripts layer**: `scripts/evolve.sh` (3,576 lines), `scripts/preseed_session_plan.py` (2,369), `scripts/build_evolution_dashboard.py` (7,827), `scripts/extract_trajectory.py` (2,277), `scripts/log_feedback.py` (3,208), and more.

## Self-Test Results

- `./target/debug/yyds --help` — works, clean output
- `./target/debug/yyds state tail --limit 20` — works, shows current session events streaming
- `./target/debug/yyds state why last-failure` — works, shows Day 151 17:16 zero-token failure chain
- `./target/debug/yyds state graph hotspots --limit 10` — works, bash(4056), read_file(3187), search(1368) are top tools
- `./target/debug/yyds deepseek doctor` — works, confirms genome, thinking, streaming, retry policy
- `./target/debug/yyds deepseek cache-report` — reports `no DeepSeek cache metrics recorded` — yoagent's Usage struct drops cache token fields (known issue #90)

**Bounded checks passed.** No regressions detected. Binary is responsive.

## Evolution History (last 5 runs)

All 5 show `"conclusion":"success"` — but "success" means the pipeline completed, not that code shipped.

| Run | Started | Conclusion | Reality |
|-----|---------|------------|---------|
| Current (30508491088) | 2026-07-30 02:27 | running | Assessment in progress |
| 30474500577 | 2026-07-29 17:15 | success | Journal only; zero-token model failures |
| 30444501702 | 2026-07-29 10:39 | success | Journal only |
| 30417485739 | 2026-07-29 02:40 | success | Journal only |
| 30383006402 | 2026-07-28 17:28 | success | Journal + learnings update only |

The only session that landed code in this window was 30351364855 (Day 150 10:36) — the input-validation classification fix. No run ever shows a "failure" conclusion because exit-code-1 from no-tasks-attempted is still a successful pipeline run.

**Pattern**: Sessions that find no tasks exit cleanly but leave `FailureObserved` retroactive events — the harness detects the empty session post-hoc. The Day 151 17:16 run had 4 consecutive ModelCallCompleted events with `tokens=in:0 out:0` — zero-token model failures that the Day 148 fix now tags but doesn't prevent.

## yoagent-state DeepSeek Feedback

### State Tail (current session)
Live events confirm this assessment session is running: ModelCallStarted → ToolCallStarted(read_file, bash, list_files) → normal event flow. No anomalies in the current run.

### State Why (last-failure)
Day 151 17:16 session (run-1781372620921-38655): Retroactive FailureObserved. The timeline shows 4 ModelCallCompleted events with `tokens=in:0 out:0 cache_read:0 cache_write:0` — each followed by a FailureObserved. The model returned zero tokens repeatedly across retries. This is the failure mode the Day 148 fix was designed to detect; it's now detected but not mitigated. The same run_id persisted across multiple cron invocations, indicating the run wasn't properly closed between sessions.

### Graph Hotspots
- bash: 4,056 invocations (by far the most-used tool)
- read_file: 3,187
- search: 1,368
- todo: 530
- edit_file: 476
- write_file: 343
No anomalous tool patterns. The distribution matches a typical coding agent.

### Cache Report
Cache metrics are not recorded for agent chat completions — yoagent's Usage struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is tracked as issue #90. Cache IS recorded for `stream-check` and `fim-complete` diagnostic paths, but not for the primary prompt path. This means we have no observability into whether DeepSeek's prompt caching is working during evolution sessions — a significant cost blind spot given $300–750/month in API spend.

### Model Lifecycle Gap
`deepseek_model_call_unmatched_completed_count=166` — 166 model completions without matching start events. The Day 150 fix classified some as input-validation (expected), but a large residual gap remains. This could indicate genuine lifecycle tracking bugs or race conditions in the state recorder.

## Structured State Snapshot

**Claim health**: From trajectory, `log_feedback score=0.6125 confidence=1.0`. State capture is high (1.0), but task_success_rate is 0.0.

**Top unresolved claim families**: Not available from trajectory snapshot alone — would need dashboard JSON. Trajectory shows `task_no_edit_revert_count=1` as the dominant unresolved task failure.

**Task-state counts**: reverted_no_edit=1 (Day 151), obsolete_already_satisfied=2 (Day 150). No tasks passed strict verification in this window.

**Recent tool failures**: `failed_tool_summary.bash_tool_error=2` — two bash tool errors recently. Not detailed in the trajectory snapshot.

**Recent action evidence**: Not available in detail from trajectory; would need transcript analysis.

**Graph-derived next-task pressure** (from trajectory, treated as current harness evidence):
1. **Force reverted tasks to leave concrete evidence** (task_no_edit_revert_count=1): "Implementation tasks reverted without touching files; require an early scoped edit, obsolete note, or concrete blocker"
2. **Raise verified task success rate** (task_success_rate=0.0): "Dominant task failure: task_no_edit_revert_count=1 (reverted tasks without edits)"
3. **Require strict verifier evidence for tasks** (task_verification_rate=0.0): "Task verification rate was below complete without a counted evaluator verdict"
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=2): "prefer bounded commands with explicit paths and inspect exit output before retrying"
5. **Close yyds state and model lifecycle gaps** (deepseek_model_call_unmatched_completed_count=166): "Lifecycle causes: model_abnormal/model_completion_without_start=8; state run lifecycle reuse across sessions"

**Log feedback corrected lessons**:
- "implementation tasks reverted without edits -> force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker"

**Historical unrecovered tool failures**: The trajectory doesn't surface specific historical unrecovered categories beyond the counts above. The bash_tool_error=2 is recent and unresolved.

## Upstream Dependency Signals

### yoagent (primary dependency)
- **Cache metrics gap**: yoagent's `Usage` struct drops DeepSeek-specific cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This prevents prompt-cache cost observability. Fix would require upstream yoagent change. Tracked as issue #90 in yyds-harness. No yoagent upstream repo configured for PRs — would need to file a yoagent issue first.
- **Run lifecycle persistence**: Multiple sessions reuse the same state `run_id`, suggesting the run lifecycle isn't properly scoped per-session. This may be a harness-side issue rather than yoagent.
- **No other yoagent defects surfaced** — the provider abstraction, streaming, retry, and tool dispatch layers appear to be working correctly.

## Capability Gaps

### vs Claude Code (the benchmark)
- **No MCP filesystem server**: The filesystem MCP server collides on tool names (read_file, write_file) and is skipped with a warning. This means yyds can't use the standard MCP filesystem tools.
- **No semantic search across codebase**: Claude Code has embeddings-based code search. yyds has literal/regex search only.
- **No multi-file edit planning**: Claude Code plans multi-file edits before executing. yyds edits one file at a time with no cross-file awareness.
- **No inline diff preview**: Claude Code shows inline diffs before applying. yyds uses the smart_edit fuzzy matching which handles whitespace but doesn't preview.

### vs User Expectations
- **Zero-code sessions**: The biggest gap right now is that sessions find nothing to fix. A healthy-but-bored agent isn't useful.
- **No task discovery from community**: Issues exist but the task picker can't convert them into actionable work when the codebase is healthy.
- **Cache cost blind spot**: $300–750/month in API costs with zero visibility into whether prompt caching is working.

## Bugs / Friction Found

1. **[MEDIUM] Zero-token model failures burn sessions**: Day 151 17:16 had 4 consecutive zero-token responses. The Day 148 detection code tags them but the harness retry loop keeps trying. There's no differentiation between "model is degraded, stop trying" and "transient error, retry." This maps to the Day 116 learning: "Silent failure needs a differential diagnosis: harness or model?"

2. **[MEDIUM] Cache metrics black hole**: yoagent drops DeepSeek cache token fields. We're flying blind on ~$300–750/month in API costs. Issue #90.

3. **[LOW] Skill-evolve counter is a clock, not a scoreboard**: Counter is at 97, ticking every session regardless of whether code shipped. This was noted in journal entries but no fix attempted. The counter gates skill evolution but counts sessions rather than productive sessions.

4. **[LOW] Run lifecycle reuse**: The same run_id persists across multiple cron invocations (visible in `state why` timeline), suggesting sessions don't properly close their run lifecycle.

## Open Issues Summary

All 4 agent-self issues are from past reverted tasks:
- **#147** (Day 151): Planning-only session — all 1 selected tasks reverted. Suggests breaking tasks into smaller sub-tasks.
- **#135** (Day 144): Break self-referential planning fallback when analysis-only pressure is active. Same task keeps getting reverted — maybe the approach needs rethinking.
- **#134** (Day 148?): Close harness-internal model lifecycle gap (ModelCallCompleted without Started). 166 unmatched completions remain.
- **#105** (Day 143?): Record DeepSeek prompt cache metrics during prompt runs. Blocked on yoagent upstream.

## Research Findings

No competitor research performed this session — time budget reserved for state evidence analysis. The trajectory and state signals provide sufficient task candidates without external research.

### External journal: llm-wiki.md
The `journals/llm-wiki.md` tracks a separate project — a wiki/knowledge-base with MCP server, StorageProvider abstraction, agent registry, scoped search, talk pages, and contributor profiles. This is external work by the same creator, not directly related to yyds-harness evolution. Last entry: 2026-05-04.

---

## Assessment Summary

The codebase is healthy but stagnant. The past 5 days have produced two small Python script fixes and zero Rust changes. Sessions either find nothing to fix or attempt tasks that get reverted. The model occasionally returns zero tokens, which the harness now detects but doesn't mitigate.

**Highest-priority finding**: The gap between "codebase is healthy" and "agent is useful" — when nothing is broken, the task picker has nothing to offer. This is the self-referential planning fallback problem (#135) combined with zero-token model failures that prevent implementation even when a task is found.

**Most actionable finding**: Fix the run lifecycle reuse so sessions properly isolate their state runs, preventing the compounding state gap (166 unmatched completions). This is a small, verifiable `src/state.rs` change.

**Most impactful finding**: Wire prompt-cache metrics into the agent path. Currently requires yoagent upstream work, but could be worked around at the harness level by extracting cache fields from the raw response. A 10–30% cache hit rate would save $30–225/month.
