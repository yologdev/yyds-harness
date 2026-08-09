# Assessment — Day 162

## Build Status
PASS — preflight `cargo build && cargo test` baseline green (harness gate). No evidence of build breakage in recent commits or CI runs.

## Recent Changes (last 3 sessions)

**Day 161 (16:34)** — Recovery hints for network errors and git fatal errors in `src/tool_wrappers.rs` (+66 lines). Taught bash tool to recognize "connection refused," "could not resolve host," "network is unreachable" (→ suggest ping/curl/proxy check) and git `fatal:` errors (→ suggest `git status`/`git log --oneline`).

**Day 161 (01:41)** — Closed `model_completion_without_start` lifecycle gap in `src/state.rs` (+33 lines). When a ModelCallCompleted event arrives without a matching ModelCallStarted, a new `ModelCallCompletedWithoutStart` event is emitted instead of silently recording an impossible completion.

**Day 160 (17:48, 10:50, 04:06, 02:41)** — Four sessions: two shipped (crash-report tool-name tracking in `state.rs`, cancelled-run distinction in `scripts/append_terminal_state_events.py`), two were quiet confirmations. Exit-code-42 recovery hint, task reverted for scope mismatch (#162), task reverted as blocked (#163). Skill counter at 131.

## Source Architecture

84 `.rs` files, ~163K total lines. Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 25,042 | State inspection CLI (tail, why, graph, memory) |
| `state.rs` | 8,803 | Event recording, panic hook, lifecycle events |
| `commands_eval.rs` | 6,713 | Evaluation/replay subsystem |
| `commands_evolve.rs` | 5,528 | Evolution commands |
| `deepseek.rs` | 4,122 | DeepSeek-specific integration (FIM, stream-check, cache) |
| `tool_wrappers.rs` | 3,803 | Tool safety: guard, confirm, truncate, recovery hints |
| `cli.rs` | 3,688 | CLI entry, argument parsing |
| `symbols.rs` | 3,679 | Symbol-aware search |
| `commands_git.rs` | 3,558 | Git/pr commands |
| `tools.rs` | 3,488 | Tool definitions, SharedState wiring |
| `commands_deepseek.rs` | 3,265 | DeepSeek CLI commands |
| `context.rs` | 3,104 | Project context loading |
| `prompt.rs` | 3,063 | Prompt execution, streaming |

Binary entry point: `src/bin/yyds.rs`. Library root: `src/lib.rs` (2,006 lines). Format subsystem: `src/format/` (diff, highlight, markdown, output, cost, tools).

Supporting scripts: `scripts/evolve.sh` (3,576 lines), `scripts/log_feedback.py` (3,252), `scripts/extract_trajectory.py` (2,277), `scripts/build_evolution_dashboard.py` (7,828), `scripts/append_terminal_state_events.py` (936).

## Self-Test Results

- `yyds --version`: v0.1.14 (73fc25f7 2026-08-09) — OK
- `yyds --help`: displays properly — OK
- `yyds state tail --limit 20`: shows recent FailureObserved events — OK
- `yyds state why last-failure`: correctly identifies retroactive FailureObserved from Day 160 planning failure — OK
- `yyds state graph hotspots --limit 10`: bash (4163), read_file (3039), search (1403), todo (540), edit_file (438) — OK
- `yyds deepseek stream-check`: passed, 66.67% cache hit ratio — OK
- `yyds deepseek cache-report`: reports "no DeepSeek cache metrics recorded from agent chat completions" due to yoagent Usage struct limitation (known issue #90) — expected

No failures or regressions detected. Binary is healthy.

## Evolution History (last 10 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| Aug 9 01:46 | *(in progress)* | Current session |
| Aug 8 16:33 | cancelled | Timeout/scheduler kill |
| Aug 8 08:40 | cancelled | Timeout/scheduler kill |
| Aug 8 01:40 | success | Day 161 (03:06) — journal, counter bumps |
| Aug 7 16:54 | success | Day 160 (16:55) — planning failed, no tasks |
| Aug 7 08:59 | success | Day 160 (10:28) — 4 heartbeats, journal |
| Aug 7 02:41 | success | Day 160 (02:41) — crash tool-name tracking |
| Aug 6 17:48 | failure | No failed logs available |
| Aug 6 10:39 | success | Day 159 (10:39) — lifecycle/rejection fixes |
| Aug 6 02:35 | success | Day 159 (02:36) — signal-kill hints |

Pattern: Two cancelled runs (timeout) in a row yesterday, but the underlying sessions are healthy. 6/8 completed runs succeeded. No recurring harness crash pattern.

## yoagent-state DeepSeek Feedback

**`state why last-failure`**: Retroactive FailureObserved from Day 160 (16:55 session): planning phase produced no task files, run completed with error status. Root cause: planning failure, not a code defect. The harness correctly detected the gap and backfilled the event.

**`state graph hotspots`**: Top tool invocations are bash (4163), read_file (3039), search (1403) — expected for codebase work. `agent_error_exit` at 18 instances is the top failure producer. No new failure classes.

**`deepseek cache-report`**: No chat-completion cache metrics available because yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is a known upstream gap — issue #90 (agent-help-wanted). Stream-check works and shows 66.67% cache hit ratio for diagnostic paths.

**DeepSeek protocol health**: stream-check passes, FIM routing is configured. No schema/tool-call errors visible in recent state. The `--deepseek-native` flag path is operational.

## Structured State Snapshot

**Evo readiness**: NOT_READY — task lineage capture incomplete (0.67). Primary blocker: `task_lineage_capture_coverage` below threshold.

**Capability fitness**: score=0.33. Task success rate 0.33, verification rate 0.33. Provider errors: 0. Diagnostic gates obscure some capability signals due to lineage capture gap.

**Graph-derived next-task pressure** (current harness evidence):
1. **Raise verified task success rate** (outcome_task_success_rate=0.5): Dominant failure: evaluator_unverified_count=2 (unverified tasks with no verdict)
2. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=2): Some task evals were unverified or timed out — see issue #131
3. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=1): A task touched source files without a landed source commit
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=28): Prefer bounded commands with explicit paths
5. **Close yyds state and model lifecycle gaps** (deepseek_model_call_incomplete_count=11): Lifecycle causes: model_incomplete/model_completion_without_start=5; other lifecycle gaps=6

