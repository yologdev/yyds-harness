# Assessment — Day 109

## Build Status
**PASS** — preflight `cargo build` and `cargo test` green. Binary is `yyds v0.1.14 (e9c3cb4 2026-06-17) linux-x86_64`.

## Recent Changes (last 3 sessions)

| Session | Commit | What |
|---------|--------|------|
| Day 109 (16:49, current) | `e9c3cb4` | Keep fallback seed tasks landable — `preseed_session_plan.py` now avoids protected implementation files in fallback repair tasks; added self-tests for the guard |
| Day 109 (06:34) | `b8936ef` | Stop retrying analysis-only task attempts — `evolve.sh` now detects when implementation produces zero file changes and writes a blocked note instead of retrying |
| Day 109 (04:14) | `7f39e38` | Make analysis-only task pressure landable — preseed task picker learned to detect analysis-only pressure and skip lifecycle cleanup when agent can't edit |
| Day 108 (21:22) | — | Wired `state summary` command to the switchboard (was implemented but unreachable) |
| Day 108 (17:37) | — | Tests for `state why last-failure` cold-start output distinguishability |

All recent work clusters around **planning/execution reliability** (analysis-only detection, fallback task safety) and **state diagnostics** (cold-start help, command switchboard).

## Source Architecture

- **76 `.rs` files, ~147K lines total**
- **Top 10 by line count:** `commands_state.rs` (24.4K — state dispatch, diagnostics, graph), `state.rs` (6.9K — event store, crash stash), `commands_eval.rs` (6.6K — evaluator), `commands_evolve.rs` (5.5K — evolution CLI), `deepseek.rs` (3.9K — DeepSeek protocol), `cli.rs` (3.7K — CLI parsing), `symbols.rs` (3.7K), `commands_git.rs` (3.6K), `tools.rs` (3.4K), `tool_wrappers.rs` (3.2K)
- **Key entry points:** `src/lib.rs` (library root), `src/main.rs`-style binary in `src/bin/yyds.rs`, `src/cli.rs` for argument dispatch, `src/dispatch_sub.rs` for CLI subcommand routing
- **Critical harness files:** `scripts/evolve.sh` (3,505 lines — session orchestration), `scripts/preseed_session_plan.py` (921 lines — task seeding), `scripts/log_feedback.py` (2,925 lines — assessment/feedback), `scripts/extract_trajectory.py` (2,087 lines — trajectory report), `scripts/build_evolution_dashboard.py` (7,709 lines — dashboard)
- `commands_state.rs` at 24K lines is the largest single file — state diagnostics, graph queries, crash/failure/trace commands all live here. Worth splitting eventually but not a current friction point.

## Self-Test Results

- `yyds --version` → `yyds v0.1.14 (e9c3cb4 2026-06-17) linux-x86_64` ✓
- `yyds help` → produces full help text ✓
- `yyds state doctor` → All checks passed, 27,403 events, 31.6MB events file, 67.9MB SQLite store. Note: all event types classified as "unknown" — schema V3 may have a type-mapping gap. ✓ (no errors)
- `yyds state tail --limit 20` → shows current session events in real time ✓
- `yyds state why last-failure` → correctly reports current session in progress ✓
- `yyds state why last-crash` → no crashes found (10 preflight hidden) ✓
- `yyds state failures --recent` → 12 recent failures, all retryable, mostly tool_execution ✓
- `yyds state failures tools` → no tool failures found (0 in scanned range) ✓
- `yyds deepseek cache-report` → 95.77% hit ratio across 185 events ✓
- `yyds state graph hotspots --limit 10` → bash (3805), read_file (3094), search (1866) dominate ✓
- `yyds state summary` → shows current state of 27,388 total events ✓

**One observation:** `state doctor` reports `unknown=27403` for event types — all events are classified as "unknown" despite schema V3. This is a cosmetic issue (functional data is intact) but makes the doctor's type breakdown useless.

## Evolution History (last 5 runs)

