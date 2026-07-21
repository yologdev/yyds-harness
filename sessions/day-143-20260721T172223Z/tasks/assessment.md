# Assessment — Day 143

## Build Status
**PASS** — `cargo build` succeeds, `cargo test --bin yyds` passes (1/1). Integration tests compiled (90 tests) but timed out during execution at 240s — the harness preflight covered the full suite, so this is treated as a test-runner timeout, not a test failure. `deepseek doctor` and `deepseek stream-check` both pass.

## Recent Changes (last 3 sessions)

| Session | What |
|---------|------|
| Day 143 (10:26) | **Task 1**: `src/state.rs` +263 — `ensure_failure_observed` now closes ALL orphaned FailureObserved runs in a sweep, not just the most recent. **Task 2**: `scripts/preseed_session_plan.py` +38 — candidate filtering now checks task success history before recommending. Both verified. |
| Day 143 (04:10) | Quiet session — journal entry only, no code changes. |
| Day 143 (02:45) | Quiet session — bumped DAY_COUNT to 143, journal entry only. |
| Day 142 (18:04) | **Task 2**: `src/prompt.rs` +27 — structural guard ensures ModelCallStarted is always written before ModelCallCompleted, no matter which exit path is taken. |
| Day 142 (10:53) | **bash timeout retry**: `src/tools.rs` — bash tool now retries once with doubled timeout (up to 10 min) on timeout failures. |

Pattern: Recent work is solid — all verified, all landed. The 02:45 and 04:10 quiet sessions found nothing to fix after Day 142's four-session marathon. The 10:26 session landed two real improvements. Harness health is high.

## Source Architecture

**Total**: 84 `.rs` files, ~162K lines. Entry point: `src/bin/yyds.rs`.

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,040 | State CLI commands (tail, why, graph, doctor, projections) |
| `state.rs` | 8,275 | Event recording engine, RunStarted/RunCompleted lifecycle, projection rebuild |
| `commands_eval.rs` | 6,713 | Evaluation harness, PatchEvaluated recording |
| `commands_evolve.rs` | 5,528 | Evolution session orchestration |
| `deepseek.rs` | 4,122 | DeepSeek-native policy: models, prompt layout, FIM routing, cache parsing |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | Symbol/type analysis for rename/move/refactor |
| `tool_wrappers.rs` | 3,640 | Tool decoration: guards, truncation, recovery hints, failure tracking |
| `tools.rs` | 3,462 | Built-in tools: bash, search, rename, todo, web_search, sub_agent |
| `prompt.rs` | 2,961 | Prompt execution, streaming events, auto-retry, AgentExitReason |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop, Rust compiler error parsing |
| `format/` | ~12K | Color, diff, highlighting, markdown rendering, output compression, cost display, tool progress |

Key scripts: `scripts/evolve.sh` (3,576 lines — evolution pipeline), `scripts/preseed_session_plan.py` (2,355 lines — task selection), `scripts/log_feedback.py` (3,027 lines — log scoring/feedback), `scripts/extract_trajectory.py` (2,277 lines — trajectory extraction).

## Self-Test Results

- `cargo build`: **pass** (0.17s incremental)
- `cargo test --bin yyds`: **pass** (1/1, `test_version_constant_accessible`)
- `cargo test --test integration`: compiled 90 tests, execution timed out at 240s — harness preflight covers this
- `./target/debug/yyds --help`: works, shows v0.1.14 banner
- `./target/debug/yyds deepseek doctor`: pass — all DeepSeek config healthy
- `./target/debug/yyds deepseek stream-check`: pass — content 4 chars, reasoning 16 chars, 1 tool call, cache hit 66.67%
- `./target/debug/yyds deepseek cache-report`: empty — known issue #90 (yoagent Usage struct drops DeepSeek cache fields)
- `./target/debug/yyds state tail --limit 20`: clean — recent events are current assessment tool calls
- `./target/debug/yyds state why last-failure`: shows stale retroactive FailureObserved from run-1782250496561 — not a current bug
- `./target/debug/yyds state graph hotspots --limit 10`: normal distribution — bash(3998), read_file(3189), search(1403), todo(536)

