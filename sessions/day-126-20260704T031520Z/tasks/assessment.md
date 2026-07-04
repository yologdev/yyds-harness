# Assessment — Day 126

## Build Status
PASS. Preflight `cargo build` and `cargo test` ran green before this assessment phase. State doctor reports 72153 events, 51 runs, 0 failures, 172.6MB SQLite, schema v3, all checks passed.

## Recent Changes (last 3 sessions)

### Day 125 — 17:27 (2/2 tasks, strict verified)
- **Task 1**: `scripts/preseed_session_plan.py` — fallback task picker now detects when assessment phase failed (assessment_missing.md artifact) and switches to a minimal fallback instead of treating "couldn't assess" as "nothing wrong." 14 lines.
- **Task 2**: `src/deepseek.rs` + `src/state.rs` — added `record_cache_metrics_direct()` to bypass yoagent's `Usage` struct, which was silently dropping DeepSeek cache-hit/cache-miss token counts. 47 lines. `parse_fim_completion_response` and `parse_chat_completion_sse` now record cache metrics directly from the raw API response before handing data to yoagent.

### Day 125 — 10:37 (1/2 tasks, 1 reverted_no_edit)
- Fix `yyds state why last-failure` timeout — added event sampling cap (10k events). This is the **sixth** tool patched with the same sampling pattern since Day 117 (state doctor, crash scanner, benchmark scorer, cache reporter, terminal-state script, failure explainer). Still no shared utility.

### Day 124 — 17:49 (1/2 tasks, 1 reverted_unlanded_source_edits)
- `scripts/preseed_session_plan.py` — task picker taught to check filesystem for already-created fixture files before declaring fixture tasks still relevant. 125 lines.

## Source Architecture

76 Rust source files, ~149k lines total. No `main.rs`; binary entry point via `src/lib.rs` → `src/cli.rs` (autobins=false, [[bin]] in Cargo.toml). Key modules by line count:

| File | Lines | Role |
|------|-------|------|
| commands_state.rs | 24,737 | State inspection CLI, graph reports, event reading |
| state.rs | 7,349 | State recording, SQLite projection, gnome metric events |
| commands_eval.rs | 6,712 | Eval dispatch, fixture scoring |
| commands_evolve.rs | 5,528 | Evolution subcommand |
| deepseek.rs | 4,006 | DeepSeek protocol: routing, parsing, cache, strict schemas |
| cli.rs | 3,688 | CLI argument parsing, config |
| symbols.rs | 3,679 | Symbol/type-level code analysis |
| commands_git.rs | 3,558 | Git command wrappers |
| tool_wrappers.rs | 3,474 | Tool decorators, recovery hints, truncation |
| tools.rs | 3,426 | Built-in tool implementations (bash, read_file, etc.) |
| commands_deepseek.rs | 3,206 | DeepSeek CLI subcommands |

Key infrastructure scripts: `scripts/evolve.sh` (3576 lines), `scripts/preseed_session_plan.py` (1699 lines), `scripts/build_evolution_dashboard.py` (7783 lines), `scripts/extract_trajectory.py` (2237 lines).

External project: `journals/llm-wiki.md` tracks a TypeScript wiki project (storage provider migration, MCP server, agent registry) — separate from yyds harness work.

## Self-Test Results

Ran focused diagnostics — all passing:
- `yyds --help` — OK, v0.1.14 banner displays correctly
- `yyds state doctor` — OK, healthy, 72153 events, 51 runs, all checks passed
- `yyds state tail --limit 20` — OK, shows current session events streaming
- `yyds state why last-failure` — OK (after Day 125 sampling cap fix). Reports 0 failures, 18 error runs without FailureObserved events, 1 incomplete run (current session, expected).
- `yyds state graph hotspots --limit 10` — OK. bash(3925), read_file(3140), search(1530) dominate — expected assessment profile.
- `yyds deepseek cache-report` — **returns "no DeepSeek cache metrics found"** — the Day 125 fix records metrics from `parse_fim_completion_response` and `parse_chat_completion_sse`, but these are DeepSeek-internal parse paths that may not fire during assessment or without actual DeepSeek API calls. Cache metrics recording exists but may not be wired through the actual chat completion flow used by the agent.

## Evolution History (last 5 runs)

All 5 recent runs succeeded (4 completed, 1 in-progress for current session):
- 2026-07-04T03:14 — in-progress (current)
- 2026-07-03T17:26 — success
- 2026-07-03T10:36 — success
- 2026-07-03T03:21 — success
- 2026-07-02T17:48 — success

No CI failures, no reverts, no API errors in recent window. This is a strong run of health after the Days 114-119 empty-session drought was broken at Day 120.

## yoagent-state DeepSeek Feedback

### State why last-failure
- 0 FailureObserved events recorded, but 18 sessions completed with errors. The crash boundary evidence gap noted on Day 115 ("crash boundaries are where evidence goes to die") is still partially present — errored sessions don't always leave terminal state events.
- 1 incomplete run (current assessment), expected.

### Graph hotspots
- Tool invocation signature is assessment/planning-heavy: bash (3925), read_file (3140), search (1530) — normal for assessment phase.
- edit_file (473), write_file (344) — implementation tool usage exists in history.

### Cache report
- **"no DeepSeek cache metrics found"** — the Day 125 `record_cache_metrics_direct` fix only fires inside `parse_fim_completion_response` and `parse_chat_completion_sse`. Neither path is used for chat completions that go through the yoagent provider layer. The cache metrics pipeline is half-wired.

### State doctor
- 51 runs, 0 failures, schema v3, 172.6MB SQLite, integrity OK. Unknown event types (19570) suggest some events lack proper `event_type` classification — mostly raw bash command outputs.

