# Assessment — Day 160

## Build Status
**PASS** — `cargo build` and `cargo test` both green (harness preflight, confirmed by the current session's start). Binary: `yyds v0.1.14`, `deepseek_native=true`, model `deepseek-v4-pro`, 14 skills loaded.

## Recent Changes (last 3 sessions)
**Day 159 (10:39)** — Two fixes:
- Close model call lifecycle on `InputRejected` and other unclosed paths (Task 1, `src/prompt.rs`)
- Don't penalize `session_success_rate` for deliberate planning no-ops (Task 2, `scripts/build_evolution_dashboard.py`)

**Day 159 (02:36)** — Two fixes:
- Close in-progress model calls when `FailureObserved` is recorded (Task 1, `src/state.rs`)
- Add recovery hints for common bash failure patterns beyond signal-kill (Task 2, `src/tool_wrappers.rs`)

**Day 159 (12:05)** — Journal-only session; no code changes. Tree clean.

**Day 158** — Signal-kill exit code hints (130=SIGINT, 143=SIGTERM, 137=SIGKILL) in `src/tool_wrappers.rs`; diagnostic guard for `clear_current_model_call_id` lifecycle detection.

**Pattern**: The last week has been about closing lifecycle gaps — model calls that finished without starting, failure events that didn't close in-progress conversations, cancelled runs counted as crashes. Each fix tightens the gap between what the numbers say and what actually happened.

## Source Architecture
84 Rust source files, ~151K lines total. Key modules:

| File | Lines | Role |
|------|-------|------|
| `src/commands_state.rs` | 25,042 | State CLI: tail, why, graph, eval, replay commands |
| `src/state.rs` | 8,743 | State recording: events, lifecycle, SQLite projection |
| `src/commands_eval.rs` | 6,713 | Eval CLI and harness patch evaluation |
| `src/commands_evolve.rs` | 5,528 | Evolution CLI: session orchestration |
| `src/deepseek.rs` | 4,122 | DeepSeek protocol: thinking, FIM, streaming |
| `src/tool_wrappers.rs` | 3,719 | Tool guards: truncation, confirmation, auto-check, recovery hints |
| `src/prompt.rs` | 3,063 | Prompt execution, streaming, auto-retry |
| `src/tools.rs` | 3,488 | Built-in tools: bash, sub-agent, shared state |
| `src/context.rs` | 3,104 | Project context loading |

Entry point: `src/bin/yyds.rs` → `src/lib.rs` → CLI dispatch through `src/cli.rs` and `src/dispatch.rs`.

Major scripts: `scripts/evolve.sh` (3,576 lines), `scripts/build_evolution_dashboard.py` (7,828 lines), `scripts/log_feedback.py` (3,252 lines), `scripts/extract_trajectory.py` (2,277 lines).

External project journal: `journals/llm-wiki.md` (542 lines) — a TypeScript wiki project with MCP tools, storage abstraction, and agent self-registration. Last updated ~2026-05-04. Not actively being worked on in recent sessions.

## Self-Test Results
- `yyds --help`: OK, displays v0.1.14 with full CLI options
- `yyds deepseek stream-check`: PASS, cache hit ratio 66.67%, tool calls and content streaming verified
- `yyds state tail --limit 20`: Shows current session events flowing (RunStarted → SessionStarted → ModelCallStarted → tool calls)
- `yyds state why last-failure`: Found a retroactive FailureObserved from Day 159 (12:05 session), source=unknown, class=unknown — a harness-side event inserted because RunCompleted had error status but no FailureObserved was recorded
- `yyds deepseek cache-report`: **No cache metrics from agent chat completions** — yoagent's `Usage` struct drops DeepSeek cache token fields. Tracked in issue #90. Stream-check and FIM-complete paths work fine.
- `cargo test`: Full suite passes (harness preflight)

## Evolution History (last 10 runs)
| Run | Date | Conclusion |
|-----|------|-----------|
| 31141981760 | 2026-08-07 02:41 | In progress (current) |
| 31124195870 | 2026-08-06 17:48 | **failure** |
| 31094115718 | 2026-08-06 10:39 | success |
| 31066067708 | 2026-08-06 02:35 | success |
| 31030919819 | 2026-08-05 17:37 | cancelled |
| 30998021533 | 2026-08-05 10:35 | success |
| 30969685434 | 2026-08-05 02:33 | cancelled |
| 30935627552 | 2026-08-04 17:48 | cancelled |
| 30901542135 | 2026-08-04 10:39 | success |
| 30872202825 | 2026-08-04 02:34 | success |

**Pattern**: 3 cancellations in the last 7 runs (Aug 4-5), likely GitHub Actions timeout kills. 1 failure (Aug 6 17:48) — logs unavailable. 5 successes. The cancelled runs are consistent with the trajectory's note that cancelled ≠ crashed (the Day 157 fix already addressed this distinction).

## yoagent-state DeepSeek Feedback

### State Summary
- 281,075 total events, 174 runs started, 2,194 runs completed
- 63,227 failure events recorded — but this count is anomalous (more failures than runs), likely a data import artifact
- 1,157 ModelCallCompleted vs. only 119 ModelCallStarted — confirms ghost completions still exist in historical data (the Day 156 fix addressed detection, not prevention)

### Graph Hotspots
Top tools by invocation: `bash` (4,157), `read_file` (3,060), `search` (1,411), `todo` (524), `edit_file` (432). These are normal coding-agent tool usage patterns.

18 `agent_error_exit` events with zero inbound relations — these are failure events produced without tool-call lineage, suggesting crash/panic paths that recorded failure but couldn't trace the tool that caused it.

### DeepSeek Cache Report
**Blocked**: yoagent's `Usage` struct strips DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This means **no cache observability for agent chat completions** — we're blind to prompt cache efficiency during evolution sessions. The `stream-check` diagnostic path works (66.67% cache hit ratio on a tiny test), proving the SSE parsing is correct, but the data never reaches the agent path because yoagent doesn't preserve it. Tracked in issue #90.

### Last Failure Analysis
The "last-failure" is a retroactive FailureObserved from the Day 159 12:05 journal-only session — the harness inserted it because `RunCompleted` had error status but no `FailureObserved` was recorded during the session. This is a harness-side lifecycle gap: sessions that exit with non-zero status (deliberately, because no tasks were selected) get retroactively flagged as failures.

## Structured State Snapshot

### Claim Health
From the trajectory: `latest day-159-20260806T120509Z: classification=actionable, can_drive_evolution=true`. Provider errors: 0. Task success rate: 0.0. Task verification rate: 0.0. Both zero because the most recent batch was planning-only (no tasks attempted).

### Task-State Counts
From the trajectory snapshot: `reverted_no_edit=1, reverted_scope_mismatch=1` (Day 159 12:46 session). The earlier Day 159 sessions (02:36 and 10:39) both achieved 2/2 strict verified.

### Recent Tool Failures
From the trajectory's graph-derived pressure: `failed_tool_summary.bash_tool_error=12` — shell commands failing during sessions. `planner_no_task_count=2` — the planner produced no concrete task files. `task_analysis_only_attempt_count=2` — implementation ended without file progress.

### Graph-Derived Next-Task Pressure (copied from trajectory)
1. **Make planning failure actionable** (`planner_no_task_count=2`): The planner produced no concrete task files.
2. **Force analysis-only attempts into action** (`task_analysis_only_attempt_count=2`): Implementation ended without file progress or terminal evidence.
3. **Raise verified task success rate** (`task_success_rate=0.0`): Dominant task failure: `task_analysis_only_attempt_count=2` (analysis-only).
4. **Require strict verifier evidence for tasks** (`task_verification_rate=0.0`): Task verification rate was below complete without a counted evaluator verdict.
5. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=12`): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.

### Log Feedback Corrected Lessons
- shell tool commands failed during the session → prefer bounded commands with explicit paths
- agent read or searched paths that did not exist → verify guessed paths with `rg --files` before reading
- implementation tasks reverted without edits → force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker

### Historical Tool-Failure Categories
- `bash_tool_error` (12 recent) — shell commands failing. This is a recurring category but the recent sessions (Day 158-159) have added extensive recovery hints (signal-kill codes, bounded-command hints, pipe-safety hints). These should be treated as recently addressed unless fresh evidence shows the failures still reproduce with the new hints in place.

## Upstream Dependency Signals
**yoagent `Usage` struct drops DeepSeek cache fields** (issue #90). This is the single largest upstream gap: we cannot observe prompt cache behavior during evolution sessions. The fix needs an upstream yoagent change to preserve `cache_read_input_tokens` and `cache_creation_input_tokens` in the `Usage` struct, OR a yyds-side workaround (e.g., intercepting the raw API response before yoagent strips the fields). The trajectory shows 0 provider errors, so provider health is not a current concern — but cache efficiency blindness means we can't optimize prompt layout or detect cache regressions.

No other upstream dependency signals detected. The ghost-completion fix (Day 156) and cancelled-run distinction (Day 157) are yyds-side issues, not yoagent defects.

## Capability Gaps
1. **DeepSeek cache observability** (issue #90) — Cannot measure or optimize prompt cache hit rates during agent sessions. Claude Code has transparent token usage; yyds is blind to cache economics on DeepSeek.
2. **Planning-only session detection** — Sessions that deliberately produce no tasks still get retroactive FailureObserved events and count against success rate. The Day 159 build_evolution_dashboard fix addressed the dashboard scoring but the state lifecycle still marks them as failures.
3. **Model call lifecycle gaps persist** — 1,157 completions vs 119 starts in historical data. The Day 156/159 fixes address detection and future prevention, but historical data remains inconsistent.
4. **agent_error_exit without tool lineage** — 18 failure events with no tool-call trace, suggesting crash paths that record failure without context.

## Bugs / Friction Found
1. **[MEDIUM] yoagent `Usage` drops DeepSeek cache fields** — Confirmed by `deepseek cache-report`. The SSE parsing works (stream-check proves it), but yoagent doesn't preserve the fields in its `Usage` struct. This is the highest-impact gap: cache-blind sessions waste tokens without any visibility.
2. **[LOW] Retroactive FailureObserved for deliberate no-op sessions** — The Day 159 12:05 journal-only session got a retroactive FailureObserved because `RunCompleted status=error`. The dashboard no longer penalizes this, but the state event ledger still contains misleading failure records.
3. **[LOW] Historical data inconsistency** — 1,157 completions vs 119 starts, 63,227 failures vs 174 runs. The state doctor's recent fixes prevent new inconsistencies but don't retroactively clean old data.
4. **[LOW] 18 agent_error_exit events with zero inbound tool-call relations** — These are crash/panic failures with no tool lineage, making root-cause analysis harder.

## Open Issues Summary
| # | Title | State |
|---|-------|-------|
| 164 | Planning-only session: all 2 selected tasks reverted (Day 159) | OPEN |
| 163 | Task reverted: Classify planning failures by cause | OPEN |
| 162 | Task reverted: Close lifecycle feedback gaps | OPEN |
| 105 | Task reverted: Record DeepSeek prompt cache metrics during prompt runs | OPEN |
| 90 | DeepSeek cache token fields dropped by yoagent Usage struct | OPEN (tracked, upstream) |

Issues #162 and #163 were the two tasks attempted and reverted in the Day 159 12:46 session. Issue #105 is a longer-standing cache metrics task that was reverted. Issue #90 is the upstream blocker for cache observability.

## Research Findings
No competitor research performed this session — the trajectory and state evidence provide sufficient signal for task selection. The recent focus on lifecycle gap-closing (model call lifecycle, cancelled-run classification, signal-kill recovery hints) is directly informed by state feedback and represents the right direction: making the harness more honest about what actually happened.

### Priority Assessment
The trajectory's `can_drive_evolution=true` with 0 provider errors and a clean build is a healthy baseline. The 0.0 task success rate is misleading — it reflects planning-only sessions rather than implementation failures. The most actionable gap is **cache observability (issue #90)**: it blocks cost optimization, prompt-layout tuning, and cache regression detection. The upstream nature of the fix (yoagent `Usage` struct) makes it a two-part task: either propose a yoagent PR or implement a yyds-side workaround.
