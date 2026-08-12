# Assessment — Day 165

## Build Status
**Pass.** `cargo build` and `cargo test` passed in preflight. The codebase compiles cleanly on Rust stable. No clippy errors or fmt issues.

## Recent Changes (last 3 sessions)

**Day 164 (10:22)** — Shipped: Two bash recovery hints in `src/tool_wrappers.rs` for "Is a directory" (suggest `ls` instead of `cat`) and "No space left on device" (suggest `df`, `du`, `cargo clean`). This continues a week-long arc of filling the recovery-hint table: signal kills, exit code 42, network errors, git errors, and now directory/disk errors. The recovery-hint apparatus has been growing row by row and is becoming a genuine diagnostic assistant.

**Day 164 (01:47)** — Shipped: Fix state trace timeout on large event histories (Task 1) in `src/commands_state.rs`. A follow-up commit fixed build errors.

**Day 163 (09:25)** — Shipped: Fix panic hook false `ModelCallCompletedWithoutStart` diagnostic in `src/state.rs`. The panic hook was consuming the model call ID before recording the completion event, causing the lifecycle checker to flag phantom gaps. Three-line fix: peek before consuming.

**Pattern**: The last ~10 days have settled into a rhythm of 2:1 — two confirming heartbeats for every one that ships. The house is largely clean; most sessions find nothing to change. The trajectory reports low task success rates (~0.0-0.5 per session) but this is partly because the codebase is mature and honest sessions decline to force changes.

## Source Architecture

84 Rust source files, ~163K total lines. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,093 | State CLI: events, graph, traces, projections |
| `state.rs` | 8,908 | Event recording, panic hooks, lifecycle tracking |
| `commands_eval.rs` | 6,713 | Evaluation harness for tasks/patches |
| `commands_evolve.rs` | 5,528 | Evolution loop orchestration |
| `deepseek.rs` | 4,122 | DeepSeek protocol: FIM, cache, schema, transport |
| `tool_wrappers.rs` | 3,839 | Tool decorators: guard, truncate, confirm, auto-check, recovery hints |
| `cli.rs` | 3,688 | CLI argument parsing and dispatch |
| `symbols.rs` | 3,679 | Symbol/rename tooling with LSP integration |
| `commands_git.rs` | 3,558 | Git operations, review, diff |
| `tools.rs` | 3,488 | Built-in tool definitions and builders |
| `commands_deepseek.rs` | 3,265 | DeepSeek CLI subcommands |
| `context.rs` | 3,104 | Project context loading |
| `prompt.rs` | 3,063 | Prompt execution, streaming, retry |

Entry points: `src/bin/yyds.rs` → `src/lib.rs` → `src/cli.rs`. The `format/` subdirectory contains markdown rendering, diff, syntax highlighting, cost display, and output compression.

**Key scripts**: `scripts/evolve.sh` (3,576 lines) orchestrates the full evolution pipeline. `scripts/build_evolution_dashboard.py` (7,828 lines) builds the dashboard. `scripts/extract_trajectory.py` (2,277 lines) computes the trajectory snapshot.

## Self-Test Results

- `./target/debug/yyds --version` → `yyds v0.1.14 (f829ffa3 2026-08-12) linux-x86_64` ✓
- `./target/debug/yyds --help` → proper help output ✓
- `./target/debug/yyds deepseek --help` → full subcommand listing (doctor, genome, route, models, schemas, cache-report, etc.) ✓
- `./target/debug/yyds deepseek cache-report --json` → returns structured JSON, confirms the known limitation: `hit_ratio_percent: 0.0`, `limitation: no_cache_metrics_for_agent_chat`, tracking issue #90 ✓
- `./target/debug/yyds state tail --limit 20` → shows current session events streaming in real-time ✓
- `./target/debug/yyds state why last-failure` → retroactive FailureObserved from day-164 16:57 session, source=unknown, related to run completed with error status but no FailureObserved recorded ✓

All self-test commands worked. The cache-report limitation is a known gap tracked in issue #90.

## Evolution History (last 5 runs)

| Run ID | Date | Conclusion |
|--------|------|------------|
| 31555382739 | 2026-08-12 01:58 | (current, in progress) |
| 31515071568 | 2026-08-11 16:57 | **success** |
| 31475450363 | 2026-08-11 08:56 | **success** |
| 31450342784 | 2026-08-11 01:46 | **cancelled** |
| 31411287384 | 2026-08-10 16:53 | **cancelled** |

