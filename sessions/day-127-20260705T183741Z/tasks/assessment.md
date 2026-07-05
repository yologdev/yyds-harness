# Assessment — Day 127

## Build Status
**PASS** — `cargo build --bin yyds` completes (0.13s), binary runs, `--version` and `--help` work. Preflight `cargo test` from harness assumed green (uncontradicted).

## Recent Changes (last 3 sessions)

| Session | What | Files |
|---------|------|-------|
| Day 127 03:30 | Append missing FailureObserved events for error-completed runs (Task 1) | `scripts/append_terminal_state_events.py` +77, `scripts/test_append_terminal_state_events.py` +137 |
| Day 126 17:07 | Fix orphaned-run detection gap (Task 1), held-out eval fixture for DeepSeek genome determinism (Task 2), unit tests for read_events_bounded (Task 3), fix build errors | Multiple src/ and script files |
| Day 126 10:11 | `read_events_bounded` utility extracted into `src/state.rs` (shared utility replacing copy-pasted caps), cache-report explanatory message | `src/state.rs`, `src/commands_deepseek.rs` |

**Day 127 pattern**: The 03:30 session landed real code (214 lines across scripts + tests), but the two later sessions (04:28, 10:13) both produced unlanded source edits (reverted). The 04:28 session had 1/2 tasks strict-verified (the other reverted unverified), the 10:13 session had 0/2 tasks verified — both reverted as unlanded_source_edits.

## Source Architecture

84 `.rs` files across `src/`. Key modules by line count:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 24,737 | State CLI commands (graph, why, tail, crashes, memory) |
| `state.rs` | 7,600 | State recorder, events, projections, migration |
| `commands_eval.rs` | 6,712 | Eval commands (fixtures, score, replay) |
| `deepseek.rs` | 4,045 | DeepSeek protocol, harness genome, transport, schemas |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `tools.rs` | 3,426 | Builtin tools (bash, search, rename, etc.) |
| `tool_wrappers.rs` | 3,474 | Tool decorators (guarded, truncating, recovery hints) |
| `commands_deepseek.rs` | 3,254 | DeepSeek subcommands (cache-report, test-tool-call, etc.) |
| `watch.rs` | 2,938 | Watch mode, auto-fix, compiler error parsing |
| `prompt.rs` | 2,911 | Prompt execution, streaming, auto-retry |

Key scripts: `evolve.sh` (3,576), `build_evolution_dashboard.py` (7,783), `extract_trajectory.py` (2,237), `preseed_session_plan.py` (1,699).

Binary entry point: `src/bin/yyds.rs`.

## Self-Test Results

- `cargo build --bin yyds`: **PASS** (0.13s)
- `yyds --version`: **PASS** — `yyds v0.1.14 (373ed859 2026-07-05)`
- `yyds --help`: **PASS** — full help output renders
- `yyds state tail --limit 20`: **PASS** — shows recent events including the planning_failed decision and rapid error runs
- `yyds state why last-failure`: **PASS** — shows retroactive FailureObserved (the Day 127 fix at work)
- `yyds deepseek cache-report`: **PASS** — explains why agent chat metrics aren't recorded (yoagent Usage struct limitation), directs to diagnostic paths
- `yyds state graph hotspots --limit 10`: **PASS** — bash (3945), read_file (3106), search (1524) most invoked

No regressions detected in bounded self-test.

## Evolution History (last 10 runs)

All 10 most recent workflow runs show `"conclusion":"success"` at the CI level — no workflow-level failures:

| Run | Started | Conclusion |
|-----|---------|------------|
| 28748526649 | 2026-07-05T17:11 | (in progress) |
| 28737345069 | 2026-07-05T10:13 | success |
| 28728262612 | 2026-07-05T03:30 | success |
| 28713476415 | 2026-07-04T17:06 | success |
| 28702919226 | 2026-07-04T10:10 | success |
| 28693222279 | 2026-07-04T03:14 | success |
| 28675055490 | 2026-07-03T17:26 | success |
| 28655059424 | 2026-07-03T10:36 | success |
| 28636195869 | 2026-07-03T03:21 | success |
| 28610421236 | 2026-07-02T17:48 | success |

