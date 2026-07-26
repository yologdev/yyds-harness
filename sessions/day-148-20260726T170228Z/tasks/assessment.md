# Assessment — Day 148

## Build Status
**Pass.** Preflight `cargo build && cargo test` ran green before this assessment phase. Binary is `yyds v0.1.14 (f9621933 2026-07-26) linux-x86_64`.

## Recent Changes (last 3 sessions)

**Day 148 02:50 — productive:** Added zero-token diagnostic to `ModelCallCompleted` in `src/prompt.rs` (+68 lines). When the model returns zero tokens consumed AND produced, the event is tagged with `zero_tokens` error label. Build fix followed (variable scoping inside retry loop). Also fixed `scripts/preseed_session_plan.py` task seeder: `_check_code_already_exists` was grep-ing through `.py`/`.sh` files and finding its own task definition text, creating false contradictions. Now only checks Rust source files.

**Day 148 10:02 — empty:** Journal entry only. Exit code 1. `FailureObserved` retroactively recorded by `append_terminal_state_events.py`. The run completed with status `error` but the agent produced no code. State doctor says: "run completed with error status 'error' but no FailureObserved was recorded" — the harness patched it retroactively.

**Day 147 — three empty sessions:** 02:42, 09:48, 16:58 all produced journal entries only. Three consecutive sessions found a clean tree. Before that, Day 146 had four productive sessions (error message rewrites, diagnostic filter fix, test for diagnostic error pocket, graph hotspots `--kind` suggestions).

Pattern: 6 of the last 8 sessions landed zero code. The productive ones are islands. The planning pipeline (preseed → assess → plan → implement) produces empty sessions when it can't find actionable work.

## Source Architecture

84 `.rs` source files, ~151K total lines. Entry point: `src/bin/yyds.rs`.

**Core layers (by line count):**

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 25042 | State CLI subcommands (tail, graph, why, trace, etc.) |
| `state.rs` | 8418 | Append-only event recorder on yoagent-state primitives |
| `commands_eval.rs` | 6713 | Eval subcommands (replay, run, harness patch proposal) |
| `commands_evolve.rs` | 5528 | Evolve CLI (harness propose, apply, reject) |
| `deepseek.rs` | 4122 | DeepSeek-native policy: prompt layout, thinking, FIM routing, cache |
| `tool_wrappers.rs` | 3640 | Tool decorators: GuardedTool, TruncatingTool, ConfirmTool, etc. |
| `cli.rs` | 3688 | CLI argument parsing, subcommands, configuration |
| `symbols.rs` | 3679 | AST-based symbol search/rename across files |
| `tools.rs` | 3488 | Builtin tools: StreamingBash, ProjectSearch, RenameSymbol, etc. |
| `commands_git.rs` | 3558 | Git subcommands (diff, log, review, etc.) |
| `commands_deepseek.rs` | 3265 | DeepSeek diagnostics (stream-check, fim-complete, cache-report) |
| `prompt.rs` | 3028 | Agent prompt execution, streaming, retry |
| `context.rs` | 3104 | Project context loading (YOYO.md, CLAUDE.md, git status, recent files) |
| `watch.rs` | 2938 | Watch mode: auto-fix loop after prompts |

**Key dependency:** `yoagent` (agent framework) + `yoagent-state` (event primitives). No local yoagent upstream repo configured. DeepSeek cache metrics are blocked by yoagent's `Usage` struct dropping `cache_read_input_tokens` / `cache_creation_input_tokens` (tracked in issue #90).

## Self-Test Results

- `yyds --version`: outputs `yyds v0.1.14 (f9621933 2026-07-26) linux-x86_64` — correct
- `yyds --help`: full help output, all subcommands listed — correct
- `yyds state tail --limit 20`: shows live events from this assessment session; recorder is functioning
- `yyds state why last-failure`: returned retroactive FailureObserved from the 10:02 session (`run completed with error status 'error'`)
- `yyds state graph hotspots --limit 10`: normal distribution (bash 4052, read_file 3199, search 1370, etc.)
- `yyds deepseek cache-report`: "no DeepSeek cache metrics recorded from agent chat completions" — yoagent limitation, not a bug in yyds