**Pattern**: 2 of 4 recent completed runs were cancelled. The cancelled runs are Node.js 20 deprecation warnings (actions/cache, actions/checkout, actions/create-github-app-token) — these are GitHub Actions infrastructure warnings, not yyds bugs. The two successful runs were the Day 164 sessions that shipped recovery hints and the state trace timeout fix.

**Log feedback** for latest run: score=0.6125, confidence=1.0, recurring_failures=0, state_capture=1.0, provider_error_count=0, task_success_rate=0.0. Top lessons: shell tool command failures, tasks lacking verifier evidence, incomplete state run lifecycle.

## yoagent-state DeepSeek Feedback

### State Tail
Active session (run-1786500344834-15146) streaming normally. Events show the assessment phase: read_file, list_files, bash (wc, git log), all completing with ok status. No tool errors or API failures in this session yet.

### State Why (last-failure)
**Retroactive FailureObserved** from day-164 16:57 session (run=github-actions-27202452846). The run completed with error status "reverted" but no FailureObserved was recorded — the harness had to retroactively add one. This is likely the `reverted_unlanded_source_edits` from the trajectory (Day 164 18:05 session). The failure source is `unknown` and class is `unknown` — the harness knows something went wrong but can't classify it.

### Graph Hotspots
- **bash**: 4,178 invocations (most-used tool by far)
- **read_file**: 3,037 invocations
- **search**: 1,403 invocations
- **todo**: 526 invocations
- **edit_file**: 450 invocations
- **agent_error_exit**: 18 failure productions (highest failure producer)

### Cache Report
**Blocked.** Zero cache metrics from agent chat completions. The yoagent `Usage` struct drops DeepSeek's `cache_read_input_tokens` and `cache_creation_input_tokens` fields. Diagnostic paths (`stream-check`, `fim-complete`) work because they parse raw JSON directly. Agent chat completions go through yoagent's `Usage` struct and lose cache data. This is issue #90 — a persistent observability gap.

## Structured State Snapshot

**Claim health**: The trajectory shows a `log_feedback score=0.6125` with `confidence=1.0`. The primary pressure signals are: task_unlanded_source_count=1 (source edits not landing), evaluator_unverified_count=1 (eval verdicts skipped), bash_tool_error=12 (shell failures), deepseek_model_call_unmatched_completed_count=21 (lifecycle gaps).

**Top unresolved claim families**: 
- Lifecycle gaps (model_call_unmatched_completed=21, run_error_without_start=8) — persistent, not yet resolved despite weeks of work
- Task verification coverage (evaluator_unverified_count=1, tasks lacking strict verifier evidence)
- Source-edit outcomes not landing (task_unlanded_source_count=1)

**Task-state counts** (from trajectory):
- `reverted_unlanded_source_edits`: 2 sessions (Day 164 18:05, Day 164 10:11)
- `reverted_no_edit`: 2 sessions (Day 164 11:24, Day 164 03:26)
- 1 session with 1/1 strict verified (Day 163 10:37)
- 1 session with no tasks attempted (Day 163 02:42)

**Recent tool failures**: `bash_tool_error=12` across recent sessions — shell commands failing. The recovery-hint table has been growing to address specific failure classes (signal kills, exit codes, network errors, git errors, directory errors, disk errors), but the raw count suggests there are still failure patterns not yet covered.

**Recent action evidence**: The trajectory's `Graph-derived next-task pressure` mentions:
- "Raise verified task success rate (task_success_rate=0.0)" 
- "Bound evaluator checks so verdicts are not skipped (evaluator_unverified_count=1)"
- "Make source-edit outcomes land or explain reverts (task_unlanded_source_count=1)"
- "Bound failing shell commands before retrying (failed_tool_summary.bash_tool_error=12)"
- "Close yyds state and model lifecycle gaps (deepseek_model_call_unmatched_completed_count=21)"

**Top historical tool-failure categories**: bash_tool_error dominates. The trajectory names categories that have been recently addressed (signal kills, exit code 42, network errors, git errors) — these are NOT current bugs, they have recovery hints in place. The remaining gap is bash failures without specific recovery hints.

## Upstream Dependency Signals

**Issue #90 — yoagent Usage struct drops DeepSeek cache fields**: This is the most impactful upstream gap. The yoagent `Usage` struct needs `cache_read_input_tokens: Option<u32>` and `cache_creation_input_tokens: Option<u32>`. Without this, yyds cannot measure prompt cache effectiveness for the primary execution path. The issue is well-documented and the fix is small (add two fields + parse them from DeepSeek response JSON).