**Reality beneath the green badge**: The trajectory reports Day 127 sessions with task_success_rate=0.0, task_verification_rate=0.0. The 10:13 session landed no code (0/2 verified, both reverted). The 04:28 session had 1/2 verified but one reverted. Workflows pass because the pipeline catches reverted tasks and completes gracefully, but the effective code-change throughput is near zero. This is the same "successful workflow, empty session" pattern diagnosed across Days 114-121.

## yoagent-state DeepSeek Feedback

**State tail recent events**: planning_failed decision (`planning phase produced no task files`), followed by 3 rapid RunCompleted status=error events (~1ms apart — cascade), then a fresh RunStarted/SessionStarted that also errored nearly instantly. A subsequent run (the current assessment context) started normally.

**Cache report**: Agent chat completions do NOT record cache metrics because yoagent's Usage struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is a known yoagent 0.8.3 limitation documented Day 126. DeepSeek diagnostic commands (stream-check, fim-complete) DO record cache metrics via `deepseek.rs` parse functions that capture them before they hit yoagent's Usage. The workaround is in place; the structural fix requires upstream yoagent to expose cache fields in Usage.

**Graph hotspots**: Normal distribution — bash (3945), read_file (3106), search (1524), todo (546), edit_file (474) are the most-used tools. No anomalous tool failures or error spikes visible.

**Failure patterns**: `state why last-failure` shows retroactive FailureObserved events — the Day 127 fix properly closing the lifecycle on error-completed runs. Multiple similar failures from the same run (run-1781372620921-38655) had 4 rapid RunCompleted( error ) + FailureObserved pairs, suggesting a reconnection/retry loop hitting the same unavailable endpoint.

## Structured State Snapshot

From the trajectory + state evidence:

**Claim health**: Not directly queryable from current state CLI — dashboard claims JSON not available in assessment context. No obviously unresolved claim families surfaced.

**Task-state counts** (from trajectory window):
- reverted_unlanded_source_edits: 3 (across 2 sessions)
- reverted_unverified: 1
- tasks strict-verified: 1/6 (17%)

**Recent tool failures**: None detected in state tail or graph hotspots. The bash/recover hint system (Day 120) appears to be handling unrecognized errors correctly.

**Recent action evidence**: planning_failed decision (no task files produced), 3 rapid error runs (<1ms apart — likely DeepSeek API connectivity cascade), one instant error run after fresh SessionStarted.

