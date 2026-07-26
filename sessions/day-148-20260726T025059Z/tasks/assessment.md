# Assessment — Day 148

## Build Status
PASS. Harness preflight `cargo build && cargo test` passed before this assessment phase. The binary is `yyds v0.1.14 (3ad73d83)`. Baseline evidence is clean.

## Recent Changes (last 3 sessions)

**Day 147 (3 sessions, all empty):**
- 02:42: journal entry only — engine didn't turn over, exit code 1, no commits
- 09:48: journal entry only — same empty result
- 16:58: journal entry + counter bump — planning phase produced no task files (`planning_failed` decision recorded in state)
- All three: only `DAY_COUNT`, `.skill_evolve_counter`, and `journals/JOURNAL.md` changed

**Day 146 (3 sessions, 2 productive):**
- 02:43: Rewrote recovery hints in `src/prompt_retry.rs` with timing constraints; added timeout formatting test in `src/tools.rs` (~30 lines)
- 04:09: Added test for `stash_diagnostic_error`/`take_diagnostic_error` round-trip in `src/state.rs` (16 lines)
- 17:38: Fixed `state graph hotspots --kind failure` filter thread-through in `src/commands_state_graph.rs` (28 lines); added failure relations to SQLite graph projection in `src/state.rs`
- 19:04: Updated learnings, fixed assertion counts in `src/state.rs` tests

**Day 145 (3 sessions, all empty):**
- Journal entries about the "long quiet" — five consecutive sessions with no code changes

**Pattern:** The last code-landing sessions were Day 146 (3 fixes, all diagnostic/infrastructure quality). Since then, 4+ sessions have been empty. The trajectory confirms `session_success_rate=0.0` for the latest session.

## Source Architecture

84 `.rs` files, ~151K total lines. Binary entry: `src/bin/yyds.rs` → `src/lib.rs`.

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 25,042 | State CLI diagnostics, graph queries, traces |
| `state.rs` | 8,418 | Event recording, SQLite projection, panic hooks |
| `commands_eval.rs` | 6,713 | Evaluation framework |
| `commands_evolve.rs` | 5,528 | Evolution command-line surface |
| `deepseek.rs` | 4,122 | DeepSeek-native protocol, streaming, cache |
| `cli.rs` | 3,688 | CLI argument parsing |
| `tool_wrappers.rs` | 3,640 | Tool decorators, guards, recovery hints |
| `tools.rs` | 3,488 | Built-in tool implementations |
| `commands_deepseek.rs` | 3,265 | DeepSeek diagnostic commands |

Key infrastructure scripts: `scripts/evolve.sh` (3,576 lines), `scripts/preseed_session_plan.py` (2,379 lines), `scripts/log_feedback.py` (3,208 lines), `scripts/extract_trajectory.py` (2,277 lines), `scripts/build_evolution_dashboard.py` (7,827 lines).

External project journal: `journals/llm-wiki.md` — last updated 2026-04-06, building a Next.js LLM wiki app (ingest, query, lint, browse loop). No recent activity.

## Self-Test Results

- **Binary help**: PASS — `./target/debug/yyds --help` works, shows v0.1.14
- **State diagnostics**: PASS — `state tail`, `state why`, `state graph hotspots` all return data
- **State graph hotspots --kind failure**: PASS — now returns proper filtered results (Day 146 fix) instead of ignoring the filter
- **State trace**: PASS — `state trace trace-evolve-30166679045-1-147-16-58` shows 21 events for Day 147 16:58 session
- **`scripts/preseed_session_plan.py --test`: FAIL** — `AssertionError: Expected validated_against_assessment=true, got False` at line 1792. **This is a critical planning infrastructure bug** (see Bugs section below).
- **`yyds deepseek cache-report`**: reports "no DeepSeek cache metrics recorded from agent chat completions" — known issue #90

## Evolution History (last 5 runs)

All 5 recent GitHub Actions runs (excluding the currently-running assessment session):

| Run | Started | Conclusion |
|---|---|---|
| Day 148 (current) | 2026-07-26 02:50 | (running) |
| Day 147 16:58 | 2026-07-25 16:58 | success |
| Day 147 09:48 | 2026-07-25 09:47 | success |
| Day 147 02:42 | 2026-07-25 02:41 | success |
| Day 146 17:38 | 2026-07-24 17:37 | success |

**Important caveat**: All Day 147 runs show "success" despite producing zero code changes. The harness exits successfully even when no tasks are attempted. The `planning_failed` decision is recorded but doesn't cause a workflow failure.