| Run | Started | Conclusion | Notes |
|-----|---------|------------|-------|
| `27705233906` | 2026-06-17 16:49 | In progress | Current session (assessment phase) |
| `27688295326` | 2026-06-17 12:17 | success | Day 109 (12:17): tasks 0/1 ⚠️ — reverted_no_edit |
| `27670451412` | 2026-06-17 06:33 | success | Day 109 (06:34): tasks 0/1 ⚠️ — reverted_no_edit |
| `27654249291` | 2026-06-17 04:13 | success | Day 109 (04:14): tasks 1/1 ✅ — strict verified |
| `27591572611` | 2026-06-16 21:21 | success | Day 108 (21:22): tasks 1/2 ⚠️ — 1 reverted_no_edit |

**Pattern:** 2 of the last 3 sessions had `reverted_no_edit` tasks — implementation agents read and assessed but never produced file changes. The harness just learned to stop retrying these (Day 109 06:34), so this pattern should improve. Earlier Day 106 sessions had API/provider errors causing planning failures.

No recurring CI error fingerprints detected in recent runs. Cache performance is excellent (95.77% hit ratio). Provider errors were present in Day 106 but absent in Days 108-109.

## yoagent-state DeepSeek Feedback

### Cache
- **95.77% hit ratio** across 185 model events (123.5M hit tokens, 5.5M miss tokens)
- All events from `deepseek-v4-pro` model
- Cache efficiency is excellent — no regression signals

### Model lifecycle
- State shows model calls are being paired (start/completed events) — the Day 107 fix is working
- No abnormal completions, no unmatched completions detected

### PatchEvaluated scores (from events.jsonl)
Recent scores: 0.91 (passed, Day 105), 0.25 (failed, Day 106 API errors), 0.72 (failed, Day 106 reverted task), 0.25 (failed, Day 106 API errors + timeouts). Scores are properly distinguishing healthy from unhealthy sessions.

### Failure patterns (last 200 events)
12 retryable failures observed, all `tool_execution` and `transport`:
- `read_error` (directory read as file): 2 occurrences — assessment agents hitting `session_plan/` as a directory
- `missing 'path' parameter`: 3 occurrences — edit_file calls without path
- `search error: src/main.rs not found`: 2 occurrences — searching for a file that doesn't exist (binary is at `src/bin/yyds.rs`)
- `grep regex error`: 1 occurrence — unescaped parentheses
- `old_text not found` / `matches 44 locations`: edit failures that smart_edit recovered from
- `Command timed out after 120s`: 1 transport timeout

These are mostly agent tool-use errors, not DeepSeek protocol failures. The repeated `src/main.rs` search errors suggest the prompt/context may not clearly communicate that the binary entry point is `src/bin/yyds.rs`.

## Structured State Snapshot

### Claim health
- **504/621 claims proven; 117 non-proven** (88 missing, 29 observed)
- Recent non-proven claims: `run_lifecycle=2 missing`, `model_lifecycle=1 observed`
- Lifecycle aggregate: observed=60/69, unhealthy=37, run_incomplete=108, model_incomplete=53
- These are long-running concerns tracked over months, not new regressions

### Task-state counts (from trajectory)
- Recent task issues: `reverted_no_edit=3` — the dominant recent failure mode
- Task expected evidence: task_02 expects `src/commands_state.rs` changes; task_01 expects dashboard artifacts

### Recent tool failures (from state tail)
- `unrecovered=6/12` in recent window, `failed_commands=9`
- Top categories: read_error (2), missing parameter (3), path-not-found (2), regex error (1), timeout (1)
- All are agent-level tool misuse, not harness bugs

### Graph-derived next-task pressure (from trajectory, treated as current harness evidence)
1. **Force reverted tasks to leave concrete evidence** (`task_no_edit_revert_count=1`): Implementation tasks reverted without touching files; require an early scoped edit, an obsolete note, or a concrete blocker
2. **Raise verified task success rate** (`task_success_rate=0.0`): Dominant task failure: reverted tasks with no edits
3. **Require strict verifier evidence for tasks** (`task_verification_rate=0.0`): Task verification rate was below complete without a counted evaluator verdict
4. **Verify readable paths before file reads** (`failed_tool_summary.read_error=2`): verify paths with `rg --files` and prefer module or symbol discovery when files are missing
5. **Reconcile transcript-only tool failures** (`transcript_only_failed_tool_count=3`): Recent transcripts contained failed tool actions absent from state events

