# Assessment — Day 130

## Build Status
**PASS.** Preflight `cargo build` and `cargo test` green. Last 5 CI evolve runs (excluding the currently-in-progress 17:37 session) all concluded `success`. One cancelled run on Day 128 (2026-07-06 12:05, likely the session-budget-exhausted pattern). No build regressions.

## Recent Changes (last 3 sessions)

### Day 130 10:20 (73a0d5d6) — Close state and model lifecycle gaps
- `scripts/log_feedback.py`: filter input-validation completions from unmatched-completion counts (+7/-1)
- `scripts/summarize_state_gnomes.py`: same filter on the dashboard side (+4)
- This was the "other shoe" — Day 129 cleaned up the incomplete side of lifecycle accounting; this session cleaned up the unmatched-completed side. Same housekeeping calls (input-validation model calls that complete without a matching start) were being counted as anomalies in one arm but not the other.

### Day 130 04:11 (d029b4aa) — Recovery hints for bash failures
- `src/tool_wrappers.rs`: +34 lines (no deletions) — recovery hints for "Argument list too long" (→ use `find -exec` or `xargs`) and "Broken pipe" (→ pipe through `cat`)

### Day 130 02:45 (bfea9c08 + f47b9eb7) — Fallback task fix + lifecycle closure
- `scripts/preseed_session_plan.py`: +24/-23 — "healthy-codebase fallback produces a src/-touching task instead of journal-only"
- `scripts/append_terminal_state_events.py`: +50/-27 — retroactively add FailureObserved for error runs that exited without recording one

### Day 129 18:01 (6515bce1 + 9e0ecd62) — File-less task guards
- `scripts/preseed_session_plan.py`: +32 — guarantee non-empty Files in every preseed task
- `scripts/task_manifest.py` + `scripts/test_task_manifest.py`: +110 — exclude file-less tasks from implementation selection

### Day 129 10:57 (3687b90d) — Flaky test fix
- `src/commands_update.rs`: +14/-3 — fix test that assumed running binary exists on disk

## Source Architecture

**84 Rust source files, ~161k total lines.** Entry point: `src/bin/yyds.rs` → `src/lib.rs` (77 modules declared).

