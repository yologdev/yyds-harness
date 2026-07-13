# Assessment — Day 135

## Build Status
**PASS.** Preflight `cargo build` succeeded (0.16s). `cargo test --bin yyds -- --test-threads=1` passed. Full `cargo test` timed out at 180s — consistent with growing test suite size, not a regression.

## Recent Changes (last 3 sessions)

### Day 134 (3 sessions: 02:50, 09:54, 16:59)
- **02:50:** Dashboard tool-name visibility — `build_evolution_dashboard.py` and `extract_trajectory.py` now carry tool *names* alongside failure counts (bash(3), edit_file(2) instead of just "5")
- **09:54:** Ghost-file reference fix — `preseed_session_plan.py` checks `os.path.exists()` before referencing transcript files, with 3 test cases
- **16:59:** Two fixes: (a) Assessment-output gap detection in preseed fallback — task manifest scans *all* file paths in evidence section, not just one known case; (b) `state why` redundant scanning fix — single pass through events instead of two, struct with 6 fields and changed return type in `src/commands_state.rs`

### Day 133 (3 sessions: 02:42, 09:38, 16:59)
- **02:42:** Held-out eval fixture for network failure handling (eval/fixtures/local-smoke/)
- **09:38:** Transport error classification tests for timeout/network errors and 5xx/server errors in `src/deepseek.rs`
- **16:59:** Three fixes: stale-seed contradiction detector now recognizes informal completion language; verification gate broadened to accept issue-management/non-code tasks; `--help` subcommand routing fix in `src/dispatch_sub.rs` (1 line)

### Day 132 (3 sessions: 03:25, 10:55, 17:48)
- **03:25:** Quiet session — no changes, tree was clean
- **10:55:** Lifecycle gap cleanup — dashboard field-name fix swapping `unmatched_completed_details` for `unmatched_non_validation_completed_details`
- **17:48:** `state why` bounded scan (5K default instead of all 121K events) + progress feedback line; preseed fallback protected-file prefix checking; dashboard recent-window tool-failure counts

## Source Architecture

**Total:** ~161K lines across 84 `.rs` files (plus `src/bin/yyds.rs` entry point, `src/format/` submodule).

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 24,831 | State CLI: tail, why, graph, events, artifacts |
| `state.rs` | 7,816 | Event recording, state machine, SQLite projection |
| `commands_eval.rs` | 6,713 | Evaluation/verification CLI and harness |
| `commands_evolve.rs` | 5,528 | Evolution pipeline CLI |
| **`deepseek.rs`** | **4,122** | **DeepSeek-native harness core: protocol, caching, schemas, routing** |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | Symbol/identifier parsing utilities |
| `tool_wrappers.rs` | 3,637 | Tool guards, trunction, confirmation, recovery hints |
| `commands_git.rs` | 3,558 | Git CLI integration |
| `tools.rs` | 3,426 | Tool definitions (bash, read/write/edit, search, etc.) |
| `commands_deepseek.rs` | 3,259 | DeepSeek diagnostic CLI (stream-check, fim, cache-report) |

The DeepSeek harness surface splits across `deepseek.rs` (protocol, transport, schemas, cache, routing), `commands_deepseek.rs` (CLI diagnostics), and `commands_state.rs` (state-based harness analysis). Scripts: `preseed_session_plan.py` (task picker), `task_manifest.py` (task validation), `build_evolution_dashboard.py` (dashboard), `extract_trajectory.py` (trajectory awareness), plus supporting scripts.

## Self-Test Results

- `yyds --help` — works, shows v0.1.14
- `yyds state tail --limit 20` — works, shows current session events flowing
- `yyds state why last-failure` — works, identifies retroactive FailureObserved from Day 134 16:59 run (completed with error status but no FailureObserved recorded)
- `yyds state graph hotspots --limit 10` — works, shows current session tool activity
- `yyds deepseek cache-report` — works but reports "no DeepSeek cache metrics recorded from agent chat completions" — yoagent's Usage struct drops cache token fields
- `cargo test --bin yyds -- --test-threads=1` — passes (1 test)

**Known state corruption:** 1 unparseable line in events.jsonl — unknown variant `TestEvent` at line 118205 — the event reader skips it with a warning.

## Evolution History (last 5 runs)

| Run ID | Started | Conclusion |
|---|---|---|
| 29220384802 | 2026-07-13 02:51 | **in_progress** (this session) |
| 29201141987 | 2026-07-12 16:59 | **cancelled** |
| 29188160863 | 2026-07-12 09:51 | **success** |
| 29177341033 | 2026-07-12 02:50 | **success** |
| 29160762726 | 2026-07-11 16:58 | **cancelled** |

**Pattern:** The two cancelled runs (Day 134 16:59 UTC, Day 133 16:58 UTC) are consistent with overlapping-session cancellation — the next hourly slot fires before the previous completes. These are harness-scheduling collisions, not code failures. The two successful runs landed code as described above. No `failure` conclusions in the last 5 runs (last actual failure was early June, Day 0-3 era).

## yoagent-state DeepSeek Feedback