No regressions found.

## Evolution History (last 5 runs)

| Started | Conclusion | Notes |
|---|---|---|
| 2026-07-26 17:01 | *(running)* | This session |
| 2026-07-26 10:00 | success | Journal-only, exit code 1 |
| 2026-07-26 02:50 | cancelled | Landed code (zero-token diagnostic + seeder fix), but run was cancelled |
| 2026-07-25 16:58 | success | Journal-only, no tasks |
| 2026-07-25 09:47 | success | Journal-only, no tasks |

The 02:50 session landing code while being "cancelled" is suspicious — it did real work (commits pushed) but the CI run was cancelled, likely by the next cron invocation overlapping (the 10:02 session). This is the session-budget race condition tracked in issue #262 — the hourly cron fires while a previous session is still running, and GitHub cancels the in-flight one. The `YOYO_SESSION_BUDGET_SECS` env var exists in code but isn't exported in `evolve.sh`.

## yoagent-state DeepSeek Feedback

**state why last-failure:** Retroactive FailureObserved from run `run-1785061468101-22454` (the 10:02 empty session). The run completed with error without recording its own failure; `append_terminal_state_events.py` patched it. Similar failures: 3 other `source=unknown, signal=failure recorded` events. This is the "crash boundaries are where evidence goes to die" pattern from Day 115's learnings — every 2-3 sessions produces an unclean lifecycle that needs retroactive patching.

**graph hotspots:** Normal distribution. No outlier tools. The `call_00_00Cx8K3UzrkACYiknS1X1119` unknown kind with degree 2 is a stale artifact (truncated tool_call_id appears as a node).

**deepseek cache-report:** No cache metrics from agent chat completions. yoagent's `Usage` struct drops DeepSeek-specific fields. Issue #90 tracks the upstream fix needed. Cache metrics ARE captured for `stream-check` and `fim-complete` diagnostic paths.

**PatchEvaluated:** Last 5: passed, failed (d05b92c5), passed, passed, passed. One recent failed eval — the d05b92c5 patch evaluation.

## Structured State Snapshot

**Claim health:** 1683/1989 proven (84.6%). 306 non-proven: 219 missing, 87 observed. 11 recent non-proven claims, dominated by `run_lifecycle=5 missing` and `model_lifecycle=3 missing, model_lifecycle=2 observed`.

**Unresolved claim families:**
- run_lifecycle gaps: 5 recent missing — run lifecycle events not fully closed
- model_lifecycle gaps: 3 missing, 2 observed — model call start/complete mismatches
- state_unmatched_non_validation: 1 — event recorded without matching validation

**Task-state counts** (from trajectory): 1/3 strict verified in day-148 05:20 session; `obsolete_already_satisfied=1`, `reverted_unverified=1`. Prior sessions had zero tasks attempted.