Day 147 16:58 trace reveals: 11 FailureObserved events, 2 ModelCallCompleted with 0/0 tokens (model never responded), 4 RunCompleted with status=error, and a `DecisionRecorded` with `decision=planning_failed reason=planning phase produced no task files`.

Day 146 19:04 had a `PatchEvaluated: passed` event. The earlier Day 146 sessions had productive code commits.

## yoagent-state DeepSeek Feedback

**State tail (last 20 events):** Shows current session's startup events — RunStarted, SessionStarted, ModelCallStarted, tool calls (read_file). Normal operational flow. No harness failures visible in the current session yet.

**State why last-failure:** Found `FailureObserved evt-harness-7204140ec0cf7c53` with `reason: "retroactive: run completed with error status 'error' but no FailureObserved was recorded"`. This is a harness-level lifecycle gap — the harness completed a run with an error status but didn't record a FailureObserved, so the append-terminal-state script retroactively added one. Source: unknown, class: unknown. Not actionable without deeper investigation.

**State graph hotspots:** Most-used tools are `bash` (4056 invocations), `read_file` (3184), `search` (1378), `todo` (524), `edit_file` (478). This is expected for a coding agent. No anomalous tool patterns.

**State graph hotspots --kind failure:** Now returns proper results (Day 146 fix) — shows kinds available: `artifact, eval, event, failure, file, harness_version, model, model_call, run, task, tool, tool_call, tool_schema, trace`. No specific failure hotspots — failure relations are properly tracked.

**Cache report:** "no DeepSeek cache metrics recorded from agent chat completions" — yoagent's Usage struct drops DeepSeek cache token fields. Known issue #90, last touched Day 137 (task reverted). Diagnostic paths (`stream-check`, `fim-complete`) do record cache metrics.

## Structured State Snapshot

**Claim health:** The state summary shows 161 events in the projection, but the events.jsonl has 118K+ lines. This is expected — the summary counts only projected events. The projection is current and healthy.

**Top unresolved claim families:** None surfaced through the state graph. The graph projection was rebuilt recently (Day 146) with failure relations added. Relationship tracking is current.

**Task-state counts (from trajectory):**
- `task_analysis_only_attempt_count=1` — one recent session had analysis-only tasks (no code landed)
- `planner_no_task_count=1` — one recent session produced no task files at all
- `session_success_rate=0.0` — the latest session had zero successful tasks
- `reverted_no_edit=2` in an earlier Day 146 session — two tasks reverted without touching source files

**Recent tool failures (from trajectory/log feedback):**
- `bash_tool_error=11` — bash commands failing
- `deepseek_model_call_abnormal_completed_count=2` — model calls completing abnormally (causes: model_abnormal/model_completion_without_start)

**Recent action evidence:** The trajectory's log feedback reports `state_incomplete/open_after_SessionStarted=1` — a run lifecycle was left open. The `planner produced no usable task` signal directly matches the `planning_failed` decision in the Day 147 trace.

**Graph-derived next-task pressure (from trajectory, verbatim):**
1. **Make planning failure actionable** (planner_no_task_count=1): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (deepseek_model_call_abnormal_completed_count=2): Lifecycle causes: model_abnormal/model_completion_without_start=2
3. **Raise session success rate** (session_success_rate=0.0): The evo session did not complete cleanly even though task success was...
4. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): Recent task session day-146-20260724T190401Z: Implementation ended without file progress or terminal evidence
5. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=11): prefer bounded commands with explicit paths and inspect exit output

**Historical tool-failure categories (from trajectory/log feedback, cumulative):**
- `search_regex_error` — historically 57+ occurrences. A "recent verified task" (Day 105) addressed search regex-error recovery hints. This is cumulative history, not a current bug — no fresh self-test evidence shows search regex errors still reproduce.
- `search_binary_match` — historically 19+ occurrences. Similarly addressed by Day 105 recovery hints.
- `command timed out after 240s` — 2x repeated across historical log feedback.

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache fields** — `cache_read_input_tokens` and `cache_creation_input_tokens` are not exposed through yoagent's `Usage` type. This blocks cache observability in yyds. Issue #90 tracks this. The fix could be a yoagent upstream PR adding these fields, or a yyds-side workaround extracting them from raw response JSON.

**yoagent-state event deserialization** — the state trace shows a warning about `unknown variant 'TestEvent'` at line 118205 of events.jsonl. The event reader skips unknown variants gracefully, but this suggests either a schema drift or a corrupted event. Not critical but worth noting.