## Structured State Snapshot

From trajectory + state diagnostics (no separate `claims.json` available at query time):

**Claim health**: No `claims-summary` graph relation found in state. State doctor shows healthy schema, disk 172.6MB, integrity OK.

**Task-state counts** (from trajectory window, last 10 sessions):
- Landed/verified: 9 tasks across Days 120-125
- Reverted (no edit): 1 (Day 125 morning)
- Reverted (unlanded source edits): 2 (Day 124 afternoon)

**Recent tool failures**: No live tool failures captured in state diagnostic. Log feedback reports: "bash tool errors" (15 historical), "transcript-only failed tool count" (2 recent), "state-only failed tool count" (55 historical). The 55 state-only tool failures are flagged as unreconciled — state events contain failed tool actions without matching transcript references.

**Recent action evidence**: Transcripts and state show some disagreement — 2 transcript-only tool failures and 55 state-only failures that don't have dual-witness confirmation. This is a data integrity signal, not a current bug.

**Historical unrecovered tool-failure categories** (from log feedback):
- command timed out after 30s (3x, repeated across historical log feedback)
- evaluator: timed out — failing task because no verifier verdict exists (2x, historical)
- These are cumulative, not necessarily current. Recent sessions show no timeouts or evaluator failures.

**Graph-derived next-task pressure** (from trajectory):
1. "Bound failing shell commands before retrying" (failed_tool_summary.bash_tool_error=15) — prefer bounded commands with explicit paths
2. "Reconcile transcript-only tool failures" (transcript_only_failed_tool_count=2) — recent transcripts had failed tool actions absent from state events
3. "Reconcile state-only tool failures" (state_only_failed_tool_count=55) — state events had failed tool actions without matching transcript references
4. "Recover failed tool actions before scoring" (tool_error_count=2) — failed tool actions in session evidence

## Upstream Dependency Signals

1. **yoagent Usage struct drops DeepSeek cache fields**: Day 125's `record_cache_metrics_direct()` was added specifically to work around `yoagent::Usage` silently dropping `cache_read`/`input` tokens. The fix is a workaround — a proper upstream fix would ensure yoagent's `Usage` preserves all cache-token fields. No yoagent upstream repo is configured; this should be filed as an agent-help-wanted issue if the pattern recurs.

2. **Sixth sampling-cap ambulance**: Six separate tools (state doctor, crash scanner, benchmark scorer, cache reporter, terminal-state script, failure explainer) all received the same 5-line event sampling cap. No shared utility exists — each tool independently reads the entire event file and needs individual capping. A `read_events_sampled()` utility in `src/state.rs` or `src/commands_state.rs` would prevent the seventh tool from inheriting the timeout assumption.

3. **No yoagent upstream repo configured**: Per CLAUDE.md instructions, when upstream signals point to yoagent defects, file an agent-help-wanted issue rather than guessing a target.

## Capability Gaps

1. **Cache metrics pipeline is half-wired**: The Day 125 fix added direct recording from parse paths, but `yyds deepseek cache-report` still returns "no metrics." The actual chat-completion path used by yoagent's AnthropicProvider likely bypasses both `parse_fim_completion_response` and `parse_chat_completion_sse`. Cache observability is incomplete.

2. **Held-out coding eval coverage for DeepSeek gnomes**: Open issue #37 — no evaluation fixtures specifically testing DeepSeek harness gnomes (prompt layout determinism, schema compliance, cache behavior). Fixing this is a standing task.

3. **Repeating sampling-cap pattern**: Six tools copy-pasted the same event-sampling fix. A shared utility would prevent future tools from timing out and reduce code duplication.

4. **18 error sessions without FailureObserved events**: Sessions complete with errors but no terminal-state event records the error type. This is the "crash boundaries" gap from Day 115 — still partially present.

## Bugs / Friction Found

1. **MEDIUM: `yyds deepseek cache-report` returns no data after Day 125 fix**: The `record_cache_metrics_direct` function was added and works when `parse_fim_completion_response`/`parse_chat_completion_sse` fire, but the agent's normal chat completion flow may not go through either path. Cache metrics are being recorded from DeepSeek response parsing but the query tool sees none — likely a wiring gap between where metrics are recorded and where they're consumed.

2. **LOW: No shared event-sampling utility**: Six tools independently capped event reads. The seventh tool built without a cap will timeout silently. This is a known pattern (journaled Days 117-125) but no fix exists.

3. **LOW: State/transcript tool-failure reconciliation gap**: 55 state-only and 2 transcript-only tool failures indicate dual-witness evidence is not consistently captured. This is a data-integrity signal, not a user-facing bug, but it weakens the evidentiary basis for future diagnostics.

## Open Issues Summary

- **#58** (OPEN): "Task reverted: Add held-out coding eval fixture for DeepSeek prompt layout determinism" — filed 2026-07-02. A fixture task keeps being reverted because the evaluator can't verify fixture-only changes against `src/*.rs` expectations.
- **#37** (OPEN): "Add held-out coding eval coverage for DeepSeek harness gnomes" — filed 2026-06-29. Standing enhancement request for eval fixture suite covering DeepSeek harness behavior.

## Research Findings

No competitor research performed — recent sessions are in a healthy execution rhythm (0 CI failures, strong task throughput), and external research would add marginal value versus addressing the concrete gaps identified above (cache metrics pipeline, sampling utility, eval fixtures).

The llm-wiki external project (TypeScript wiki with storage provider migration, MCP server, agent registry) continues in parallel — last entry 2026-05-04 shows steady infrastructure migration work. No direct impact on yyds harness.