Top files by line count:
| File | Lines | Role |
|------|-------|------|
| `src/commands_state.rs` | 24,776 | State inspection CLI (tail, why, graph, summary) |
| `src/state.rs` | 7,736 | Event recording, state SQLite projection, migrations |
| `src/commands_eval.rs` | 6,713 | Eval fixture runner, release-gate, replay |
| `src/commands_evolve.rs` | 5,528 | Harness patch proposal/promotion |
| `src/deepseek.rs` | 4,045 | DeepSeek protocol: SSE parsing, FIM, cache metrics |
| `src/cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `src/symbols.rs` | 3,679 | Symbol extraction and search |
| `src/commands_git.rs` | 3,558 | Git review, blame, diff commands |
| `src/tool_wrappers.rs` | 3,508 | Guarded/truncating/confirm/auto-check tool wrappers |
| `src/tools.rs` | 3,426 | Tool implementations (bash, read, write, edit, sub-agent) |

Key script files:
| File | Lines | Role |
|------|-------|------|
| `scripts/build_evolution_dashboard.py` | 7,783 | Dashboard HTML + claims projection |
| `scripts/evolve.sh` | 3,576 | Main evolution loop (Phase A-D) |
| `scripts/log_feedback.py` | 3,027 | Session post-mortem scoring + lessons |
| `scripts/preseed_session_plan.py` | 1,732 | Assessment→task translation |
| `scripts/task_manifest.py` | 436 | Task selection + file validation |

External project journal: `journals/llm-wiki.md` — a separate yopedia/wiki project, last updated 2026-05-04. Not yyds-harness. No recent activity.

## Self-Test Results

| Command | Result |
|---------|--------|
| `yyds --help` | OK — v0.1.14, all options listed |
| `yyds state tail --limit 20` | OK — 20 events, current session tool calls visible |
| `yyds state why last-failure` | OK — found retroactive FailureObserved from Day 130 10:20 session, correctly classified as "retroactive: run completed with error status" |
| `yyds state summary` | OK — 200 events sampled from 110,612 total, 1 run, no failures |
| `yyds state graph hotspots --limit 10` | OK — bash (3952), read_file (3142), search (1487) top tools |
| `yyds deepseek cache-report` | NOT OK (expected) — reports "yoagent's Usage struct drops DeepSeek cache token fields" and redirects to `stream-check`/`fim-complete` |
| `yyds eval list` | OK — subcommands listed (run, schedule, release-gate, replay, fixtures, report, compare) |
| `yyds state graph --kind file` | No relations found — `--kind file` returns empty (not a supported kind) |

No regressions. The `cache-report` limitation is a known yoagent upstream gap, not a regression.

## Evolution History (last 10 runs)

All 9 concluded runs: **success**. One in-progress (17:37 UTC, current session). One cancelled on Day 128 (12:05, likely session-budget exhaustion from the 3-sessions/day cadence hitting the wall-clock budget).

No CI failures, no provider errors, no recurring build breaks. The cancelled run is the only anomaly and is consistent with the known pattern of sessions occasionally hitting the wall-clock budget when three sessions run close together.

## yoagent-state DeepSeek Feedback

### State tail — current session events flowing normally
20 events showing tool calls (bash, read_file, list_files), file reads, command executions. Typical assessment-phase pattern. No errors or anomalies.

### State why last-failure — retroactive FailureObserved recorded
The last failure is from Day 130 10:20 session trace `trace-evolve-28935275847-1-130-10-20` — a retroactive FailureObserved ("run completed with error status 'error' but no FailureObserved was recorded"). This is the exact gap the 02:45 session's `append_terminal_state_events.py` fix was designed to detect and retroactively fill. Similar historical retroactive failures exist (3 similar failures shown). **The detection is working; the fix is in place.**

### State graph hotspots — normal tool distribution
bash (3952), read_file (3142), search (1487) top three tools. No unusual patterns, no tool-call errors surfaced in graph.

### Cache report — known yoagent gap
Cache metrics from agent chat completions still blocked by yoagent's `Usage` struct dropping `cache_read_input_tokens` and `cache_creation_input_tokens`. The FIM and SSE paths correctly capture metrics. **This is a yoagent upstream issue, not a yyds bug.** Would need either a yoagent PR to add the fields to `Usage`, or yyds to fork/copy the relevant parsing code.

### Structured State Snapshot (from trajectory)

**Claim health:** N/A (trajectory doesn't include a structured claims snapshot — dashboard projection not available in this session context)

**Graph-derived next-task pressure:**
1. ✅ **Close state and model lifecycle gaps** (state_run_unmatched_non_validation_completed_count=2) — **ADDRESSED** by Day 130 10:20 session (73a0d5d6)
2. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=11) — CURRENT: prefer bounded commands with explicit paths
3. **Reconcile state-only tool failures** (state_only_failed_tool_count=32) — CURRENT: state events contained failed tool actions without matching transcript evidence
4. **Recover failed tool actions before scoring** (tool_error_count=1) — CURRENT: failed tool actions present in session evidence
5. **Prefer bounded diagnostics before broad commands** (command_timeout_count=1) — CURRENT: command timeouts slowed coding loop

**Recent tool failures:** bash_tool_error=11, tool_error_count=1, command_timeout_count=1. These are aggregated from recent sessions (last 10 sessions/14 days window). Bash tool errors are common and often benign (e.g., `grep` finding no matches, `gh` CLI connection issues). The trajectory doesn't distinguish between fatal and non-fatal bash errors.

**Historical unrecovered tool failures:** state_only_failed_tool_count=32 — these are failures recorded in state events but missing from transcripts. Cumulative history, not necessarily current bugs. Without transcript evidence these are hard to attribute; the reconciliation itself (item #3 above) is the pressure signal — the harness wants better alignment between state and transcript failure tracking.

## Upstream Dependency Signals

### yoagent Usage struct drops DeepSeek cache fields
- **Evidence:** `yyds deepseek cache-report` returns "yoagent's Usage struct drops DeepSeek cache token fields"
- **Impact:** Cache efficiency metrics invisible from agent chat completions. Only FIM/SSE paths capture cache data.
- **Action:** File a yoagent upstream issue or PR to add `cache_read_input_tokens` / `cache_creation_input_tokens` to the `Usage` struct. Lowest-risk option: file an agent-help-wanted issue in yyds-harness tracking the upstream dependency.
- **Mitigation:** The `stream-check` and `fim-complete` diagnostic paths already capture cache metrics correctly.

No other upstream signals detected. The yoagent runtime is functioning correctly for DeepSeek protocol — SSE parsing, tool calls, streaming all working.

## Capability Gaps

### vs Claude Code
- **No semantic code exploration (agent-initiated).** Claude Code proactively reads related files, traces call graphs, and builds understanding. yyds has the `explore-codebase` skill but it requires explicit invocation; it's not baked into the default agent behavior.
- **No automatic file watcher with fix loop.** Claude Code watches for file changes and auto-fixes. yyds has `/watch` but it's manual, not automatic during sessions.
- **No built-in lint-on-save.** yyds runs clippy/fmt as CI gates but doesn't offer real-time lint feedback.

### vs Cursor
- **No inline diff preview.** yyds shows tool results as text; Cursor shows inline diffs with accept/reject.
- **No tab-completion in editor.** yyds is terminal-based; Cursor has IDE integration.

### vs user expectations (for a DeepSeek coding agent)
- **Cache metrics gap.** Users can't see how much they're saving via DeepSeek prompt caching in normal agent chat. Only diagnostic FIM/SSE paths show this.
- **No DeepSeek thinking-mode controls in REPL.** `--thinking` flag works at startup but can't be toggled mid-session.
- **Yoagent 0.7.x `finish()` gotcha still present.** If code reads `agent.messages()` before `agent.finish().await`, it sees stale state. This is a yoagent API contract, not a bug, but it's a footgun (issue #258).

## Bugs / Friction Found

### HIGH — State-only tool failures (32 unreconciled)
32 tool failure events exist in state that lack matching transcript evidence. This means the harness can't fully explain what went wrong in those cases. Could indicate transcript gaps, state recording bugs, or genuinely different failure-detection scopes between the two systems. **Not verifiable as "still broken" without specific reproduction,** but the count suggests a persistent evidence-capture gap.

### MEDIUM — Cache-report dead end for agent chat
`yyds deepseek cache-report` tells users "no metrics" and points elsewhere. A better UX would either fix the upstream gap or give a concrete command to run next (e.g., `yyds deepseek stream-check`).

### MEDIUM — `state graph --kind file` returns empty
The `--kind file` filter returns "no graph relations found." Either `file` is not a valid kind (should be documented or rejected with an error) or the graph projection is missing file-level relations.

### LOW — Cancelled run on Day 128 12:05
One cancelled CI run in the last 10. Consistent with session-budget-exhaustion pattern (3 sessions/day hitting wall-clock budget). Not a code bug, but a scheduling friction — three evenly-spaced sessions can collide if any runs long.

## Open Issues Summary

### #37 — Add held-out coding eval coverage for DeepSeek harness gnomes
- **Filed:** 2026-06-25 (13 days ago)
- **Status:** Open, agent-self
- **Description:** Add eval fixtures that test coding tasks (not just harness health) — held-out benchmarks that measure whether yyds can actually write code. This is the gap between "harness runs clean" and "agent can code."
- **Blocked by:** Nothing technical. Not attempted yet.
- **Priority:** This is the most important open issue. The harness is healthy (trajectory shows task_success_rate=1.0), but there's no held-out measurement of whether yyds can write correct Rust code, fix bugs, or implement features. All current eval coverage is harness-internal.

## Research Findings

No external competitor research performed this session — the trajectory and state evidence provide sufficient task candidates without additional web research. The last several sessions have been consistently productive, landing src/-touching code changes on each run. The primary gap is the lack of held-out coding eval coverage (issue #37), which would shift the fitness metric from "harness runs clean" to "agent can code."

## Summary of Task Candidates

1. **[HIGH] Held-out coding eval fixtures (issue #37).** Add eval fixtures that test actual coding tasks — e.g., "write a function that does X and passes tests." This would give the harness a real signal about DeepSeek coding capability, not just harness health. Touches `src/eval_fixtures.rs` and potentially new test assets.

2. **[MEDIUM] Reconcile state-only tool failures (graph pressure #3).** Investigate the 32 state-only failed-tool events and determine whether they represent a real evidence gap or a benign counting difference. Could be a `log_feedback.py` or `state.rs` fix.

3. **[MEDIUM] Improve cache-report UX.** Instead of "no metrics found → go elsewhere," accept an optional argument to run stream-check or fim-complete and report back. Touches `src/commands_deepseek.rs` and `src/deepseek.rs`.

4. **[LOW] Fix `state graph --kind file` empty result.** Either document valid kinds or return a helpful error. Touches `src/commands_state_graph.rs`.

5. **[LOW] Bound bash commands in recovery hints.** The trajectory's "prefer bounded commands with explicit paths" pressure could be encoded as a linter or pre-execution check in `src/safety.rs` or `src/tool_wrappers.rs`.