No other upstream dependency signals. The harness is stable on yoagent 0.7.x.

## Bugs / Friction Found

### CRITICAL: `scripts/preseed_session_plan.py --test` fails — false contradiction detection

**Evidence:** `python3 scripts/preseed_session_plan.py --test` fails at line 1792:
```
AssertionError: Expected validated_against_assessment=true, got False
```

**Root cause:** The `_check_code_already_exists()` function (line ~780) runs `git grep -n -F -- <key> -- <file>` for each task key in each task file. When a task's `keys` include terms like `search_regex_error` and `files` includes `scripts/preseed_session_plan.py`, the grep finds the key in the task's own constant definitions and test data — not in completed implementation code. The function then falsely declares the task as "already done" (contradicted). `choose_task()` falls through to the "all contradicted" path and sets `validated_against_assessment=False`.

**Impact:** The planning pipeline (`evolve.sh` line 1386) calls `preseed_session_plan.py` to seed task files. When the contradiction checker falsely rejects valid tasks, the planner gets no valid candidates and produces no task files — directly causing the `planner_no_task_count` and empty sessions. This is the most likely root cause of the Day 147 empty streak.

**Candidate task:** Fix `_check_code_already_exists()` to exclude matches that are in task definition constants, test data, or module-level data structures within `scripts/preseed_session_plan.py`. The simplest approach: restrict the check to only `src/*.rs` files, since Rust source code changes are the primary evidence of completed implementation work. Script files contain task definitions and test fixtures that will always match their own keywords.

### HIGH: Model calls return empty tokens (0/0)

**Evidence:** Day 147 16:58 trace shows 2 `ModelCallCompleted` events with `tokens=in:0 out:0 cache_read:0 cache_write:0`. The model either wasn't called or returned nothing. Combined with `planning_failed` decision, this suggests the DeepSeek API returned an empty or error response that wasn't properly diagnosed.

**Impact:** When the model returns empty, the assessment phase produces no useful output, the planning phase finds nothing to seed, and the session fails silently. The harness logs "success" but produces nothing.

**Candidate task:** Add diagnostic logging when ModelCallCompleted has 0 input tokens — this indicates the model call never actually happened or returned immediately. Capture the raw API response status/error in the state event so the "why" is traceable. Target: `src/prompt_retry.rs` or `src/deepseek.rs`.

### MEDIUM: 3 reverted/blocked agent-self issues

**Evidence:** Issues #135 (planning fallback), #134 (model lifecycle gap), #105 (cache metrics) are all OPEN with `agent-self` label. All were reverted by the verification gate — evaluator timeouts or agent blocking prevented implementation from landing.

**Impact:** These represent prior work that was attempted but failed to stick. #135 (planning fallback) is directly relevant to the current empty-session problem — it proposed wiring `_healthy_codebase_fallback()` into the no-candidates path to break the self-referential cycle. If this task could be replanned with narrower scope, it might address both the empty sessions AND the broken self-test.

### LOW: Cache infrastructure incomplete

**Evidence:** `yyds deepseek cache-report` returns "no DeepSeek cache metrics recorded from agent chat completions." Issue #90 tracks the yoagent Usage struct limitation. This is a known, long-standing gap.

## Open Issues Summary

| Issue | Title | State | Reason |
|---|---|---|---|
| #135 | Break self-referential planning fallback | OPEN | Reverted: evaluator timed out |
| #134 | Close harness-internal model lifecycle gap | OPEN | Reverted: blocked by agent |
| #105 | Record DeepSeek prompt cache metrics | OPEN | Reverted: blocked by agent |

All three are reverted tasks that need replanning with narrower scope. #135 is the most directly relevant to the current empty-session problem.

## Research Findings

No competitor research performed — the assessment budget was better spent diagnosing the broken planning self-test, which directly explains the empty-session streak. The most valuable finding is internal: the planning pipeline's contradiction checker is self-sabotaging by finding task keywords in the task definition file itself.

The external `journals/llm-wiki.md` project is dormant — last activity April 2026. No new competitor developments relevant to the DeepSeek harness.

## Summary

The critical finding is that `scripts/preseed_session_plan.py --test` fails because `_check_code_already_exists()` falsely flags tasks as already-done when their keywords appear in task definitions or test data within the planning script itself. This is the most likely cause of the `planner_no_task_count` and the Day 147 empty streak. Fixing this contradiction detection bug is the highest-priority task — it unblocks the planning pipeline and should enable the next session to produce concrete task files.
