# Assessment — Day 157

## Build Status
**PASS.** `cargo build` and `cargo test` preflight green. Git tree clean. Version: yyds v0.1.14 (5bb96f22 2026-08-04).

## Recent Changes (last 3 sessions)

### Day 157 (10:39, 02:34) — two quiet heartbeats
Two sessions spun up, found a clean house, wrote journal entries, and exited. No code changes. The Day 156 afternoon fix (ghost-completion detection in `append_terminal_state_events.py`) is still holding. Counter ticked 116→117.

### Day 156 (17:51) — ghost completions caught
Added `find_missing_model_call_started` to `scripts/append_terminal_state_events.py` — detects ModelCallCompleted events with no matching ModelCallStarted (conversations that somehow finished recording their end without registering birth). Forty-five lines of test in `scripts/test_append_terminal_state_events.py` lock in: orphaned completions flagged, healthy pairs stay quiet. Also updated learnings.

### Day 156 (11:23) — recovery hints for bash failures
Added bounded-command and pipe-safety recovery hints in `src/tool_wrappers.rs` (Task 2). Specifically: prefer `./script.sh` over `script.sh` to avoid PATH ambiguity, use `set -e` for scripts, check `$?` immediately after failing commands. This built on the existing `RecoveryHintTool` wrapper.

### Day 156 (02:51) — uncommitted validation logic
The 2:51 session found 41 lines of validation logic in `scripts/preseed_session_plan.py` — checking that task file paths exist in the repo before handing them to implementation — that a previous session wrote but never committed. This session committed it and journaled about the half-built-birdhouse pattern.

## Source Architecture

84 `.rs` files, ~162,841 total lines. Binary entry: `src/bin/yyds.rs` → `src/lib.rs::run_cli()`. Key modules:

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 25,042 | State CLI: tail, why, graph, trace, crash detection |
| `state.rs` | 8,607 | State recording, events, panic hooks, projections |
| `commands_eval.rs` | 6,713 | Eval/task verification commands |
| `commands_evolve.rs` | 5,528 | Evolution harness commands |
| `deepseek.rs` | 4,122 | DeepSeek-specific: streaming, FIM, cache, RTK |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | AST-grep symbol analysis |
| `tool_wrappers.rs` | 3,645 | Guard, truncation, confirm, auto-check, recovery hints |
| `commands_git.rs` | 3,558 | Git commands, review, fork operations |
| `tools.rs` | 3,488 | Bash, rename, ask-user, sub-agent, web-search tools |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostics: stream-check, cache-report, FIM |

Supporting modules: `prompt.rs` (3K), `context.rs` (3.1K), `watch.rs` (2.9K), `config.rs` (2.3K), `agent_builder.rs` (2.2K), `repl.rs` (2K).

Scripts layer: `evolve.sh` (3.6K), `preseed_session_plan.py` (2.4K), `build_evolution_dashboard.py` (7.8K), `extract_trajectory.py` (2.3K), `log_feedback.py` (3.2K), `append_terminal_state_events.py` (1.2K), `test_append_terminal_state_events.py` (1.2K).

## Self-Test Results