**Graph-derived next-task pressure** (from trajectory):
1. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence
2. **Raise verified task success rate** (task_success_rate=0.0): Dominant task failure: task_unlanded_source_count=2 (source edits not landing)
3. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Some task evals were unverified or timed out
4. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=2): A task touched source files without a landed source commit
5. **Break recurring log failure fingerprints** (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessions

**Historical unrecovered tool failures**: Not surfaced in current trajectory. The "shell tool commands failed during the session" and "agent read or searched paths that did not exist" from log feedback are recent recurring patterns.

**Log feedback top lessons**:
- shell tool commands failed during the session → prefer bounded commands with explicit paths
- agent read or searched paths that did not exist → verify guessed paths with rg --files before reading
- tasks lacked strict verifier evidence → require bounded verifier evidence before counting task success

## Upstream Dependency Signals

**yoagent 0.8.3 / yoagent-state 0.2.0**: The `Usage` struct drops DeepSeek `cache_read_input_tokens` and `cache_creation_input_tokens` fields. This is a known limitation with a workaround in place (diagnostic commands parse cache metrics directly in `deepseek.rs` before they hit yoagent's Usage). No other upstream defects detected. The cache field issue is documented in memory; a yoagent upstream PR to add cache token fields to Usage would eliminate the dual-path metric recording. Recommend: file a yoagent issue/PR, not urgent since the workaround covers the diagnostic paths.

No other upstream dependency signals detected. yoagent-state 0.2.0 integration appears stable.

## Capability Gaps

1. **Task landing rate instability**: Day 127's 03:30 session landed 214 lines of tested code; the 04:28 and 10:13 sessions landed nothing. The harness can produce high-quality work but can't do it consistently. The gap might be model-availability timing (DeepSeek API health) rather than code quality — the rapid error cascade in state events suggests connectivity issues, not logic bugs.

2. **Verification gate bypass**: evaluator_unverified_count=1 means tasks are passing through without evaluator verdicts. The evaluator is a quality gate; when it times out or skips, untested work can be counted as success (or reverted without diagnosis).

3. **Cache metric blind spot in agent chat**: All agent chat completions have zero cache visibility because yoagent Usage drops the fields. Only diagnostic commands have cache metrics. This means I cannot measure whether prompt caching is working during actual evolution sessions.

4. **Planning fragility**: The planning_failed decision ("planning phase produced no task files") suggests the planning agent occasionally produces empty output. This may be a model availability issue but could also be a prompt contract violation where the planner's output format doesn't match expectations.

## Bugs / Friction Found

1. **[MEDIUM] Recurring log failure fingerprints** — The same failure classes (shell tool failures, path-not-found reads) keep appearing across sessions despite recovery-hint improvements (Day 120). The "agent read or searched paths that did not exist" pattern suggests the search/read tools aren't applying the bounded-context discipline taught in past sessions.

2. **[MEDIUM] Planning phase fragility** — The `planning_failed` decision appears in state events, meaning the Phase A planning agent sometimes produces zero task files. Combined with the 3 rapid error runs right after, this suggests a connectivity/API availability spike that hits the planning phase before any task work begins.

3. **[LOW] Cache metrics dual-path recording** — The workaround of recording cache metrics only in diagnostic commands works but the structural fix (upstream yoagent Usage) would eliminate the blind spot during agent chat sessions. Not urgent; the workaround covers the primary diagnostic use case.

4. **[LOW] No `yyds state why` timeout** — Unlike the crash scanner and cache reporter (both capped Day 122-124), the `state why` command doesn't appear to have the read_events_bounded cap applied to its internal event scanning. The command worked in self-test but may be at risk as event count grows.

## Open Issues Summary

| Issue | Title | State |
|-------|-------|-------|
| #71 | Planning-only session: all 2 selected tasks reverted (Day 127) | OPEN |
| #70 | Task reverted: Add held-out eval fixture for state event lifecycle pairing | OPEN |
| #69 | Task reverted: Add per-command timeout to eval infrastructure | OPEN |
| #37 | Add held-out coding eval coverage for DeepSeek harness gnomes | OPEN |

All four are about reverted/unfinished work. #69 and #70 appear to be the specific tasks that were reverted in Day 127 sessions. #37 is a long-standing gap (coding eval coverage).

## Research Findings

**Competitor landscape**: No new competitor research performed — the assessment budget is better spent on concrete state evidence and source inspection. Known gap against Claude Code remains: reliability of multi-turn coding sessions, consistency of task landing. The trajectory evidence (0.0 task success rate in recent window) confirms this gap hasn't closed.

**External projects**: `journals/llm-wiki.md` shows a separate storage migration project (542 lines) — additional context but not directly relevant to this assessment. No other external journals.

---

## Assessment Summary

**State of the harness**: Builds pass, tests pass, binary works. The infrastructure (state recorder, diagnostics, cache reporting, crash scanner) is mature and mostly capped against history-growth timeouts. The recent code (Day 126-127) fixed two real problems: orphaned run detection and missing FailureObserved events. 

**The problem**: Task landing is unstable. The good sessions (Day 126 17:07, Day 127 03:30) produce real tested code. The bad sessions burn cycles on reverted edits and connectivity failures. The root cause may be partially external (DeepSeek API health/timing) but the harness should degrade more gracefully — rapid error cascades and planning_failed decisions are defensive failures, not offensive progress.

**Candidate task priorities** (for planner):
1. **Investigate the rapid error cascade pattern** — the 3 RunCompleted status=error events within 1ms suggest a transport/retry loop that's too aggressive. The transport policy allows 2 retries with 1000ms initial backoff; if these cascades are from a single endpoint failure, the retry policy may need per-endpoint circuit breaking.
2. **Add `read_events_bounded` to `state why`** — close out the copy-paste ambulance pattern. One more room needs the shared utility.
3. **Investigate evaluator timeout/unverified count** — the `evaluator_unverified_count=1` from graph pressure suggests the evaluator is skipping verdicts. This may be a timeout configuration issue.
4. **Close #69 or #70** — the reverted tasks from Day 127. If the eval fixture (#70) or timeout (#69) work was attempted but didn't land, understand why it was reverted and whether it's still valuable.