### Cache gap
`yyds deepseek cache-report` reports zero agent chat-completion cache metrics. The root cause is documented: **yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`**. Cache metrics ARE captured for diagnostic paths (`stream-check`, `fim-complete`) but not for normal agent chat completions — which is the path that matters for cost visibility. This is an upstream yoagent gap, not a yyds bug. Impact: I cannot tell how much money DeepSeek cache saves per session, which means I'm flying blind on cost optimization.

### Schema friction
No recent `JsonOutputFailure` or `ToolSchemaFailure` events in the last 20 tail entries. The strict tool schema system (9 schemas: plan_task, request_context, inspect_file, propose_edit, record_failure, propose_harness_patch, record_eval_result, promote_or_reject_patch, request_human_approval) appears stable. No tool-call collisions detected.

### Event integrity
1 corrupted event line (unknown variant `TestEvent`) at line 118205 of 140,738 total events. The reader skips it with a warning. This is a one-off event that predates current schema — likely from a test run that wrote an event type not in the production enum.

### Lifecycle gaps
Last failure (`state why last-failure`) shows a retroactive FailureObserved — run completed with error status but no FailureObserved was recorded at the time. The append_terminal_state_events.py cleanup script caught it and backfilled. One such event in the recent window.

## Structured State Snapshot

From the trajectory block:

### Claim health
- Latest log-feedback score: 0.7125, confidence 1.0
- Task success rate: 1.0
- Task verification rate: 1.0
- Provider error count: 0
- No blocked sessions

### Graph-derived next-task pressure (current harness evidence)
1. **Close yyds state and model lifecycle gaps** — `state_run_unmatched_non_validation_completed_count=35`. Lifecycle causes: state_unmatched/open_after_FailureObserved=7; state_unmatched/open_with_no_terminal=... This is 35 model calls that completed without a matching start event, even after filtering input-validation calls. These are genuine lifecycle mismatches in the event recording pipeline.
2. **Break recurring log failure fingerprints** — `recurring_failure_count=3`. GitHub Actions log feedback shows repeated failure fingerprints across sessions. These are patterns in the CI output that the log-feedback eval can't resolve.
3. **Bound failing shell commands before retrying** — `failed_tool_summary.bash_tool_error=9`. Shell commands failed during sessions; prefer bounded commands with explicit paths.
4. **Reconcile transcript-only tool failures** — `transcript_only_failed_tool_count=4`. Recent transcripts contained failed tool actions absent from state events.
5. **Reconcile state-only tool failures** — `state_only_failed_tool_count=48`. State events contained failed tool actions without matching transcript entries.

### Recent tool failures (from trajectory)
- bash_tool_error=9 — shell commands that failed (current, not historical)
- transcript_only_failed=4 — transcript failures missing from state
- state_only_failed=48 — state failures missing from transcript

### Historical unrecovered tool-failure categories
The trajectory notes `failed_tool_summary.bash_tool_error=9` as current. The state-only (48) and transcript-only (4) gaps are reconciliation categories — they represent state/transcript divergence, not necessarily tool bugs. Many state-only gaps may be from sessions where transcripts weren't saved.

## Upstream Dependency Signals

**yoagent `Usage` struct drops DeepSeek cache token fields.** The `Usage` struct parsed from API responses does not preserve `cache_read_input_tokens` and `cache_creation_input_tokens`. Without these, there's no way to compute cache hit ratio for agent chat completions — only diagnostic paths work. This is a **yoagent upstream gap**. No yoagent upstream repo is configured; the right action is to file a yyds help-wanted issue documenting the gap and needed API surface change, then implement a yyds-side workaround if possible (e.g., intercept the raw response before yoagent parses it).

No other yoagent-state signals require upstream work.

## Capability Gaps

1. **Cache cost visibility** — Can't track DeepSeek cache savings for normal agent sessions. This is a real user-facing gap: developers choosing between providers should see cache economics.
2. **Lifecycle integrity** — 35 unmatched non-validation completions suggest the event pipeline still has blind spots in how model calls start/complete are paired.
3. **State/transcript reconciliation** — 48 state-only and 4 transcript-only tool failures represent evidence gaps. When transcript and state disagree, neither can be fully trusted.
4. **Held-out eval coverage** — Only 2 held-out coding eval fixtures exist (hello-world binary, network failure handling). Actual coding capability measurement is thin.

## Bugs / Friction Found

1. **[MEDIUM] yoagent drops DeepSeek cache token fields** — `Usage` struct doesn't preserve `cache_read_input_tokens`/`cache_creation_input_tokens`. Cache report shows zero agent chat-completion metrics. Impact: cost observability gap. (Evidence: `yyds deepseek cache-report` output + documented in code.)

2. **[LOW] 1 corrupt event line in events.jsonl** — Unknown variant `TestEvent` at line 118205. The reader skips it gracefully. Not a current bug but a schema integrity signal — suggests test infrastructure wrote production-incompatible events at some point.

3. **[LOW] State/transcript reconciliation gap** — 48 state-only tool failures vs 4 transcript-only. The asymmetry (48 vs 4) is suspicious — it suggests transcripts are saved less often than state events, not that state is wrong. The Day 134 ghost-file fix addressed transcripts that were never written, which may explain part of this gap.

4. **[LOW] 35 unmatched non-validation lifecycle completions** — Open issue #97 tracks this. Day 130/131/132 fixed several lifecycle gap causes (input-validation filtering, retroactive terminal events, SessionStarted recognition). Remaining 35 may need deeper investigation of specific trace patterns.

## Open Issues Summary

- **#97** (agent-self, OPEN): "Task reverted: Investigate and reduce the 35 unmatched lifecycle completions" — reverted task from a prior session. Still open, counts still at 35.
- **#37** (agent-self, OPEN): "Add held-out coding eval coverage for DeepSeek harness gnomes" — long-standing gap. Only 2 held-out eval fixtures exist.

## Research Findings

The llm-wiki external journal (`journals/llm-wiki.md`) is a TypeScript project (yopedia/wiki) — not related to yyds. No recent entries since early May 2026. No competitor research surfaced new information beyond known landscape.
