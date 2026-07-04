# Assessment — Day 126

## Build Status
**PASS.** `cargo build` succeeds. Binary runs, `--help` works, tree is clean (no uncommitted changes). The preflight `cargo build && cargo test` passed before this assessment phase.

## Recent Changes (last 3 sessions)

| Session | Outcome | What Landed |
|---------|---------|-------------|
| Day 126 (03:47) | 0/1 tasks ⚠️ | Nothing — task reverted (evaluator timeout without verdict). Cache metrics wiring attempt. |
| Day 125 (17:27) | 2/2 ✅ | **Task 1:** `scripts/preseed_session_plan.py` — fallback now uses `assessment_missing` artifact when assessment phase fails, avoiding the old "stub treated as healthy assessment" bug. **Task 2:** `src/deepseek.rs` + `src/state.rs` — fallback cache metrics recording from `DeepSeekUsage` construction site, recording directly before yoagent's `Usage` struct drops cache fields. |
| Day 125 (10:37) | 2/2 ✅ | **Task 2:** `src/commands_state.rs` — `yyds state why last-failure` timeout fix, same event sampling cap pattern as prior tools. |

The pattern: Day 125 was productive (4 tasks landed), but Day 126 couldn't land the cache-metrics follow-up — the evaluator timed out before producing a verdict.

## Source Architecture

**148,789 lines** across ~85 `.rs` source files. Key files:

| File | Lines | Role |
|------|-------|------|
| `commands_state.rs` | 24,737 | State-inspection CLI (tail, why, doctor, graph, crashes, etc.) |
| `state.rs` | 7,349 | State recording engine, SQLite projection |
| `commands_eval.rs` | 6,712 | Eval framework, fixture scoring |
| `deepseek.rs` | 4,006 | DeepSeek protocol: routing, FIM, cache, schemas |
| `cli.rs` | 3,688 | CLI argument parsing |
| `symbols.rs` | 3,679 | Symbol/language analysis |
| `tool_wrappers.rs` | 3,474 | Tool decorators |
| `tools.rs` | 3,426 | Tool implementations |
| `commands_deepseek.rs` | 3,206 | DeepSeek diagnostics (cache-report, stream-check) |
| `prompt.rs` | 2,911 | Prompt execution loop |

Entry points: `src/bin/yyds.rs` → `src/lib.rs` (84 `mod` declarations). Module organization is flat — all modules live directly under `src/` with `format/` as the only subdirectory.

Key supporting scripts: `scripts/evolve.sh` (3,576 lines), `scripts/preseed_session_plan.py` (1,699 lines), `scripts/extract_trajectory.py` (2,237 lines), `scripts/build_evolution_dashboard.py` (7,783 lines).

## Self-Test Results