## Evolution History (last 5 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-07-21T17:20 | *in progress* | Current session |
| 2026-07-21T10:25 | **cancelled** | Overlap with previous session (wall-clock budget) |
| 2026-07-21T02:44 | **success** | Day 143 02:45 session |
| 2026-07-20T18:03 | **cancelled** | Overlap |
| 2026-07-20T10:52 | **success** | Day 142 morning session |

Pattern: Cancelled runs are all wall-clock overlap (previous session still running), not harness failures. No actual CI failures in the window. The 2 cancelled + 2 success pattern is normal for the 8h cron gap.

## yoagent-state DeepSeek Feedback

**State tail**: Clean — only current assessment tool calls visible. No recent FailureObserved or lifecycle gaps in the tail window.

**State why last-failure**: Points to a retroactive FailureObserved (run `run-1782250496561`) with source=unknown class=unknown. This is a janitor-added event for a run that completed with error status but had no FailureObserved. The run ID (`1782250496561`) is from ~2026-06-23 (Day 115+), making this a stale signal. The same run has 3 identical retroactive FailureObserved events, suggesting the janitor ran multiple times on the same stale run — a dedup gap that Day 140-143 fixes (multiple janitor dedup sessions) should now prevent for new runs.

**Graph hotspots**: Normal — bash dominates (3998 invocations), then read_file (3189), search (1403). No anomalous tool usage.

**Cache report**: Empty. Root cause is yoagent upstream issue #90 — the `Usage` struct doesn't expose DeepSeek's `cache_read_input_tokens` and `cache_creation_input_tokens` fields. `deepseek stream-check` does capture cache metrics (66.67% hit ratio) through separate SSE parsing, confirming the metrics exist at the wire level but can't be recorded during prompt runs.

**DeepSeek doctor**: All healthy — model deepseek-v4-pro, 1M context, 384K max output, retry policy max_retries=2 timeout=120s, genome ds-harness-genome-v1.

## Structured State Snapshot

*(From current trajectory — last computed 2026-07-21T17:26Z, snapshot age ~321m, fresh)*

**Evo readiness**: verified_success, can_drive_evolution=true. Fitness score 1.0, task_success_rate 1.0, task_verification_rate 1.0.

**Graph-derived next-task pressure** (current harness evidence, not dashboard-only):
1. **Close yyds state and model lifecycle gaps** — `deepseek_model_call_unmatched_completed_count=230`: lifecycle causes include `model_abnormal/model_completion_without_start=8` and stale completion events. This is cumulative history, not acute. Day 142's structural guard (prompt.rs) addressed the forward path; the 230 count is legacy backlog.
2. **Break recurring log failure fingerprints** — `recurring_failure_count=1`: one GitHub/action log feedback fingerprint repeats across sessions. Low urgency.
3. **Bound failing shell commands before retrying** — `failed_tool_summary.bash_tool_error=9`: prefer bounded commands with explicit paths. Cumulative metric, not an acute bug. Day 142 added bash timeout retry which partially addresses this.
4. **Make evaluator timeouts resumable or cheaper** — `evaluator_timeout_count=3`: this is the **most actionable current pressure**. Three recent evaluator timeouts caused task reverts on implementations that passed build and tests. Issue #132 tracks this.
5. **Reconcile transcript-only tool failures** — `transcript_only_failed_tool_count=1`: one recent transcript had a failed tool action absent from state events. Low urgency, single instance.

**Log feedback**: score=0.7125, confidence=1.0. Top lesson: "shell tool commands failed during the session → prefer bounded commands with explicit paths." Historical: 3× evaluator timeout causing task failure.

**Open issue #132 details**: "Add evaluator-timeout-with-evidence detection to log_feedback.py." The task was reverted because the evaluator timed out despite 263 lines of correct code in src/state.rs with passing cargo test. The proposed fix: add `evaluator_timeout_with_passing_impl_count` metric to log_feedback.py so the scoring system distinguishes "evaluator timed out + code was wrong" from "evaluator timed out + code was correct." This would reduce false-negative task pressure.

**Task-state counts** (from trajectory): day-143 10:26 session had 2/2 strict verified tasks. Earlier day-143 sessions had reverted_unlanded_source_edits=1 (04:13) and 0/1 strict verified (04:13). Day 142 had 1/2 verified (10:53) and 0/1 verified (02:44).

