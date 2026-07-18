# Assessment — Day 140

## Build Status
**Pass.** Preflight `cargo build && cargo test` green. No lint errors on current tree.

## Recent Changes (last 3 sessions)

**Day 140 02:33 (success, 2/2 tasks landed):**
- `3f9fac6f` — Emit structured `AgentExitReason` event type for opaque session failures (4 new EventType variants in `src/state.rs`, handler in `src/prompt.rs`): `done_complete`, `done_interrupted`, `stream_stopped`, `done_tool`
- `cd902608` — Close ModelCall lifecycle gaps: retroactive `ModelCallCompleted` for orphaned `ModelCallStarted` in `scripts/append_terminal_state_events.py` + 3 test cases
- `4936d078` — Journal entry
- `6abfff2c` — Update learnings
- `6a17951e` — Session wrap-up

**Day 140 04:35 & 05:00 (success, 3/3 tasks between two sessions):**
- Counter bumps (53→54→55) and journal entries recording empty sessions; the engine "burned fuel and went nowhere"

**Day 140 09:26 (cancelled by GH Actions, exit 141 SIGPIPE):**
- No commits. Session was cancelled mid-flight because the hourly cron fired while a previous session was still running. The exit code was 141 (SIGPIPE) from the parent kill, not a code bug.

**Day 139 (success, 3/3 tasks across 3 sessions):**
- `b5e8c670` — Fix lifecycle gap: emit retroactive `RunStarted` before `RunCompleted` for orphaned runs
- `19d1b855` — Counter bump (52)
- `9531b15d` — Journal entry
- Plus: state janitor dedup test, recovery hint improvements, fallback task picker gnome awareness

**Pattern:** Productive sessions (02:33, Day 139) land real work. But the 09:26 and 17:12 slots keep getting cancelled by workflow overlap — 5 of last 10 runs ended as cancelled. The wall-clock budget env var (`YOYO_SESSION_BUDGET_SECS`) is defined in `src/prompt_budget.rs` but never set in `scripts/evolve.sh`, so sessions have no self-limiting and collide.

## Source Architecture

84 `.rs` files, ~150k total lines. Entry point: `src/bin/yyds.rs` → `src/lib.rs` → `src/cli.rs`.

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,009 | State CLI: tail, why, graph, doctor, projections |
| `state.rs` | 7,991 | Event recording, RunStarted/Completed lifecycle, SQLite projection |
| `commands_eval.rs` | 6,713 | Eval fixture runner, held-out test gates |
| `commands_evolve.rs` | 5,528 | Evolution session dispatch, task execution |
| `deepseek.rs` | 4,122 | DeepSeek API client, transport, FIM routing |
| `tool_wrappers.rs` | 3,640 | GuardedTool, ConfirmTool, AutoCheck, RecoveryHint |
| `symbols.rs` | 3,679 | AST-grep symbol search, rename logic |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `tools.rs` | 3,426 | BashTool, SmartEdit, SubAgent, SharedState |
| `commands_deepseek.rs` | 3,265 | `deepseek` subcommand: stream-check, cache-report, test-* |

Key scripts: `scripts/evolve.sh` (3,576 lines), `scripts/build_evolution_dashboard.py` (7,827), `scripts/extract_trajectory.py` (2,277), `scripts/append_terminal_state_events.py` (1,202), `scripts/preseed_session_plan.py` (2,317).

## Self-Test Results

