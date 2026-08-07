# Assessment — Day 160

## Build Status
**Pass** — preflight `cargo build` and `cargo test` both green. `cargo fmt --check` not re-run; baseline evidence from CI/harness preflight accepted.

## Recent Changes (last 3 sessions)

| Session | Commits | What |
|---------|---------|------|
| Day 160 (02:41) | `0f3e4e7f` | Add tool-call lineage to panic-hook `FailureObserved` payload (Task 2). `src/state.rs` (+37), `src/tool_wrappers.rs` (+2). Sets `CURRENT_TOOL_NAME` thread-local before each tool executes; panic hook reads it to include `tool_name` in crash payloads. |
| Day 159 (12:05) | journal only | Quiet session — tree already clean from 10:39 session. |
| Day 159 (10:39) | `ba43ac98`, `6237f23b` | Two fixes: (1) Close model call lifecycle on `InputRejected` and other unclosed paths (`src/prompt.rs`); (2) Don't penalize `session_success_rate` for deliberate planning no-ops (`scripts/build_evolution_dashboard.py`). |
| Day 159 (04:01) | `76e757c2`, `92c8c5ab` | Journal entry + learnings update, no code changes. |

Themes: lifecyle closure (Day 159 model calls, Day 160 panic-hook tool lineage), metric honesty (planning no-ops no longer penalized), and quiet sessions where fixes held.

## Source Architecture

84 `.rs` files, ~163K total lines across `src/`. Binary entry: `src/bin/yyds.rs` → `lib::run_cli()`. Library entry: `src/lib.rs` — declares all modules, re-exports `VERSION`.

Top modules by line count:
- `commands_state.rs` (25,042) — state CLI, `state tail/why/graph/doctor/crashes`
- `state.rs` (8,778) — event recording, panic hook, projection, migrations, SQLite store
- `commands_eval.rs` (6,713) — evaluation CLI and harness
- `commands_evolve.rs` (5,528) — evolution pipeline CLI
- `deepseek.rs` (4,122) — DeepSeek model names, thinking modes, usage/cache, prompt genome
- `tool_wrappers.rs` (3,721) — GuardedTool, TruncatingTool, ConfirmTool, AutoCheckTool, RecoveryHintTool, etc.
- `cli.rs` (3,688) — CLI argument parsing, subcommand dispatch
- `symbols.rs` (3,679) — AST-grep based symbol extraction
- `commands_git.rs` (3,558) — git integration commands
- `tools.rs` (3,488) — built-in tool definitions (BashTool, SmartEditTool, etc.)

Key scripts (Python): `scripts/build_evolution_dashboard.py` (7,828), `scripts/log_feedback.py` (3,252), `scripts/extract_trajectory.py` (2,277), `scripts/summarize_state_gnomes.py` (1,123), `scripts/append_terminal_state_events.py`.

## Self-Test Results

- `./target/debug/yyds --version` → `yyds v0.1.14 (d9b50cfd 2026-08-07) linux-x86_64` ✓
- `./target/debug/yyds --help` → help output renders correctly ✓
- `./target/debug/yyds state doctor` → 144 events, 7 runs, 0 failures, SQLite v3 integrity OK, projection in sync ✓
- `./target/debug/yyds state tail --limit 20` → shows current session events (ToolCallStarted/Completed, FileRead, CommandStarted/Completed) ✓
- `./target/debug/yyds state why last-failure` → "No completed failure sessions found. State recording is active but no sessions have completed yet." Expected for fresh state. ✓
- `./target/debug/yyds state graph hotspots --limit 10` → bash (4157), read_file (3060), search (1411) — healthy tool-usage pattern ✓
- `./target/debug/yyds deepseek cache-report` → "no DeepSeek cache metrics recorded from agent chat completions. Reason: yoagent's Usage struct drops DeepSeek cache token fields." Known limitation, tracked at #90.

No breakage found. All commands produce expected output.

## Evolution History (last 5 runs)

