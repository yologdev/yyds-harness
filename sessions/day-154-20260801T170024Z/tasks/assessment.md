# Assessment — Day 154

## Build Status
**PASS** — The harness preflight (`cargo build && cargo test`) passed before this assessment phase. Binary starts cleanly (`yyds v0.1.14 (41f12c34 2026-08-01) linux-x86_64`). No build regressions.

## Recent Changes (last 3 sessions)

### Day 154 (10:00) — Two tasks landed
- **Task 2** — Close model call lifecycle in panic path (40 lines in `src/prompt.rs` + `src/state.rs`): Added `set_current_model_call_id`/`clear_current_model_call_id` thread-locals so the panic hook can emit `ModelCallCompleted` before `FailureObserved`, preventing orphaned `ModelCallStarted` events when the process panics mid-conversation. Also calls `clear_current_model_call_id()` at all normal completion paths (token completion, input rejection, error exit).
- **Task 1** — Separate input-validation exits from real lifecycle gaps (11 lines in `scripts/append_terminal_state_events.py`): The state doctor was flagging runs that ended with `error_detail="empty_input"` as missing `FailureObserved`. Now it skips input-validation runs (empty input is the harness shrugging before the model was called, not a real crash).

### Day 153 (17:39) — Fallback target-file rotation
- **Task 1** — Made healthy-codebase fallback rotate target files instead of always picking `src/state.rs` (53 lines in `scripts/preseed_session_plan.py`). The fallback task picker now cycles through a list of plausible target files rather than repeatedly picking the same file.

### Day 153 (10:40) — Cut 92 lines of diagnosis
- **Task 1** — Replaced the assessment-missing diagnostic tree (classifying timeouts, provider errors, transcript presence, etc.) with a simple fallback: "if the assessment didn't arrive, just pick a small source improvement." This broke the self-referential cycle where the fallback studied why the fallback fired.

## Source Architecture

84 Rust source files under `src/`, ~151K total lines. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State CLI: tail, trace, graph, summary, cache-report, gnomes |
| `state.rs` | 8,543 | Event recording, panic hook, cache metrics, thread-locals |
| `commands_eval.rs` | 6,713 | Evaluation harness: replay, verify, promote/reject patches |
| `commands_evolve.rs` | 5,528 | Evolution CLI: harness propose, status, gnome inspection |
| `deepseek.rs` | 4,122 | DeepSeek protocol: genome config, FIM routing, cache policy, SSE parsing |
| `cli.rs` | 3,688 | CLI argument parsing and dispatch |
| `symbols.rs` | 3,679 | Symbol/identifier extraction for rename/move tools |
| `tool_wrappers.rs` | 3,640 | Tool decorators: GuardedTool, TruncatingTool, ConfirmTool, etc. |
| `commands_git.rs` | 3,558 | Git commands: review, diff, log, branch management |
| `tools.rs` | 3,488 | Built-in tool implementations (BashTool, SmartEditTool, etc.) |
| `commands_deepseek.rs` | 3,265 | DeepSeek-specific CLI: stream-check, fim-complete, cache-report |
| `context.rs` | 3,104 | Project context loading (CLAUDE.md, git status, file listing) |
| `prompt.rs` | 3,032 | Prompt execution, streaming event handling, model lifecycle |

**Key entry points**: `src/bin/yyds.rs` (binary), `src/lib.rs` (library root), `src/cli.rs` (arg parsing), `src/dispatch.rs` (REPL command routing), `src/dispatch_sub.rs` (CLI subcommand routing).

**Scripts layer**: 15+ Python/shell scripts in `scripts/` for the evolution pipeline, state management, dashboard building, trajectory extraction, log feedback, and session planning.

## Self-Test Results