**Graph-derived next-task pressure (from trajectory):**
1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files. Action: bound discovery and require a selected task artifact before implementation work starts.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_abnormal_completed_count=1`): Lifecycle causes: `state_unmatched/open_after_FailureObserved=2`; model call events without matching completions.
3. **Raise session success rate** (`session_success_rate=0.0`): The evo session did not complete cleanly.
4. **Validate seeded tasks against fresh assessment** (`task_seed_contradiction_count=1`): Seeded tasks contradicted assessment evidence; validate seeds before implementation.
5. **Bound evaluator checks so verdicts are not skipped** (`evaluator_unverified_count=1`): Recent task session had unverified evals.

**Recent tool failures:** None reported in current trajectory snapshot (all recent sessions were empty or journal-only).

**Historical unrecovered tool failures:** Not applicable — no current tool-failure categories in the trajectory snapshot. The recent history shows clean tool execution; the failures are at the session/planning level, not the tool level.

## Upstream Dependency Signals

**yoagent blocks DeepSeek cache observability (issue #90):** yoagent's `Usage` struct only exposes standard `input_tokens`/`output_tokens` fields. DeepSeek adds `cache_read_input_tokens` and `cache_creation_input_tokens` which are silently dropped. This makes yyds blind to prompt caching efficiency — a core DeepSeek cost/reliability metric. The fix requires an upstream yoagent change to expose these fields. Since no yoagent upstream repo is configured for direct PRs, the path is: file a yyds help-wanted issue or propose an upstream yoagent PR (needs human routing).

**Session-budget cron race (issue #262):** The in-code `YOYO_SESSION_BUDGET_SECS` mechanism exists in `prompt_budget.rs` but `evolve.sh` doesn't export the env var. The 02:50 session that landed real code was cancelled (likely by the 10:02 cron invocation), and this same problem keeps eating productive sessions. The shell-side export in `evolve.sh` was documented as "a separate (human-approved) follow-up" — it's still pending.

## Capability Gaps

1. **Planning pipeline produces empty sessions too often.** 6 of 8 recent sessions produced zero code. The preseed task picker can't find actionable work when the codebase is healthy, and the fallback behavior sometimes creates self-referential tasks (the "fix yourself" loop from Day 115's learning). The trajectory diagnostic `planner_no_task_count=1` confirms this.

2. **DeepSeek prompt cache metrics invisible during agent runs.** Without cache token visibility, yyds can't measure or optimize its largest operational cost. This is blocked on upstream yoagent.

3. **Session lifecycle events still have gaps.** `run_lifecycle=5 missing`, `model_lifecycle=3 missing` — the state recorder's lifecycle tracking is incomplete. The Day 142 fix (hello-before-goodbye guard in `prompt.rs`) and Day 148's zero-token fix helped but gaps persist.

4. **No competitor parity measurement.** The trajectory's capability fitness score is "unknown." There's no structured way to measure how close yyds is to Claude Code parity for real DeepSeek-backed coding work.

## Bugs / Friction Found

1. **[HIGH] Planning pipeline can't distinguish "healthy codebase" from "broken planner."** The trajectory's `planner_no_task_count=1` and the 6-of-8 empty sessions suggest the planner needs a minimum-output guarantee: if it can't find a task, it should explain why in a structured way, not just produce nothing. The `logs feedback` says "planner produced no usable task -> bound discovery and require a selected task artifact before implementation work starts."

2. **[MEDIUM] Seeded task contradictions (#144 open).** The preseed task seeder's `_check_code_already_exists` has false positives. Day 148 02:50 fixed one case (checking .py/.sh files for Rust code), but issue #144 remains open and describes a reverted fix — meaning the contradiction detection still needs work.

3. **[MEDIUM] Model lifecycle gap (#134 open).** `deepseek_model_call_abnormal_completed_count=1` with `state_unmatched/open_after_FailureObserved=2`. Model call events don't always have matching completions. Issue #134 was filed as a reverted task.

4. **[LOW] Session budget not enforced in evolve.sh (#262).** The in-code budget mechanism exists but isn't activated. Productive sessions get cancelled by overlapping cron invocations. This is a harness-config change, not a code change.

5. **[LOW] Evaluator verdicts sometimes skipped.** `evaluator_unverified_count=1` — one recent task session had unverified evals. Low priority because it's a single occurrence, but the trajectory flags it.

## Open Issues Summary

4 open `agent-self` issues, all reverted tasks:
- **#144:** Fix false contradiction detection in `_check_code_already_exists` — partially addressed Day 148 but reverted
- **#135:** Break self-referential planning fallback when analysis-only pressure is active — reverted
- **#134:** Close harness-internal model lifecycle gap — reverted
- **#105:** Record DeepSeek prompt cache metrics during prompt runs — reverted

All four are reverted past attempts. #144 is the most actionable (partially addressed recently). #134 directly corresponds to the trajectory pressure point "Close yyds state and model lifecycle gaps."

## Research Findings

**External project journal (`journals/llm-wiki.md`):** Last active 2026-04-06. Built a personal wiki system with ingest/browse/query/lint operations using Next.js + Claude API. Hasn't been touched in 3+ months. Not relevant to current harness evolution.

**Competitor landscape:** No new curl-based research performed — the assessment budget prioritizes current harness evidence over landscape scanning. The known gap remains: Claude Code has structured multi-file editing, Cursor has IDE integration, and yyds is a terminal agent. The relevant competitor parity question is about DeepSeek-backed reliability, not feature count.