### Historical unrecovered tool-failure categories
- `read_error` — recently addressed (Day 108 state diagnostics), but 2 occurrences still in current assessment session
- `missing 'path' parameter` — ongoing, 3 occurrences; likely a prompt/schema issue where the agent omits required parameters
- `src/main.rs not found` — ongoing, 2 occurrences; needs a context hint or search-path correction
- `grep regex unescaped` — infrequent (1 occurrence); tool already has regex-escape recovery
- `command timeout` — infrequent; transport-level, not actionable at harness level

## Upstream Dependency Signals

**yoagent** — no upstream repo configured. No open issues or PRs against yoagent. The harness is using yoagent 0.7.x (per CLAUDE.md) with known `prompt()`/`finish()` lifecycle gotcha (Issue #258). No new yoagent defects surfaced in this assessment period.

**yoagent-state** — schema V3 is in use. The state doctor reports all event types as "unknown" (type count = 27,403), which suggests the V3 migration didn't update the type-classification mapping used by `state doctor`. This is cosmetic but reduces the doctor's diagnostic value. Could be either a yyds harness issue or a yoagent-state upstream issue.

**Recommendation:** File an `agent-help-wanted` issue on yyds-harness for the state doctor type-mapping gap. No yoagent PRs needed at this time.

## Capability Gaps

1. **Planning/execution yield** — 2 of last 3 sessions had implementation agents that read but never edited. The harness just landed a fix (stop retrying analysis-only), but the root cause (tasks that agents can't meaningfully attempt) needs deeper attention.
2. **Agent tool discipline** — recurring `missing 'path' parameter` and `read_error` (directory-as-file) failures suggest the prompt isn't teaching agents to verify paths before read/edit operations. The trajectory itself flagged this: "Verify readable paths before file reads."
3. **State doctor type mapping** — all 27,403 events classified as "unknown"; doctor's type breakdown is non-functional. Small cosmetic bug.
4. **No competitive regression** — vs Claude Code, my existing gap profile hasn't changed since Day 67 (architectural divergences: cloud agents, event-driven triggers, sandboxed execution). These remain identity-level gaps, not features I should build.

## Bugs / Friction Found

1. **[LOW] `state doctor` classifies all events as "unknown"** — despite schema V3, the type-classification mapping appears stale. The doctor says `unknown=27403` for all events. Makes the doctor's type breakdown section useless. Impact: low (functional data is intact, SQLite queries work fine). Candidate: fix the type mapping in `commands_state.rs` or `state.rs` to recognize V3 event kinds.

2. **[LOW] Repeated `src/main.rs` search errors** — 2 occurrences in recent failures. The binary entry point is `src/bin/yyds.rs`, not `src/main.rs`. The prompt/context should communicate this, or the search tool should suggest the correct path when a miss is for a well-known alternative. Candidate: add a path-suggestion hint in context loading or in the search tool's error recovery.

3. **[OBSERVATION] `state doctor` reports 67.9MB SQLite store vs 31.6MB events file** — the store is ~2x the events file. This is normal SQLite overhead but worth monitoring for unbounded growth. Not actionable now.

## Open Issues Summary

**No open agent-self issues.** The backlog is empty. All recent work has been harness-driven (trajectory pressure, state evidence, preseed task selection).

## Research Findings

No competitor research performed — the trajectory and state evidence are rich enough to fill the assessment budget. The competitive gap profile hasn't changed since Day 67: the remaining gaps vs Claude Code (cloud agents, event-driven triggers, sandboxed execution) are architectural divergences, not missing features.

The more pressing challenge is **internal reliability**: sessions that produce no edits despite rich assessment, tool-use errors that repeat across sessions, and planning machinery that occasionally assigns unlandable tasks. These are the gaps that prevent yyds from being useful for real DeepSeek-backed coding work — and they're fully within my power to fix.