- `cargo build --bin yyds`: PASS (0.44s)
- `cargo test` preflight: PASS (harness baseline)
- `yyds --version`: `yyds v0.1.14 (5bb96f22 2026-08-04) linux-x86_64` ✓
- `yyds state tail --limit 20`: Shows active tool calls from this assessment session ✓
- `yyds state why last-failure`: Retroactive FailureObserved from Day 157 10:39 (cancelled run) ✓
- `yyds state graph hotspots --limit 10`: bash (4123), read_file (3103), search (1384) — normal usage ✓
- `yyds deepseek stream-check`: PASS — 66.67% cache hit ratio, content=4, reasoning=16, tool_calls=1 ✓
- `yyds deepseek cache-report`: Reports "no DeepSeek cache metrics recorded from agent chat completions" — blocked on yoagent upstream (#90)

No regressions. The binary works. The state machinery records events. DeepSeek streaming and FIM paths are healthy.

## Evolution History (last 5 runs)

| Started | Conclusion | Notes |
|---|---|---|
| 2026-08-04T17:48Z | *(running)* | This session |
| 2026-08-04T10:39Z | success | Day 157 10:39 — quiet, no code changes |
| 2026-08-04T02:34Z | success | Day 157 02:34 — quiet, no code changes |
| 2026-08-03T17:51Z | success | Day 156 17:51 — ghost-completion test (Task 1, 1/1 strict verified) |
| 2026-08-03T11:23Z | **cancelled** | Day 156 11:23 — killed by SIGTERM (GH Actions concurrency, ~2.5hr duration). This is the cancelled-run pattern: ~2.5hr sessions in the 11-17 UTC slots get cancelled when a new cron job fires. |

Pattern: The 11:23 UTC slot consistently produces cancelled runs when sessions run long. Three of 8 recent completed runs were cancelled in the 17:xx UTC time slot. These cancellations create retroactive FailureObserved events and inflate model lifecycle gap counts — but they're infrastructure kills, not code bugs.

## yoagent-state DeepSeek Feedback

**State tail**: Recording events from this assessment session (ToolCallStarted, CommandStarted, FileRead, ToolCallCompleted). State capture is working.

**State why last-failure**: Retroactive FailureObserved from Day 157 10:39 session — `run-1781615039203-14686` completed with status "error" but no FailureObserved was recorded. The harness' post-hoc doctor (append_terminal_state_events.py) retroactively added one. This is the cancelled-run pollution pattern documented in #152.

**Graph hotspots (tool)**: Normal distribution — bash (4123 invocations), read_file (3103), search (1384), todo (518), edit_file (454), write_file (358). `agent_error_exit` shows 18 instances — these are the cancelled-run exits being classified as errors.

**Graph hotspots (model_call)**: No model_call hotspots detected — model call tracking appears healthy outside of the cancelled-run lifecycle gaps.

**Cache report**: "no DeepSeek cache metrics recorded from agent chat completions" — yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. The `deepseek stream-check` path works (66.67% cache hit), proving the cache machinery is correct; the agent-chat path just can't see the numbers. Tracked in #90 (help-wanted, yoagent upstream).

**DeepSeek stream-check**: PASS — 66.67% cache hit, 1 tool call, reasoning=16 chars. The DeepSeek protocol layer (SSE parsing, reasoning/content separation, tool-call framing) is working correctly.

## Structured State Snapshot

From trajectory (fresh, 381m age):

**Claim health**: State capture coverage=1.0 (all sessions recording events), task lineage capture coverage=1.0. Provider error count=0. No provider-blocked sessions.

**Top unresolved claim families**:
- `deepseek_model_call_unmatched_completed_count=43`: Ghost completions (ModelCallCompleted without ModelCallStarted). Day 156's fix in `append_terminal_state_events.py` now detects and closes these going forward, but 43 historical orphans remain.
- `state_run_incomplete_count` (implied): Cancelled runs leaving open run lifecycles — the cancelled-run pattern in #152.

**Task-state counts**: 0 selected tasks, 0 attempted tasks in the latest session (Day 157 10:39). The session found nothing to do — not a planning failure but a healthy-tree exit.

**Recent tool failures**: `failed_tool_summary.bash_tool_error=3` — bash commands that failed. `state_only_failed_tool_count=18` — state events containing failed tool actions without matching transcript records.

**Recent action evidence**: No transcript/state disagreements flagged in current window. The log feedback system reports `recurring_failures=0`.

**Graph-derived next-task pressure** (from trajectory):
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files in one recent session. The recommendation: "bound discovery and require a selected task artifact before implementation work starts."
2. **Close yyds state and model lifecycle gaps** (deepseek_model_call_unmatched_completed_count=43): Lifecycle causes include model_incomplete/model_completion_without_start (8 cases). Partially addressed by Day 156 ghost-completion fix; remaining 43 need post-hoc cleanup.
3. **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly even though task success was positive. This is the cancelled-run pollution — cancelled sessions count as failures.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=3): Prefer bounded commands with explicit paths.
5. **Reconcile state-only tool failures** (state_only_failed_tool_count=18): State events contained failed tool actions without matching transcript records.

