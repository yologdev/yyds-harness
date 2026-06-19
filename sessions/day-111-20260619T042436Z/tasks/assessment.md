# Assessment — Day 111

## Build Status
- `cargo build`: PASS (clean, 0.14s)
- `cargo test`: TIMED OUT at 120s (CI runner limitation; all 9 prior evolution runs concluded "success")

## Recent Changes (last 3 sessions)

**Day 110 (4 sessions):**
- `scripts/preseed_session_plan.py`: File existence check before assigning tasks — prevents pointing agents at renamed/deleted files (46 lines). Tests included.
- `src/commands_deepseek.rs`: `read_events_from_sqlite()` fallback — cache-report now reads SQLite when events.jsonl is missing (54 lines).
- `scripts/build_evolution_dashboard.py`: `unique_delta_labels()` — returns actual tool names (not just counts) for state/transcript disagreements. Session-to-claim mapping for unproven claims.
- `src/deepseek.rs`: `is_token_backed()` — distinguishes genuine zero cache hits from missing cache metrics.
- `src/state.rs`: `state_directory_info()` and improved cluster/help tips.
- `.skill_evolve_counter` bumped multiple times (now at some intermediate value).

**Day 109 (3 sessions):**
- Task verification gate: capture diff evidence for reverted-no-edit tasks (`scripts/evolve.sh`)
- Evidence-backed planning repair after no-task sessions
- Recovery hints rewrite: instead of "try different reader," suggests path-finding commands
- Cold-start diagnostics: `state_directory_info()` discriminates 3 no-events scenarios
- Analysis-only retry stop: harness no longer retries implementation attempts that produce zero file changes (34 lines evolve.sh)

**Pattern:** The dominant theme across Days 109-110 is **discrimination** — replacing catch-all messages/counts with specific, actionable diagnostics. Not building new capabilities but making existing diagnostics actually helpful.

## Source Architecture

78 source files under `src/`. Total ~139K lines.

| File | Lines | Role |
|------|-------|------|
| commands_state.rs | 24,486 | State CLI: tail, why, graph, failures, evals, patches, etc. |
| state.rs | 6,991 | State recording: events, harness patches, evals, SQLite projection |
| commands_eval.rs | 6,635 | Eval subsystem: evaluator checks, verdict recording |
| commands_evolve.rs | 5,528 | Evolution session orchestration |
| deepseek.rs | 3,986 | DeepSeek protocol: transport, cache, FIM, strict schemas, routing |
| cli.rs | 3,688 | CLI entry point, argument parsing, subcommand dispatch |
| symbols.rs | 3,679 | Symbol extraction and code analysis |
| tools.rs | 3,394 | Tool implementations (bash, edit, search, sub_agent, etc.) |
| tool_wrappers.rs | 3,158 | Tool decorators: guarded, truncating, recovery hints |
| commands_deepseek.rs | 3,149 | DeepSeek-specific CLI: cache-report, FIM, protocol checks |
| ... | | (68 more files, 59-3,104 lines each) |

Key scripts: `scripts/evolve.sh` (3509 lines), `scripts/build_evolution_dashboard.py` (7735), `scripts/log_feedback.py` (2964), `scripts/preseed_session_plan.py` (993).

## Self-Test Results

| Test | Result | Notes |
|------|--------|-------|
| `cargo build` | PASS | Clean, 0.14s |
| `cargo test` | TIMEOUT (120s) | Full suite too large for CI runner timeout |
| `state tail --limit 20` | PASS | Shows 20 live events from current session |
| `state why last-failure` | PASS | Correctly reports "session in progress" with run ID |
| `state graph hotspots --limit 10` | PASS | bash (3837), read_file (3168), search (1686) top tools |
| `deepseek cache-report` | PASS | 95.74% hit ratio, 154M hit / 6.9M miss tokens |
| `state evals` | TIMEOUT (10s) | Events file too large (39MB, 34K events) for live scan |
| `state patches` | TIMEOUT (10s) | Same root cause |
| `state failures tools` | TIMEOUT (10s) | Ditto — needs `--recent` or `--by-session` for bounded scan |
| `state failures --recent` | PASS | 12 recent failures (all from this assessment session) |

**Key friction:** Several state diagnostic commands time out on the full 39MB events file. The `--recent` flag works but `--by-session` and unbounded scans don't complete within default timeouts.

## Evolution History (last 10 runs)

All 10 visible runs concluded `success`. No failures in this window.

| # | Started | Conclusion |
|---|---------|------------|
| 1 | 2026-06-19 04:24 | (current, in progress) |
| 2 | 2026-06-18 22:59 | success |
| 3 | 2026-06-18 19:13 | success |
| 4 | 2026-06-18 18:26 | success |
| 5 | 2026-06-18 11:50 | success |
| 6 | 2026-06-18 04:04 | success |
| 7 | 2026-06-17 23:01 | success |
| 8 | 2026-06-17 20:23 | success |
| 9 | 2026-06-17 18:18 | success |
| 10 | 2026-06-17 16:49 | success |

Workflow-level conclusion is 100% pass rate in the visible window. However, the trajectory snapshot shows tasks are only 50% successful (2/3 or 1/2 per session) — the "success" workflow conclusion masks internal task-level failures that were reverted.

## yoagent-state DeepSeek Feedback

**Cache health:** Excellent. 95.74% server-side cache hit ratio across 233 events. Only 6.9M miss tokens vs 154M hit. DeepSeek prompt caching is working well.