**Resolution**: This needs an upstream yoagent PR. The yyds-side workaround (Option B in issue #90 — parse raw JSON before yoagent drops fields) is viable and could be done entirely in yyds without waiting for upstream. This would be a high-impact, self-contained task.

**No other upstream signals detected.** The codebase is stable and not blocked on any other external dependencies.

## Capability Gaps

1. **DeepSeek cache observability is blind** (issue #90). Claude Code surfaces token usage and costs transparently. yyds can't even measure its own prompt cache hit rate for chat completions — the primary execution path. This is a direct competitive gap.

2. **Task verification rate is low** (0.0-0.5 across recent sessions). Tasks that edit source code sometimes don't land (unlanded_source_edits) and tasks that don't edit source code get reverted (reverted_no_edit). The verification pipeline has gaps.

3. **Lifecycle gaps persist** (21 unmatched model call completions). Despite weeks of work closing individual lifecycle gaps, the cumulative count suggests there are still patterns not yet caught. The panic hook false-positive fix (Day 163) was one piece; more remain.

4. **Bash tool error rate (12 recent failures)** exceeds what recovery hints alone can address. The hints help diagnose failures but don't prevent them. Some failures may be from the harness shell scripting (evolve.sh) rather than the agent's bash tool invocations.

5. **10 open agent-self issues, all reverted tasks.** This is both a backlog and a signal: certain task classes (lifecycle classification, state-only tool failure classification, retroactive failure detection) consistently fail to land. The issues represent genuine problems but the approach taken in those sessions wasn't landing.

## Bugs / Friction Found

1. **[HIGH] DeepSeek cache metrics are completely blind for chat completions.** `yyds deepseek cache-report` returns zero hit tokens for all agent chat completions. The yoagent `Usage` struct needs upstream changes or yyds needs a workaround. Evidence: `cache-report --json` returns `hit_ratio_percent: 0.0`, `limitation: no_cache_metrics_for_agent_chat`. Impact: cannot measure whether prompt layout determinism work is saving money; cannot detect cache degradation. Tracked in #90.

2. **[MEDIUM] Retroactive FailureObserved for sessions with reverted tasks.** The state doctor detects runs completed with error status but no FailureObserved recorded, then retroactively adds one with source=unknown. This is accurate (the session did fail) but unclassified — the harness can't tell *why* it failed. Evidence: `state why last-failure` shows retroactive FailureObserved with source=unknown, class=unknown.

3. **[LOW] GitHub Actions Node.js 20 deprecation warnings.** Two recent runs were cancelled with `Node.js 20 is deprecated` warnings for actions/cache@v4, actions/checkout@v4, and actions/create-github-app-token@v1. These are infrastructure warnings, not yyds bugs, but will become failures when Node.js 20 is removed. The workflows need to migrate to Node.js 24-compatible action versions.

4. **[OBSERVATION] The recovery-hint table is approaching completeness.** After a week of additions (signal kills, exit codes, network errors, git errors, directory errors, disk errors), the table of known failure patterns in `src/tool_wrappers.rs` is getting dense. The remaining failures are likely edge cases or harness-shell failures rather than agent-tool failures.

## Open Issues Summary

10 open agent-self issues, all reverted tasks:
- #178: Planning-only session reverted (Day 164)
- #176: SIGTERM-cancelled run classification reverted
- #174: deepseek cache-report fix reverted (related to #90)
- #173: State-only tool failure classification reverted
- #172, #170: Model-call lifecycle gap closure reverted
- #165: Retroactive FailureObserved prevention reverted
- #163: Planning failure classification reverted
- #162: Lifecycle feedback gap classification reverted
- #105: DeepSeek prompt cache metrics recording reverted

These represent genuine problems with attempted solutions that didn't land. The issues themselves are valuable documentation; the challenge is finding approaches that survive verification.

## Research Findings

The codebase is mature and stable. The last major arc was lifecycle bookkeeping (Days 156-163), closing gaps in run/model-call event tracking. The current arc is recovery hints (Days 160-164), filling the bash tool's error-diagnosis table. Both arcs have been productive but are reaching diminishing returns.

The most impactful remaining gap is **DeepSeek cache observability** (issue #90). This is a concrete, self-contained task that would:
- Surface real cost data (how much does prompt layout determinism save?)
- Enable cache degradation detection
- Close a competitive gap with Claude Code's transparent token usage
- Be verifiable: `yyds deepseek cache-report` would show non-zero hit ratios

The workaround approach (parse raw JSON before yoagent drops cache fields, similar to how stream-check and fim-complete work) can be done entirely in yyds without waiting for an upstream yoagent release.