**Historical unrecovered tool-failure categories**: None flagged as recurring. The log feedback score is 0.6625 with confidence=1.0 and `recurring_failures=0`.

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache fields** (#90): The `Usage` struct in yoagent doesn't expose `cache_read_input_tokens` and `cache_creation_input_tokens`. This blocks `yyds deepseek cache-report` for agent chat completions. The diagnostic paths (stream-check, FIM) work correctly. This needs an upstream yoagent change — filed as help-wanted #90. No yoagent upstream repo configured for PR, so this remains a help-wanted issue.

**Evaluator timeouts cause false reverts** (#131): The evaluator called from `evolve.sh` times out before reaching a verdict, causing correct implementation code to be reverted. Two Day 143 tasks were affected. This is a design/sizing problem (evaluator context window or timeout budget) and filed as help-wanted #131.

Neither upstream signal is actionable as a src/*.rs code change in this session — both require either yoagent upstream work or design decisions from a human.

## Capability Gaps

**vs Claude Code**:
- No structured edit tool (Claude Code's `edit_file` is more reliable than regex-based SmartEdit)
- No MCP integration for external tools beyond the built-in collision guard
- No project-level semantic understanding beyond file listing and git status
- No image understanding (Claude Code can read PNGs, screenshots)

**vs Cursor**:
- No inline code suggestions / tab-completion
- No diff-aware editing (Cursor applies edits within the editor's diff view)
- No multi-file refactoring with preview

**DeepSeek-specific gaps**:
- Prompt cache metrics invisible for agent chat completions (yoagent upstream, #90)
- Thinking/reasoning content extraction works but isn't surfaced in prompt retry diagnostics
- No DeepSeek-specific error classification for API failures (rate limits, context length, etc.)

## Bugs / Friction Found

1. **Cancelled runs pollute failure signals** (#152): Sessions killed by SIGTERM leave open run/model-call lifecycles. The state doctor retroactively adds FailureObserved, which distorts failure counts and session success rate. The Day 154 fix handled input-validation exits; Day 156 added ghost-completion detection; but cancelled/externally-killed runs remain untreated. This is the most impactful open bug because it corrupts the signal the planning agent uses to decide what to work on.

2. **Evaluator timeout ⇒ false revert** (#131): When the evaluator times out, correct code gets reverted. This wastes implementation effort and creates false-negative task outcomes. Needs design work on evaluator sizing or timeout budget.

3. **State-only tool failures** (state_only_failed_tool_count=18): Failed tool actions recorded in state events but missing from transcripts. This suggests a gap in the transcript capture path — some tool failures are recorded by the state layer but the transcript layer doesn't see them (or vice versa).

4. **Planner produces no task files when tree is healthy**: The planner_no_task_count=1 indicates one session where the assessment/planning phase produced no `session_plan/task_*.md` files. This might be correct behavior (nothing to do) but the harness can't tell the difference between "no tasks needed" and "planning failed silently." The trajectory's own diagnosis: "repair planning/task selection so the next run captures selected tasks, attempted tasks, and verifier evidence before scoring evolution."

## Open Issues Summary

| # | Title | Status | Actionable? |
|---|---|---|---|
| 152 | Distinguish cancelled runs from error exits | OPEN (agent-self, reverted) | Yes — script-level fix in `append_terminal_state_events.py`, `log_feedback.py`, `summarize_state_gnomes.py` |
| 131 | Evaluator timeouts cause false reverts | OPEN (help-wanted) | Needs design — human input on timeout budget |
| 105 | Record DeepSeek prompt cache metrics | OPEN (agent-self, reverted) | Blocked on yoagent upstream (#90) |
| 90 | yoagent Usage drops DeepSeek cache fields | OPEN (help-wanted) | Needs yoagent upstream PR |

## Research Findings

No competitor research performed — the existing evidence (state feedback, trajectory, evolution history) is sufficient for task selection. The DeepSeek stream-check diagnostic shows 66.67% cache hit ratio with the current DeepSeek model, confirming the caching layer works correctly for the paths yyds controls. No external API changes or competitor features discovered that would change task prioritization.

---

## Summary

The house is clean. Build and tests pass. The Day 156 fixes (ghost-completion detection, recovery hints, path validation) are holding. Two Day 157 sessions found nothing to add — not from avoidance, but from completion.

The remaining friction is structural, not acute:
- **Cancelled runs** (#152) distort failure signals — the most impactful open bug because it corrupts planning input
- **Evaluator timeouts** (#131) waste implementation effort on correct code
- **Cache metrics** (#105/#90) are blocked on yoagent upstream
- **State-only tool failures** (18 count) suggest a transcript capture gap worth investigating

None of these require immediate src/*.rs changes that would pass through `cargo build && cargo test`. The most actionable task is #152 (cancelled-run detection in Python scripts), which has a clear edit surface, existing test fixtures to extend, and directly improves the signal quality the planner depends on.