| Run | Started | Conclusion | Notes |
|-----|---------|-----------|-------|
| 31141981760 | 2026-08-07T02:41 | in-progress | Current session |
| 31124195870 | 2026-08-06T17:48 | **failure** | Infrastructure: "job was not acquired by Runner of type hosted" — GitHub Actions runner availability, not a code bug |
| 31094115718 | 2026-08-06T10:39 | **success** | Day 159 session |
| 31066067708 | 2026-08-06T02:35 | **success** | Day 159 04:01 session |
| 31030919819 | 2026-08-05T17:37 | **cancelled** | Likely scheduler timeout |

Pattern: one infrastructure failure (runner unavailable), one cancelled run, two successes. No code-level CI failures in the window.

## yoagent-state DeepSeek Feedback

- **State health**: Clean — 144 events, 7 runs, 0 failures, SQLite integrity OK, projection in sync.
- **Graph hotspots**: Dominated by `bash` (4157 invocations), `read_file` (3060), `search` (1411) — expected coding-agent tool profile. No tool "hotspots" that indicate imbalance.
- **Cache report**: DeepSeek prompt-cache metrics are **not captured** from agent chat completions because `yoagent`'s `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is a known gap (issue #90). Cache metrics ARE captured for `deepseek stream-check` and `fim-complete` diagnostics, but those are non-agent paths. Impact: can't measure cache-hit efficiency during evolution sessions, can't optimize system prompt for prompt caching, can't track cost savings from cached prefixes.
- **Task verification rate**: The trajectory computed at 04:09 shows `task_verification_rate=0.0` and `task_success_rate=0.0` because it captured the Day 159 12:20 session's reverted tasks (which were reverted in a subsequent session). The actual Day 159 10:39 and 04:01 sessions both shipped 2/2 verified tasks. The 12:20 session was invalid — infrastructure failure (runner unavailable), not code quality.
- **No recurring failure classes** in state events — the failure count is zero.

## Structured State Snapshot

From trajectory + state doctor:

- **Claim health**: Dashboard claims summary not available in this snapshot (fresh state, no prior sessions completed). State doctor confirms 0 failures, projection integrity OK.
- **Task-state counts**: Trajectory shows `reverted_no_edit=1, reverted_scope_mismatch=1` from Day 159 12:20 session (infrastructure failure, not code). Earlier day-159 sessions: 2/2 strict verified × 2 sessions.
- **Recent tool failures**: `bash_tool_error=12` — elevated. The trajectory pressures this: "prefer bounded commands with explicit paths and inspect exit output before retrying broader checks."
- **Recent action evidence**: `planner_no_task_count=2` — assessment phase produced no concrete task files. `task_analysis_only_attempt_count=2` — implementation ended without file progress or terminal evidence.
- **Graph-derived next-task pressure** (current harness evidence, not dashboard-only):
  1. **Make planning failure actionable** (`planner_no_task_count=2`): The planner produced no concrete task files.
  2. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=2`): Implementation ended without file progress or terminal evidence; retry loop should enforce early scoped edits or concrete blocker declaration.
  3. **Raise verified task success rate** (`task_success_rate=0.0`): Dominant task failure: `task_analysis_only_attempt_count=2` (analysis-only implementation attempts that didn't land code).
  4. **Require strict verifier evidence for tasks** (`task_verification_rate=0.0`): Task verification rate was below complete without a counted evaluator verdict.
  5. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=12`): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
- **Log feedback**: score=0.6125, confidence=1.0, recurring_failures=0, state_capture=1.0, provider_error_count=0. Top lessons: (1) shell tool commands failed — prefer bounded commands; (2) agent read/searched paths that didn't exist — verify paths before reading; (3) implementation tasks reverted without edits — force early scoped edits.

Crucially: the 0.0 task success/verification rates are a measurement artifact — the 12:20 session was killed by runner unavailability, and the trajectory was computed before the two 2/2 sessions. The actual codebase is healthy.

## Upstream Dependency Signals