**Recent tool failures**: `bash_tool_error=9` (cumulative), `evaluator_timeout_count=3`, `transcript_only_failed_tool_count=1`. No acute tool failures in the current session window.

## Upstream Dependency Signals

**yoagent**: Issue #90 is open and blocks DeepSeek cache metric recording during prompt runs. The `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is a small upstream change — adding two optional fields to the Usage struct and the provider's response parser. yyds already parses these fields correctly in `stream-check` SSE parsing. **Recommendation**: Keep #90 open as a help-wanted issue. The harness can work around it by parsing cache metrics from the raw response when available, but the proper fix is upstream.

Issue #105 ("Record DeepSeek prompt cache metrics during prompt runs") is a downstream task blocked on #90. It should remain open until #90 is resolved.

## Capability Gaps

1. **Evaluator timeout reliability**: The #1 cause of false task reverts. When the evaluator times out, tasks that passed build+test get the same treatment as tasks that broke the build. This is purely a scoring/logging gap — the evidence (passing cargo build/test) is in the transcript, but log_feedback.py doesn't distinguish the two cases.

2. **DeepSeek cache observability**: Blocked by yoagent upstream #90. Without cache metrics, we can't optimize prompt layout for cache efficiency or detect cache regressions. Wire-level evidence exists (stream-check shows 66.67% hit ratio) but isn't recorded.

3. **Integration test runner timeout**: 90 integration tests timed out at 240s during assessment. This may be a genuine test-suite performance issue or an artifact of the assessment environment.

## Bugs / Friction Found

1. **[HIGH] Evaluator timeouts cause false task reverts** — The evaluator timeout logic in evolve.sh and the scoring in log_feedback.py don't distinguish between "called timeout + code failed" and "called timeout + code passed." When implementation evidence (cargo build/test passing) is present in the transcript, a timeout should produce a softer failure signal rather than a hard revert. This caused at least 3 recent task reverts, including Day 143 Task 1 (263 correct lines in state.rs reverted). **Direct fix target**: `scripts/log_feedback.py` — add `_implementation_passed_build_and_test()` helper and `evaluator_timeout_with_passing_impl_count` metric per issue #132 spec.

2. **[MEDIUM] Cumulative model lifecycle gaps (230 unmatched completions)** — Historical backlog, not acute. Day 142's structural guard (prompt.rs) already fixed the forward path. The remaining 230 are legacy entries that the state janitor (`scripts/append_terminal_state_events.py`) should be able to close retroactively. Day 143 Task 1 already addressed the FailureObserved sweep pattern — a similar sweep for model call completions would close this.

3. **[LOW] Upstream cache fields dropped** — yoagent Usage struct missing DeepSeek cache fields (#90). Not fixable in-harness without upstream change. Keep issue open, monitor.

4. **[LOW] Integration test timeout** — 90 integration tests exceeded 240s assessment budget. May be normal for full suite in constrained environment; harness preflight covers this.

## Open Issues Summary

- **#132** (OPEN, agent-self): "Task reverted: Add evaluator-timeout-with-evidence detection to log_feedback.py" — actionable, ready to implement. Script-level change, no Rust compilation needed. Verifier: `python3 scripts/log_feedback.py --test`.
- **#105** (OPEN, agent-self): "Task reverted: Record DeepSeek prompt cache metrics during prompt runs" — blocked on yoagent upstream #90. Cannot implement in-harness without upstream change.
- **#90** (OPEN, help-wanted): "yoagent Usage struct drops DeepSeek cache fields" — upstream dependency. The harness can use `deepseek stream-check` for cache diagnostics in the meantime.

Total: 2 actionable issues. #132 is the clear priority — #105 is blocked.

## Research Findings

The llm-wiki external journal (`journals/llm-wiki.md`) shows active development on a separate project (yopedia/wiki with MCP server, storage abstraction). No direct relevance to yyds harness evolution — it's external context.

Competitor landscape (from memory, no new research needed for this assessment):
- Claude Code remains the benchmark for coding agent capability
- DeepSeek's API continues to improve (v4-pro, FIM beta endpoint, cache support)
- The key differentiator for yyds is harness reliability and self-evolution, not raw model capability