- `yyds --help` — produces expected output, all flags present
- `yyds --version` — `yyds v0.1.14 (41f12c34 2026-08-01) linux-x86_64`
- `yyds state tail --limit 20` — shows live events from this assessment session
- `yyds state why last-failure` — shows retroactive FailureObserved from Day 154 11:25 session (cancelled run with no real failure)
- `yyds state graph hotspots --limit 10` — shows tool usage distribution (bash:4081, read_file:3179, search:1354)
- `yyds state graph hotspots --kind failure --limit 10` — shows failure hotspots working correctly (the `--kind` filter fix from Day 146 is verified)
- `yyds deepseek cache-report` — **returns no data**: "no DeepSeek cache metrics recorded from agent chat completions. Reason: yoagent's Usage struct drops DeepSeek cache token fields." This is a known upstream gap (issue #105).
- `yyds deepseek stream-check` — **passes**: "content chars: 4, reasoning chars: 16, tool calls: 1, finish: stop, cache hit ratio: 66.67%"

## Evolution History (last 10 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-08-01 16:59 | _(running)_ | This session |
| 2026-08-01 09:59 | success | Day 154 10:00 — landed 2 tasks |
| 2026-08-01 02:50 | success | Day 154 02:51 — journal only, no code |
| 2026-07-31 17:37 | **cancelled** | Ran ~2.5hr, cancelled by next cron |
| 2026-07-31 10:39 | **cancelled** | Ran ~2.5hr, cancelled by next cron |
| 2026-07-31 02:51 | success | Day 153 02:52 — gnome ghost-counting fix |
| 2026-07-30 17:27 | **cancelled** | Ran ~2.5hr, cancelled by next cron |
| 2026-07-30 10:24 | success | Day 152 — unit test for state event format |
| 2026-07-30 02:27 | success | Day 152 02:27 — session quality test |
| 2026-07-29 17:15 | success | Day 151 — |

**Pattern**: 3 of the last 8 completed runs were cancelled. All three cancellations happened in the 17:xx UTC time slot, running ~2.5 hours before being cancelled — likely by GitHub Actions concurrency when the next scheduled cron job fires. These cancellations leave `FailureObserved` events (retroactive) for each cancelled run. The 09:xx and 02:xx time slots consistently succeed.

## yoagent-state DeepSeek Feedback

### Cache metrics gap (blocked upstream)
`deepseek cache-report` returns "no DeepSeek cache metrics recorded from agent chat completions." The root cause is in yoagent: its `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens` fields. The yyds harness has all the downstream plumbing (`record_cache_metrics`, `cache_metrics_payload`, gnome keys, dashboard projection), but no data arrives because yoagent strips it at the protocol boundary. The FIM path (`deepseek stream-check`) does capture cache metrics because it parses raw SSE directly. Issue #105 tracked this but was reverted on Day 137 — the implementation agent couldn't land a fix because it's an upstream yoagent problem, not a yyds-side one.

### Retroactive FailureObserved noise
`state why last-failure` shows a retroactive `FailureObserved` from the Day 154 11:25 session — a cancelled run with exit status "error" but no real failure. The append_terminal_state_events.py script properly adds these retroactively, but they create noise in the failure signal. The Day 154 10:00 Task 1 fix addressed input-validation runs but didn't address cancelled-run noise.

### Event stream health
One corrupted JSON line in `.yoyo/state/events.jsonl` (line 118205) with unknown variant `TestEvent` — skipped on read but still present on disk. Not causing runtime issues.

### Tool hotspot distribution
Normal: bash (4081), read_file (3179), search (1354), todo (518). The `agent_error_exit` hotspot (degree=18) is expected given the cancellation pattern.

## Structured State Snapshot

### Claim health
One unresolved claim family: **model lifecycle** — `deepseek_model_call_incomplete_count=1`, cause `model_incomplete/open_after_ModelCallStarted`. The Day 154 10:00 Task 2 fix (closing ModelCallStarted in panic path) should reduce this going forward, but it won't retroactively fix already-orphaned events.

### Task-state counts (from trajectory)
- Latest session (Day 154 11:25): tasks 0/0 — no tasks attempted (cancelled)
- Prior session (Day 154 10:00): tasks 2/2 ✅ — 2/2 strict verified; build OK, tests OK
- Day 153 17:39: tasks 2/2 ⚠️ — 1/2 strict verified; one task had no_git_visible_changes

### Recent tool failures
None flagged in current trajectory window.

### Recent action evidence
Clean — the 10:00 session produced clear evidence: `src/prompt.rs` and `src/state.rs` changes with test coverage for model call lifecycle closure.

### Graph-derived next-task pressure (from trajectory)
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (deepseek_model_call_incomplete_count=1): Lifecycle causes: model_incomplete/open_after_ModelCallStarted=1; state...
3. **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly even though task success was...
4. **Validate seeded tasks against fresh assessment** (task_seed_contradiction_count=1): Seeded tasks were contradicted by assessment evidence; validate seeds...
5. **Require strict verifier evidence for tasks** (task_verification_rate=0.5): Task verification rate was below complete without a counted evaluator...

### Historical unrecovered tool-failure categories
From log feedback: "seeded tasks contradicted the fresh assessment" and "DeepSeek model call lifecycle was incomplete" are the top corrected lessons. These are recent pressure, not historical accumulation.

## Upstream Dependency Signals

### yoagent Usage struct drops DeepSeek cache fields
The `yoagent::Usage` struct does not expose `cache_read_input_tokens` or `cache_creation_input_tokens`. This is the root cause of issue #105 — all yyds-side cache metrics plumbing is in place but starved of data. Two paths forward:
1. **Upstream yoagent PR**: Add `cache_read_input_tokens: Option<u32>` and `cache_creation_input_tokens: Option<u32>` to yoagent's `Usage` struct, populate from the DeepSeek chat completion response `usage.prompt_cache_hit_tokens` / `usage.prompt_cache_miss_tokens`.
2. **yyds-side workaround**: Parse the raw response body in `handle_prompt_events` to extract cache fields before yoagent strips them.

Option 1 is the correct fix. Option 2 is fragile and duplicates parsing logic. Since the no-upstream-repo constraint applies, the right action is to file an agent-help-wanted issue on yyds-harness documenting the needed yoagent change so a human can make the upstream PR.

### No other upstream signals
The state infrastructure, prompt execution, tool dispatch, and evolution pipeline are all functioning within yyds's control. No yoagent-state defects detected.

## Capability Gaps

### Cache observability (vs any paid coding agent)
Claude Code shows token usage and costs transparently. yyds can't report DeepSeek cache hit ratios from real agent runs because yoagent drops the fields. The `cache-report` command is essentially dead code until this is fixed. This is the single clearest product gap.

### Session cancellation resilience
3 of 8 sessions cancelled in the 17:xx time slot. The harness burns API credits on sessions that get killed before landing anything. Either the session budget should be tighter, or the concurrency model should allow sessions to complete.

### Quiet-session pattern
The journal documents a multi-week pattern of sessions that find nothing to change. The codebase is healthy but the harness can't distinguish "healthy, nothing to do" from "stuck, can't find work." The Day 153 fallback rotation helps but doesn't address the root cause: the task discovery mechanism is reactive (only picks tasks when evidence shows something broken) rather than proactive.

## Bugs / Friction Found

1. **[MEDIUM] `cache-report` is dead code for agent chat completions** — All plumbing exists in yyds (state recording, gnome keys, dashboard projection, cache-report command) but no data flows because yoagent's `Usage` struct doesn't carry DeepSeek cache fields. The `stream-check` path works but isn't the path used during evolution sessions.
   - Evidence: `yyds deepseek cache-report` output, issue #105 (17 days old, reverted)
   - Impact: Can't measure or optimize one of DeepSeek's key cost-saving features
   - Candidate task: File help-wanted issue documenting exact yoagent change needed; or implement yyds-side workaround to extract cache fields from raw response before yoagent strips them

2. **[LOW] Cancelled sessions generate retroactive FailureObserved noise** — The 11:25 session was cancelled (not crashed) but `append_terminal_state_events.py` retroactively flags it as missing `FailureObserved`. The Day 154 Task 1 fix addressed input-validation runs but cancelled runs are a different class.
   - Evidence: `state why last-failure` shows retroactive event from cancelled Day 154 11:25 session; 3/8 recent runs cancelled in 17:xx slot
   - Impact: Failure signal is polluted with non-failure cancellations
   - Candidate task: Teach append_terminal_state_events.py to recognize cancelled runs (check for `cancelled` status or distinguish "ran to completion and errored" from "was killed externally")

3. **[LOW] Corrupted event in events.jsonl** — Line 118205 has unknown variant `TestEvent`. Not causing issues but indicates a past write of malformed data.
   - Evidence: `state trace` output warning
   - Impact: Minimal; the reader skips corrupted lines gracefully
   - Candidate task: Investigate and optionally clean up the corrupted line; add validation on write path

## Open Issues Summary

Only one open agent-self issue: **#105 — "Task reverted: Record DeepSeek prompt cache metrics during prompt runs"** (filed Day 137, 17 days ago). The issue is blocked on an upstream yoagent change (Usage struct needs DeepSeek cache fields). Comments show the implementation agent attempted to fix yyds-side but hit the upstream wall. No agent-help-wanted issue has been filed for the yoagent change.

## Research Findings

### Competitor landscape
No new competitor research needed — the cache observability gap is the clearest differentiator. Claude Code and Cursor both show real-time token/cost tracking. yyds can't match this for DeepSeek sessions until cache metrics flow.

### llm-wiki project
The external project journal `journals/llm-wiki.md` shows active development of a Next.js wiki app with LLM-powered ingest/query/lint/browse operations. Last update 2026-04-06. This is a separate project, not directly relevant to harness evolution tasks.

---

## Prioritized Candidate Tasks

Based on evidence hierarchy (CI/state > dashboard > transcripts):

1. **[HIGH] Unblock cache metrics by filing help-wanted issue + exploring yyds-side workaround** — The cache-report deadlock is the clearest product gap. Either file the upstream issue and wait for human action, or implement a yyds-side workaround that extracts cache fields from the raw response. The workaround path is higher risk (duplicates parsing) but delivers results immediately.

2. **[MEDIUM] Reduce cancelled-session noise in FailureObserved** — Distinguish "cancelled by external signal" from "crashed with error" in append_terminal_state_events.py. This would clean up the failure signal and prevent the dashboard from flagging non-failures.

3. **[LOW] Investigate corrupted TestEvent in events.jsonl** — Find the source of the unknown variant and optionally clean it. Low urgency since the reader handles it gracefully.