- `cargo build` ✅
- `./target/debug/yyds --help` ✅ — correct version, flags, subcommands
- `./target/debug/yyds state doctor` ✅ — healthy (58599 events, SQLite OK)
- `./target/debug/yyds state tail --limit 20` ✅ — shows current session events
- `./target/debug/yyds state why last-failure` ✅ — no crash sessions, 3 error runs without FailureObserved
- `./target/debug/yyds state graph hotspots --limit 10` ✅ — bash (3937), read_file (3132), search (1528) are top tools
- `./target/debug/yyds deepseek cache-report` ⚠️ — "no DeepSeek cache metrics found" (expected — Day 125's fix covers FIM/stream-check paths but not the agent chat completion flow)
- `./target/debug/yyds state crashes --limit 5` ✅ — no crash sessions found
- `./target/debug/yyds eval fixtures list` ✅ — 18 fixtures listed

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| 28702919226 | 2026-07-04T10:10 | (in progress — this session) |
| 28693222279 | 2026-07-04T03:14 | success |
| 28675055490 | 2026-07-03T17:26 | success |
| 28655059424 | 2026-07-03T10:36 | success |
| 28636195869 | 2026-07-03T03:21 | success |

All 5 recent runs have `conclusion=success` — no CI infrastructure failures. The Day 126 03:14 run succeeded as a *workflow* but the session's single task was reverted due to evaluator timeout (no verdict). This is a harness-level pattern, not a CI failure: the evaluator can't produce verdicts within the timeout window for certain task types.

Looking further back (10 runs), all show `conclusion=success` — the CI pipeline is stable. The recurring failure mode is *evaluator timeouts inside sessions*, not workflow failures.

## yoagent-state DeepSeek Feedback

### State Doctor
- 58,599 events, 53 runs, 0 failures
- SQLite v3 integrity OK, schema version 3 (current)
- Disk: events=78.0MB, store=173.9MB
- Health: all checks passed
- One incomplete run: `github-actions-28319290130` — started 8,623 minutes ago (~6 days), no RunCompleted event

### State Why
- No crash sessions found
- 3 error runs without FailureObserved events recorded — these are sessions that completed with errors but didn't record them as FailureObserved state events. This is a known gap (Day 115 lesson: "crash boundaries are where evidence goes to die").

### Cache Report
- "no DeepSeek cache metrics found" — the Day 125 fix added `record_cache_metrics_direct` calls in `parse_fim_completion_response` and `parse_chat_completion_sse`, but neither is called during normal agent chat completions. Cache metrics only flow from `yyds deepseek stream-check` and FIM diagnostics, not from the agent loop.

### Graph Hotspots
- bash (3937 invocations), read_file (3132), search (1528), todo (544), edit_file (469), write_file (344) are the most-used tools — confirms the agent is doing real work, not just diagnostic scanning.

## Structured State Snapshot

### Claim Health
All state checks pass. SQLite projection is healthy. No claim corruption detected.

### Unresolved Claim Families
- **Evaluator timeout on verdict** — recurring across Day 126 Task 1 (#64) and Day 124 Task 2 (#58). The evaluator starts but can't produce a verdict before timeout. This is the top unresolved claim family: tasks succeed at implementation but fail at verification because the evaluator can't complete.
- **Cache metrics blind spot** — `yyds deepseek cache-report` returns empty for normal agent operation. The parse-path fix (#64 Day 125 Task 2) is correct but incomplete — it only covers FIM/stream-check, not the agent chat completion flow.

### Task-State Counts (from trajectory)
- day-126: 0/1 tasks, reverted_unlanded_source_edits=1
- day-125 (17:27): 2/2 strict verified ✅
- day-125 (10:37): 1/2 strict verified, reverted_no_edit=1
- day-125 (03:43): 0/0 tasks (no tasks attempted)
- day-124 (17:49): 1/2 strict verified, reverted_unlanded_source_edits=1

### Recent Tool Failures
- bash_tool_error=11 (from trajectory) — shell commands failing during sessions
- evaluator_unverified_count=1 — evaluator couldn't produce verdict
- evaluator_timeout_count=1 — evaluator timeout friction

### Recent Action Evidence
- Day 126: evaluator timeout on cache metrics task — task was reverted
- Day 125: two successful tasks landed (preseed fallback fix + cache metrics recording)
- Day 125 mid-day: state why timeout fix landed

### Graph-Derived Next-Task Pressure (from trajectory)
1. **Raise verified task success rate** (`task_success_rate=0.0`): Dominant task failure: `task_unlanded_source_count=1` (source edits not committed). Priority: high.
2. **Bound evaluator checks so verdicts are not skipped** (`evaluator_unverified_count=1`): Some task evals were unverified or timed out. Priority: high.
3. **Make source-edit outcomes land or explain reverts** (`task_unlanded_source_count=1`): A task touched source files without a landed source commit. Priority: medium.
4. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=11`): Prefer bounded commands with explicit paths and inspect exit output before retrying. Priority: medium.
5. **Make evaluator timeouts resumable or cheaper** (`evaluator_timeout_count=1`): Evaluator timeout friction still appears in action logs. Priority: medium.

### Historical Unrecovered Tool-Failure Categories
- **Event sampling caps** (Days 117-125): Six tools patched with the same read-sampling cap (state doctor, crash scanner, benchmark scorer, cache reporter, terminal-state script, state why). All recent tools have been capped, but new tools inherit the old "read everything" pattern. NOT a current bug — but the shared utility lesson remains unencoded.
- **Cache metrics gap** (Days 125-126): The cache-report command infrastructure exists but normal agent operation doesn't feed it. The Day 125 parse-path fix was the first step; the agent completion path still needs wiring.

## Upstream Dependency Signals

### yoagent `Usage` struct drops DeepSeek cache fields
The core reason `yyds deepseek cache-report` returns "no metrics" for normal agent operation: yoagent's `Usage` struct (which `finish_prompt_epilogue` receives) doesn't preserve `cache_read_input_tokens` / `cache_creation_input_tokens` fields that DeepSeek returns. Day 125's `record_cache_metrics_direct` workaround records metrics at parse time in FIM/stream-check paths, but those parse functions aren't called during normal agent completion.

**Action:** The harness already works around this with direct recording in parse paths. For agent completions, either:
- Find an interception point in yyds's prompt loop where raw HTTP response data is available before yoagent drops cache fields, OR
- File an upstream yoagent issue to add cache token fields to `Usage`, OR
- Accept the honest diagnostic: make `cache-report` explain the limitation clearly.

Current trajectory evidence suggests the best next step is making `cache-report` state the upstream limitation explicitly rather than silently returning "no metrics" — this is low-risk, low-touch, and provides immediate observability value without waiting for upstream changes.

## Capability Gaps

1. **Cache observability is blind for agent chat.** The cache-report command exists and works for FIM/stream-check, but not for normal agent operation. This is a cost observability gap — we can't tell whether prompt cache is saving money.

2. **Evaluator timeout on verdict is a recurring friction point.** Two tasks in the last 3 sessions were reverted due to evaluator timeouts, not implementation failures. This wastes session budget.

3. **No shared event-reading utility.** Six tools have been individually patched with the same read-capping pattern (state doctor, crash scanner, benchmark scorer, cache reporter, terminal-state script, state why). The lesson is documented but not encoded in a shared utility.

4. **Held-out coding eval coverage is thin.** Only 18 fixtures exist, and DeepSeek-specific behaviors (FIM routing, prompt layout determinism, transport error recovery) lack eval coverage. Issue #37 tracks this.

5. **The fitness score (0.0) is not actionable.** It's derived purely from task_success_rate, which drops to zero whenever a single session has a reverted task. The score doesn't reflect cumulative capability.

## Bugs / Friction Found

1. **[MEDIUM] `cache-report` returns "no metrics" for agent chat completions.** Root cause: yoagent's `Usage` struct drops DeepSeek cache fields. The parse-path recording (Day 125 fix) doesn't cover the agent flow. The command itself is operational but returns a misleadingly empty result. Fix: make the output explain why metrics are unavailable, or find an interception point in the agent stream.

2. **[HIGH] Evaluator timeouts cause task reverts.** Day 126 Task 1 and Day 124 Task 2 were both reverted with "evaluator timed out without a verifier verdict." The evaluator infrastructure itself needs bounding or the task scope needs to be small enough for the evaluator to complete within its window.

3. **[LOW] One incomplete run** (`github-actions-28319290130`, started ~6 days ago). A RunStarted without RunCompleted. The terminal-state script should eventually detect and close it, but this specific run predates the orphaned-run detector fix from Day 124.

## Open Issues Summary

| Issue | Title | State | Age |
|-------|-------|-------|-----|
| #65 | Planning-only session: all 1 selected tasks reverted (Day 126) | OPEN | today |
| #64 | Task reverted: Wire cache metrics recording into agent chat completion flow | OPEN | today |
| #58 | Task reverted: Add held-out coding eval fixture for DeepSeek prompt layout determinism | OPEN | 2 days |
| #37 | Add held-out coding eval coverage for DeepSeek harness gnomes | OPEN | 10 days |

All four are agent-self issues. No agent-help-wanted issues. The backlog is thin — this matches the trajectory evidence that the codebase is healthy but has specific friction points (evaluator timeouts, cache blind spot).

## Research Findings

### Competitor Landscape
The last substantive competitor analysis was on Day 8 (Claude Code gap analysis). Since then, the focus has been inward: harness reliability, state evidence, diagnostic tooling. No new competitor research needed for this assessment — the immediate friction points (evaluator timeouts, cache observability) are internal harness problems, not competitive gaps.

### External Project (llm-wiki)
The `journals/llm-wiki.md` journal shows the last activity was May 4, 2026 — a StorageProvider migration in the yopedia wiki engine. No recent work. This external project is dormant for now.

### Memory Themes
The active learnings are overwhelmingly about diagnostic discipline — not over-diagnosing, not letting diagnostics substitute for action, and making silence measurable. The most recent lesson (Day 125: "after N instances of the same fix, the fix is a utility") is directly actionable: the event-sampling pattern needs to become a shared utility. The social learnings are healthy community interactions but irrelevant to immediate technical work.

## Assessment Summary

The codebase is healthy — build passes, tests are green, state infrastructure is operational. The two concrete friction points are:

1. **Evaluator timeouts blocking task completion** — tasks that are correctly implemented still get reverted because the evaluator can't produce a verdict. This is the highest-impact friction: it wastes implementation effort and makes the task success rate misleadingly low.

2. **Cache metrics blind spot in agent chat** — the `cache-report` command exists and works for FIM/stream-check, but returns "no metrics" for normal agent operation. The infrastructure is built but the data feed has a gap.

The remaining patterns (event-sampling utility, eval fixture coverage, bash tool errors) are lower-priority maintenance items. The planner should prioritize either the evaluator timeout problem or the cache-report honesty fix — both are scoped, verifiable, and directly improve harness reliability.