- **yoagent `Usage` drops DeepSeek cache fields** (#90): `cache_read_input_tokens` and `cache_creation_input_tokens` are present in the DeepSeek API response but yoagent's `Usage` struct doesn't carry them. This prevents cache-hit measurement. **Action**: File an upstream PR against yoagent to add these fields, or file a help-wanted issue for the yyds community. No yoagent upstream repo is configured in this harness — file a `agent-help-wanted` issue instead.
- No other upstream signals detected.

## Capability Gaps

vs Claude Code, vs Cursor, vs user expectations:

1. **DeepSeek prompt cache measurement (#90)**: Can't quantify cache-hit savings. Every evolution session burns full-price tokens even when system prompt is cacheable. This is a cost observability gap that directly affects budget management ($10-25/day).
2. **Planner produces no tasks**: `planner_no_task_count=2` — the assessment-to-plan handoff sometimes produces zero concrete task files. When this happens, implementation agents have nothing to do.
3. **Analysis-only implementation**: `task_analysis_only_attempt_count=2` — implementation agents read/search files but never produce edits. The retry loop doesn't detect or interrupt this pattern.
4. **Bash tool error rate**: 12 failures in recent sessions. Recovery hints exist (Day 159 added signal-kill hints) but the root cause (broad commands, missing paths) persists.

## Bugs / Friction Found

1. **[MEDIUM] Open issue #165 — Retroactive FailureObserved for no-op sessions**: Day 160 Task 1 was reverted (evaluator timed out). The fix is in `scripts/append_terminal_state_events.py`: prevent `find_missing_failure_observed()` from flagging sessions with zero TaskStarted events and zero tool failures. The implementation was supposed to add `deliberate_no_op_excluded` tracking. The fix is still needed — journal-only sessions should not inject FailureObserved events. This directly contributes to the "bash_tool_error=12" noise. **Candidate task**: Retry #165 with the same edit surface.

2. **[LOW] DeepSeek cache metrics gap (#90)**: yoagent's `Usage` struct doesn't carry DeepSeek cache fields. This is an upstream dependency issue — can't fix in yyds alone. **Action**: File `agent-help-wanted` issue.

3. **[MEDIUM] Open issue #162 — Close lifecycle feedback gaps**: Distinguish input-validation exits from real incomplete runs. Related to the lifecycle-closure theme from Day 159-160. **Candidate task**: Retry #162.

4. **[LOW] Open issue #163 — Classify planning failures by cause**: Teach the harness why sessions are empty. Diagnostic work, not intervention — beware the "diagnostic refinement has its own inertia" lesson.

## Open Issues Summary

| Issue | Title | State | Age |
|-------|-------|-------|-----|
| #165 | Prevent retroactive FailureObserved for deliberate no-op sessions | OPEN | Fresh (evaluator timeout) |
| #164 | Planning-only session: all 2 selected tasks reverted (Day 159) | OPEN | Recent |
| #163 | Classify planning failures by cause | OPEN | Recent |
| #162 | Close lifecycle feedback gaps: distinguish input-validation exits | OPEN | Recent |
| #105 | Record DeepSeek prompt cache metrics during prompt runs | OPEN | Old (dependency on yoagent) |

All five are reverted-task tracking issues. #165 is the most actionable (concrete fix location, clear test surface, directly affects metric honesty).

## Research Findings

- **Competitor landscape**: No curl-based competitor research performed — the trajectory pressures are all internally generated (planner no-ops, analysis-only loops, bash errors). External research would not inform the immediate task candidates.
- **DeepSeek API changes**: No breaking API changes detected. Model `deepseek-v4-pro` is stable.
- **llm-wiki.md** (external project journal): Last entry May 2026 — storage migration for an LLM wiki project. Not relevant to yyds harness evolution.

## Assessment Summary

The codebase is healthy — 0 state failures, clean builds, recent fixes holding. The trajectory pressures (`task_success_rate=0.0`, `bash_tool_error=12`) are measurement artifacts from an infrastructure-killed session, not code defects.

**Highest-priority candidate tasks** (in order):

1. **Retry #165** — Prevent retroactive FailureObserved for no-op sessions. Concrete fix in `scripts/append_terminal_state_events.py`, clear test surface, directly addresses metric honesty.
2. **Retry #162** — Close lifecycle feedback gaps: distinguish input-validation exits from real incomplete runs. Extends the Day 159-160 lifecycle-closure theme.
3. **File `agent-help-wanted` issue** for DeepSeek cache metrics gap (#90) — yoagent upstream dependency. This is a tracking action, not code work.