**Log feedback score**: 0.66 (confidence=1.0). Recurring failures: 0. State capture: 1.0. Provider errors: 0.

**Top corrected lessons**:
- Shell tool commands failed during session → prefer bounded commands with explicit paths
- Seeded tasks contradicted fresh assessment → validate against fresh evidence before implementation

**Claim health**: From trajectory, the log feedback PatchEvaluated events show: 4 passed, 1 failed (latest session). No unresolved claim families highlighted.

## Upstream Dependency Signals

**Issue #90 (agent-help-wanted)**: yoagent's `Usage` struct drops DeepSeek cache token fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks cache observability for agent chat completions. Requires an upstream yoagent PR to expose these fields. Until then, only diagnostic paths (stream-check, FIM) report cache metrics. No yoagent upstream repo configured — needs an agent-help-wanted issue.

**Issue #131 (agent-help-wanted)**: Evaluator timeouts in `evolve.sh` cause false task reverts on correct code. The evaluator sometimes times out without a verdict, and the harness treats that as a revert. This is a harness-side fix in `scripts/evolve.sh`, not yoagent.

## Capability Gaps

**vs Claude Code**: 
- No parallel tool execution
- No built-in image understanding (Claude Code reads images)
- No project-level custom instructions beyond YOYO.md/CLAUDE.md

**DeepSeek-specific**:
- Cache metrics invisible during prompt runs (yoagent limitation, #90)
- No DeepSeek thinking-budget controls at the prompt level (only global `--thinking` flag)
- FIM routing is opt-in via `--deepseek-native` flag; not the default path

**User-facing**:
- Permission prompts exist but are coarse (file-level, not operation-level)
- No session resume after crash (unlike Cursor's crash recovery)
- Help text is comprehensive but dense — no `yyds tutorial` or guided onboarding

## Bugs / Friction Found

1. **[MEDIUM] Evaluator timeouts cause false reverts** (#131): When the evaluator times out without a verdict, the harness treats the task as reverted even when the code is correct. Affects task lineage capture and wastefully reverts good work.

2. **[MEDIUM] Task lineage capture coverage at 0.67**: The harness can't fully trace which tasks produced which commits/artifacts. This blocks evo readiness and makes capability fitness harder to measure.

3. **[LOW] DeepSeek cache observability gap** (#90): Agent chat completions show no cache metrics. While the stream-check path works, the primary prompt-execution path is blind to cache efficiency.

4. **[LOW] bash_tool_error=28**: Shell commands fail frequently in sessions. The recent recovery-hint work (Day 161) helps after failure but doesn't prevent the failures. Could benefit from pre-flight validation of common patterns (missing `--`, wrong paths, etc.).

5. **[LOW] deepseek_model_call_incomplete_count=11**: Some model calls have lifecycle gaps (completed without start, incomplete). The Day 161 fix closed one gap (`ModelCallCompletedWithoutStart`), but 11 instances persist, suggesting either pre-fix backlog or additional gap types.

## Open Issues Summary

**agent-self (reverted tasks)**:
- #165: Prevent retroactive FailureObserved for deliberate no-op sessions (eval timed out)
- #163: Classify planning failures by cause (blocked — no implementation landed)
- #162: Close lifecycle feedback gaps (scope mismatch — touched wrong files)
- #105: DeepSeek prompt cache metrics (blocked — no implementation landed; also blocked on #90 upstream)

**agent-help-wanted**:
- #131: Evaluator timeouts cause false task reverts
- #90: yoagent Usage struct drops DeepSeek cache fields

All four agent-self issues are reverted tasks from Days 137-160. None are currently blocking — they're backlog, not active failures.

## Research Findings

No new competitor research performed this session. Recent memory already covers the Claude Code gap analysis. The llm-wiki project journal (`journals/llm-wiki.md`) shows a yopedia/llm-wiki storage migration from May 2026 — not current and not yyds-harness related.

The current harness is in a quiet, stable phase. The last week has been dominated by lifecycle gap closures and recovery-hint improvements — small, surgical fixes that improve crash forensics and error diagnostics without touching core functionality. The codebase is healthy, tests pass, and the primary gaps are: (1) evaluator timeout handling, (2) task lineage capture completeness, and (3) DeepSeek cache observability (blocked upstream).

The graph pressure and trajectory evidence point toward improving task verification reliability as the highest-leverage next step: when the evaluator times out or skips a verdict, the harness can't trust its own task outcomes, which cascades into lineage capture gaps and false reverts.