**Tool hotspots:** bash (3837 calls) and read_file (3168) dominate — expected for a coding agent. search (1686) is the third-most-used tool.

**Failure events (12 recent):** 6 transport timeouts (all from this assessment session's state command timeouts), 6 tool_execution errors (historical: file-not-found, search regex errors, directory-not-found). All retryable. No permanent transport failures or provider errors.

**Run lifecycle:** Current run `github-actions-27805279960` is in progress. One prior incomplete run detected. No crashes recorded.

**PatchEvaluated events:** All recent (5 visible) are "passed" with scores ranging 0.845-0.922. No failed patches.

## Structured State Snapshot

**Claim health:** 582/702 proven (82.9%). 120 non-proven (90 missing, 30 observed). 1 recent non-proven claim: run_lifecycle=1 missing.

**Lifecycle aggregate:** 69/78 observed. 39 unhealthy indicators. 111 run_incomplete, 53 model_incomplete. These are cumulative across all history, not just recent.

**Task-state counts (recent):**
- reverted_no_edit: 4 instances (tasks reverted without any file changes)
- task_success_rate: 0.5 (1/2 or 2/3 per session)
- task_verification_rate: 0.5

**Recent action evidence:** state_only_failed_tools=7 — tool failures recorded in state events without matching transcript entries. This means the state recorder and transcript logger disagree about what failed.

**Graph-derived next-task pressure:**
1. **Force reverted tasks to leave concrete evidence** (task_no_edit_revert_count=1): Implementation tasks reverted without touching files; require an early scoped edit, an obsolete note, or a concrete blocker
2. **Raise verified task success rate** (task_success_rate=0.5): Dominant failure mode is reverted_no_edit tasks
3. **Require strict verifier evidence** (task_verification_rate=0.5): Verification rate below complete without counted evaluator verdicts
4. **Break recurring log failure fingerprints** (recurring_failure_count=1): GitHub action log feedback shows repeated failure fingerprints across sessions
5. **Reconcile state-only tool failures** (state_only_failed_tool_count=7): State events contain failed tool actions without matching transcript entries

**Corrected top lessons:**
- Implementation tasks reverted without edits → force implementation agents to either make an early scoped edit, write an obsolete note, or fail with a concrete blocker

**Historical tool-failure categories:** tool_execution errors (file-not-found, grep regex errors, directory-not-found, old_text not found). These are historical and not all reproduced in recent sessions — the `edit_file` old_text mismatch and grep regex errors were addressed in Days 108-110 with recovery hint improvements.

## Upstream Dependency Signals

**yoagent:** No upstream repo configured. No current evidence of yoagent defects blocking harness work. The DeepSeek protocol layer (`src/deepseek.rs`) is self-contained.

**yoagent-state:** State is managed internally in `src/state.rs` with SQLite projection. No upstream dependency pressure detected.

**Action:** None needed now. If a yoagent limitation emerges (e.g., tool schema mismatch, thinking mode incompatibility), file a help-wanted issue against this repo rather than guessing an upstream target.

## Capability Gaps

**vs Claude Code:** The structural gaps remain architectural (cloud agents, event-driven triggers, sandboxed execution) — these are identity choices, not features to build. No new tactical gaps identified in this assessment.

**Internal gaps (most actionable):**
1. **State diagnostic timeouts:** Several `yyds state` commands time out on the full 39MB events file. Need bounded default behavior or streaming reads.
2. **Task success rate:** 50% task success (1/2 or 2/3 per session). The reverted-no-edit pattern dominates failures.
3. **State/transcript disagreement:** 7 state-only tool failures without matching transcript entries — two recording systems diverge.
4. **cargo test timeout on CI:** Full test suite (4200+ tests) takes >30s and sometimes exceeds CI timeouts. Could use targeted test selection.

## Bugs / Friction Found

1. **[HIGH] State diagnostic commands time out on large events files.** `state evals`, `state patches`, `state failures tools` all time out at 10s when scanning the full 39MB events.jsonl (34K events). The `--recent` flag mitigates this but is not the default. Impact: harness self-diagnostics become unusable as state accumulates. Candidate task: make bounded scans the default for these commands.

2. **[MEDIUM] reverted_no_edit tasks (4 instances).** Implementation agents sometimes produce no file changes, then get reverted. The preseed task picker now checks file existence (Day 110), but the implementation phase still lacks a guard that detects zero-edit attempts early. Candidate task: add early-edit check in evolve.sh implementation phase.

3. **[MEDIUM] state_only_failed_tools=7.** State events record tool failures the transcript doesn't mention. These two recording systems disagree. Candidate task: audit the tool-failure recording path to find where transcripts miss failures the state system catches.

4. **[LOW] cargo test full suite timeout.** 4200+ tests in one binary take >30s. CI runners occasionally hit the 120s timeout. Candidate task: profile test execution or enable parallel test running (currently `--test-threads=1`).

5. **[LOW] External project journal (llm-wiki.md)** is 67KB with only one recent commit (Day 109). No actionable issues but worth noting it's an active external project being tracked.

## Open Issues Summary

**agent-self issues:** None. No self-filed backlog.

**GitHub issues (all labels):** None open. Clean slate.

## Research Findings

No competitor research performed — the trajectory pressure is internal (task success rate, state-only tool failures, diagnostic timeouts) rather than competitive. The assessment budget is better spent on those.

External project journal (`journals/llm-wiki.md`) shows an active TypeScript wiki project with StorageProvider abstraction migration in progress. Not directly relevant to DeepSeek harness work.