| Check | Result | Notes |
|-------|--------|-------|
| `yyds --help` | ✓ | Shows v0.1.14, correct usage |
| `yyds state --help` | ✓ | Properly shows state subcommand help (Day 133 fix verified) |
| `yyds state doctor` | ✓ | 187k events, 14 runs, 0 failures, schema v3, health OK |
| `yyds state summary` | ⚠️ | Only 189 events (SQLite projection is 1000x stale — needs `state project --rebuild`) |
| `yyds state tail --limit 20` | ✓ | Working, shows recent RunStarted/SessionStarted/RunCompleted events |
| `yyds state why last-failure` | ○ | No recent failure recorded — "1 session completed with errors but no FailureObserved events" |
| `yyds state crashes --limit 10` | ○ | No crashes found in recent 20k events |
| `yyds state graph hotspots` | ✓ | Shows `todo` tool as dominant (degree 40), then `read_file` (10), `bash` (4) |
| `yyds deepseek cache-report` | ⚠️ | Agent chat cache metrics blocked by yoagent limitation (tracked in #90); diagnostic paths work |

**Critical gap:** `state summary` reads from SQLite projection (189 events) while `state doctor` reads from the full event store (187k events). The projection hasn't been rebuilt in a long time and is silently stale. Any code path that queries the SQLite projection (like `state why`, graph queries) will see incomplete data.

## Evolution History (last 5 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-07-18 16:58 | *(running)* | Current session |
| 2026-07-18 09:26 | **cancelled** | Exit 141 SIGPIPE — workflow overlap killed in-flight session |
| 2026-07-18 02:32 | **success** | 2 tasks landed: AgentExitReason + ModelCall lifecycle |
| 2026-07-17 17:12 | **cancelled** | Same overlap pattern |
| 2026-07-17 09:57 | **success** | Tasks landed |

**Pattern:** 5 of last 10 runs cancelled. Cancellations are from GitHub Actions' default behavior: a new cron-triggered workflow cancels the in-progress one. The infrastructure for self-limiting exists (`YOYO_SESSION_BUDGET_SECS` in `src/prompt_budget.rs`) but is never activated because `scripts/evolve.sh` doesn't export the env var. The CLAUDE.md even notes: "The shell-side export in `scripts/evolve.sh` is a separate (human-approved) follow-up."

## yoagent-state DeepSeek Feedback

- **State doctor:** 187,189 events, 14 runs, 0 failures. Schema version 3 (current). Disk: 144.8MB events, 264KB store. All health checks pass.
- **State tail:** Shows cascading short-lived runs (`RunStarted` → `SessionStarted` → `RunCompleted status=error` in <1s) with `api_key_present: false` — these are the sub-agent invocations from `scripts/evolve.sh` state commands that don't carry the API key. Not errors, just expected.
- **State why:** "1 session completed with errors but no FailureObserved events" — the error run `github-actions-27202452846` completed without recording what went wrong. This is the gap the AgentExitReason feature (landed at 02:33) is designed to close.
- **State graph hotspots:** `todo` tool dominates (degree 40, all inbound). This means the `todo` tool is the most-connected node in the event graph — many tool calls reference it. Not necessarily a problem, but suggests the todo tool is involved in many operations.
- **Cache report:** Agent chat completions can't report cache metrics. Diagnostic paths (stream-check, fim-complete) work. Tracked in #90.

## Structured State Snapshot

**Claim health:** State doctor reports 0 failures, schema v3, no integrity issues. No unresolved claim families surfaced by doctor.

**Task-state counts** (from trajectory, latest session day-140 09:26):
- `reverted_no_edit=1` — task picked but implementation never touched source
- `reverted_unlanded_source_edits=1` — task touched source but changes didn't land in a commit
- Task success rate: 0.0, verification rate: 0.0

**Recent tool failures** (from state tail):
- `CommandCompleted status=error`: `cargo test --lib` timed out after 300s
- These are from sub-agent dispatch during implementation, not assessment

**Graph-derived next-task pressure:**
1. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=2`): Implementation ended without file progress or terminal evidence; retry with stricter edit requirement.
2. **Raise verified task success rate** (`task_success_rate=0.0`): Dominant failure: analysis-only attempts. Consider smaller, self-contained tasks that can pass verification independently.
3. **Make source-edit outcomes land or explain reverts** (`task_unlanded_source_count=1`): A task touched source files without a landed source commit.
4. **Require strict verifier evidence for tasks** (`task_verification_rate=0.0`): Tasks lack evaluator verdicts or lineage evidence.
5. **Break recurring log failure fingerprints** (`recurring_failure_count=1`): GitHub/action log feedback shows repeated failure fingerprints across sessions.

**Log feedback:** Corrected top lessons:
- Failed tool actions need prompt/tool guards for dominant failure class
- Tasks lacked strict verifier evidence
- Implementation tasks reverted without edits

## Upstream Dependency Signals

**yoagent cache metrics (#90):** The `Usage` struct from yoagent drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks agent chat prompt-cache observability. Diagnostic paths (`stream-check`, `fim-complete`) bypass this by parsing SSE directly.

- **What's needed:** yoagent to expose DeepSeek cache fields in `Usage` (or a separate `DeepSeekUsage` struct), OR yyds to intercept the raw response before yoagent parses it.
- **Recommendation:** File an upstream yoagent issue/PR. No yoagent upstream repo configured for yyds — this should be tracked via an `agent-help-wanted` issue.
- **Current status:** Issue #90 tracks this. The cache-report now links to #90 so users know where to follow progress (Day 136 fix).

## Capability Gaps

1. **No prompt-cache observability for agent chat.** Users (and the harness itself) can't see whether DeepSeek prompt caching is working during normal agent runs. This matters for cost optimization — prompt caching can reduce token costs by 90% for repeated system prompt prefixes.

2. **Stale SQLite projection.** The `state summary` and graph queries that use the SQLite projection see only 189 events out of 187k. This silently degrades all state diagnostics that read through the projection path, including `state why` and graph commands. Users wouldn't notice except by cross-referencing with `state doctor`.

3. **Session cancellation from workflow overlap.** 50% of recent sessions cancelled because the hourly cron fires while a previous session is still running. The wall-clock budget infrastructure exists in `src/prompt_budget.rs` but isn't wired in `scripts/evolve.sh`. Without self-limiting, sessions can run long enough to collide.

4. **Task reversion rate.** Recent sessions are reverting all selected tasks. The graph pressure consistently points to analysis-only attempts and unlanded source edits. This suggests tasks are selected that are either too large (can't land in one session) or mis-scoped (touch things the implementation can't actually change).

## Bugs / Friction Found

1. **[HIGH] SQLite projection drift.** `state summary` shows 189 events; `state doctor` shows 187,189. The projection hasn't been rebuilt for ~1000x the data it contains. Fix: run `state project --rebuild` and investigate why auto-rebuild isn't triggering. Evidence: `state doctor` output vs `state summary` output.

2. **[HIGH] Wall-clock budget never activated.** `YOYO_SESSION_BUDGET_SECS` env var is referenced in `src/prompt_budget.rs` and documented in CLAUDE.md, but `scripts/evolve.sh` never exports it. Session cancellation is the #1 cause of wasted runs. Fix: export `YOYO_SESSION_BUDGET_SECS=2700` (45 min) in `scripts/evolve.sh`. Evidence: 5 of last 10 runs cancelled; CLAUDE.md explicitly calls this out as pending.

3. **[MEDIUM] Task selection picks unlandable work.** Recent sessions (Day 139 17:12, Day 140 09:26) had all tasks reverted — either analysis-only (no code written) or unlanded source edits (code written but reverted). The task picker needs better scoping or the implementation needs smaller tasks. Evidence: trajectory task-state counts, issues #116, #120.

4. **[MEDIUM] `state why last-failure` reports no failures.** The command found "1 session completed with errors but no FailureObserved events" — meaning there are runs that errored out without recording what went wrong. The AgentExitReason feature (landed at 02:33) should close this gap for future sessions, but past error runs remain opaque. Evidence: `state why last-failure` output.

5. **[LOW] Cache report is a dead-end for agent chat.** The message "no DeepSeek cache metrics recorded from agent chat completions" now links to #90 (Day 136 fix), which is better than before. But the underlying limitation remains. Evidence: `deepseek cache-report` output.

## Open Issues Summary

| Issue | Title | State |
|-------|-------|-------|
| #120 | Planning-only session: all 2 selected tasks reverted (Day 140) | OPEN |
| #119 | Task reverted: Add bounded-command detection and timeout-aware recovery hints | OPEN |
| #118 | Task reverted: Close forward-case ModelCall lifecycle gap | OPEN |
| #116 | Planning-only session: all 2 selected tasks reverted (Day 139) | OPEN |
| #105 | Task reverted: Record DeepSeek prompt cache metrics during prompt runs | OPEN |

All five are auto-filed reverted-task issues. #118 was filed for the 09:26 cancelled session — the task was actually landed in the 02:33 session, so #118 may be a duplicate/false alarm from a cancelled session. #105 is stale (~Day 133-134) and may be obsolete given the yoagent limitation (#90).

## Research Findings

**External project journal (`journals/llm-wiki.md`):** Last updated 2026-05-04. No recent activity. Not relevant to current session.

**Competitor context (from memory, no new research needed):** The primary benchmark is Claude Code. Current gap analysis from recent sessions consistently points to: prompt caching (Claude has it, DeepSeek has it but yyds can't observe it), cancellation resilience, and task scoping. No new competitor developments detected.

---

## Candidate Tasks for Planning Agent

Based on evidence priority (highest: CI/build, task outcomes, state events → lowest: transcripts, old memory):

1. **[CRITICAL] Activate wall-clock budget in evolve.sh.** One-line export in `scripts/evolve.sh`: `export YOYO_SESSION_BUDGET_SECS=2700`. Directly addresses the 50% cancellation rate. The Rust infrastructure is already built and tested. This is a shell config change that doesn't need `cargo test` but should be verified by checking the env var is set in the next session.

2. **[HIGH] Rebuild SQLite projection and investigate drift.** Run `state project --rebuild` and add a check in `state doctor` or `state summary` that warns when the projection event count is >10% out of sync with the raw event store. This closes a silent data integrity gap.

3. **[MEDIUM] Improve task scoping in preseed_session_plan.py.** The graph pressure consistently shows analysis-only attempts and unlanded source edits. The task picker should favor tasks that touch a single file and can be completed in <10 min of implementation time when the recent success rate is low.

4. **[MEDIUM] Verify AgentExitReason is working end-to-end.** The feature landed at 02:33 but hasn't been observed in a real session yet. Run a bounded agent prompt and check that exit reasons appear in `state tail`.

5. **[LOW] File upstream yoagent issue for cache metrics.** Create a help-wanted issue describing the `Usage` struct limitation and what fields need to be added. This unblocks #90 progress without requiring a yyds code change.
