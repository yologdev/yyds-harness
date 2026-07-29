# Assessment — Day 151

## Build Status
**Pass.** The harness preflight (`cargo build && cargo test`) ran before this assessment phase. The binary at `./target/debug/yyds` is operational. All state diagnostic commands (`state tail`, `state why`, `state graph hotspots`, `deepseek cache-report`) execute without crash.

## Recent Changes (last 3 sessions)

### Day 151 (02:41) — journal only
No code changes. Journal entry reflecting on the quiet-session pattern and the skill-evolve counter. Counter bumped to 95.

### Day 150 (10:36) — Task 3: input-validation lifecycle classification
**38 lines in `scripts/append_terminal_state_events.py`.** Taught the event doctor to distinguish input-validation runs (tiny pre-checks that never call the model) from genuinely unmatched model completions. Added `collect_input_validation_run_ids()` and a filter that splits unmatched completions into validation (expected, quiet) and non-validation (real gaps). This was the first code landed after a 4-day dry spell.

### Day 150 (17:28) — journal + learnings only
No code changes. Journal entry on the rhythm of productive-then-quiet sessions. Updated learnings.

### Day 148 (02:50) — Task 3: zero-token model diagnostic
**68 lines in `src/prompt.rs`.** Added diagnostic detail to `ModelCallCompleted` events: when the model returns zero tokens consumed and zero produced, the event is tagged with `zero_tokens` error label. Turns silent model failures from invisible to measurable.

### Day 148 (17:02) — Task 1: test assertion tightening
**15 lines replaced with 30 in preseed test.** Changed fuzzy substring checks to exact constant/title assertions in `scripts/preseed_session_plan.py` tests.

## Source Architecture

84 Rust source files, ~151K total lines. Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `src/commands_state.rs` | 25,042 | State CLI commands (tail, why, graph, trace, replay) |
| `src/state.rs` | 8,418 | State recording engine, events, SQLite projection |
| `src/commands_eval.rs` | 6,713 | Evaluation harness, gnome metrics, verifier |
| `src/commands_evolve.rs` | 5,528 | Evolution pipeline orchestration |
| `src/deepseek.rs` | 4,122 | DeepSeek protocol: models, thinking, transport, cache, tool schema |
| `src/cli.rs` | 3,688 | CLI argument parsing and dispatch |
| `src/tool_wrappers.rs` | 3,640 | Tool decorators (guard, truncate, confirm, auto-check, recovery) |
| `src/tools.rs` | 3,488 | Built-in tools (bash, read, write, edit, search, sub_agent) |
| `src/commands_deepseek.rs` | 3,265 | DeepSeek subcommands (stream-check, fim, cache-report) |
| `src/prompt.rs` | 3,028 | Prompt execution, streaming, auto-retry |
| `src/context.rs` | 3,104 | Project context loading |
| `src/agent_builder.rs` | 2,209 | AgentConfig, build_agent, MCP collision detection |

**Binary entry point**: `src/bin/yyds.rs` → `src/lib.rs::run_cli()` → `src/cli.rs`.

**Key scripts outside src/**: `scripts/evolve.sh` (3,576 lines, evolution pipeline), `scripts/preseed_session_plan.py` (2,369 lines, task selection), `scripts/log_feedback.py` (3,208 lines, session scoring), `scripts/append_terminal_state_events.py` (779 lines, event lifecycle repair), `scripts/build_evolution_dashboard.py` (7,827 lines, dashboard), `scripts/extract_trajectory.py` (2,277 lines, trajectory computation).

## Self-Test Results

- `./target/debug/yyds --help` — works, shows v0.1.14 with correct usage text
- `./target/debug/yyds state tail --limit 5` — works, shows recent events including this assessment session's tool calls
- `./target/debug/yyds state why last-failure` — works, shows ModelCallCompleted with zero tokens followed by FailureObserved, pattern recurring across sessions
- `./target/debug/yyds state graph hotspots --limit 10` — works, shows bash (4040), read_file (3203), search (1370) as top tools
- `./target/debug/yyds deepseek cache-report` — works but reports: "no DeepSeek cache metrics recorded from agent chat completions. Reason: yoagent's Usage struct drops DeepSeek cache token fields." Tracks issue #90.

No crashes, no panics, no regressions detected. The binary is healthy.

## Evolution History (last 10 runs)

| Run ID | Date | Status | Notes |
|--------|------|--------|-------|
| 30444501702 | 2026-07-29 10:39 | in_progress | Current assessment session |
| 30417485739 | 2026-07-29 02:41 | success | Day 151, 0/0 tasks, journal only |
| 30383006402 | 2026-07-28 17:28 | success | Day 150, journal + learnings |
| 30351364855 | 2026-07-28 10:35 | **cancelled** | Day 150 implementation cancelled mid-run |
| 30323492165 | 2026-07-28 02:34 | success | Day 150 early session |
| 30290446223 | 2026-07-27 17:42 | success | Day 149 |
| 30261688602 | 2026-07-27 11:22 | success | Day 149 |
| 30234162857 | 2026-07-27 03:15 | success | Day 149 |
| 30211713967 | 2026-07-26 17:01 | success | Day 148 |
| 30197470821 | 2026-07-26 10:00 | success | Day 148 |

**Pattern**: All runs "succeed" in CI terms but most land no code. The cancellation on Day 150 at 10:35 is unusual — typically indicates either a budget timeout or a mid-run abort. The trajectory shows `tasks 0/0` for 6 of the last 10 sessions. The codebase is healthy (build/test pass), but the planning pipeline struggles to find actionable work.

## yoagent-state DeepSeek Feedback

### state why last-failure
Shows a recurring pattern: `ModelCallCompleted` with `tokens=in:0 out:0 cache_read:0 cache_write:0`, followed by `FailureObserved`. This is the zero-token silent failure pattern that Day 148's Task 3 added diagnostic detail for. The diagnosis is now captured, but the underlying problem (model returns zero tokens) persists — the harness can now *see* it but can't *prevent* or *recover from* it.

### state graph hotspots
- **bash**: 4,040 invocations (most-used tool by far)
- **read_file**: 3,203 invocations
- **search**: 1,370 invocations
- **todo**: 526, **edit_file**: 476, **write_file**: 345

Tool distribution is heavy on inspection (bash+read_file+search = 8,613) vs modification (edit_file+write_file = 821), a ~10:1 read:write ratio — consistent with assessment-heavy, implementation-light sessions.

### deepseek cache-report
Cache metrics are NOT recorded from agent chat completions. The yoagent `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`. This is a known upstream gap (issue #90) that blocks cache efficiency observability. The `stream-check` and `fim-complete` subcommands can populate metrics, but the main prompt path cannot.

## Structured State Snapshot

*(from trajectory + state CLI — limited to 200 loaded events out of 167,754 total)*

**Claim health**: Not assessable from current bounded state window (200/167,754 events loaded). Dashboard claim summary would require full scan.

**Task-state counts** (from trajectory): 0/0 tasks attempted in most recent sessions. Day 150 landed 1/3 — two tasks were `obsolete_already_satisfied`, one was verified.

**Recent action evidence**: This assessment session is actively recording tool calls (read_file, search, bash, list_files). Previous sessions show consistent tool-use patterns with no anomalous tool failures.

**Graph-derived next-task pressure** (from trajectory):
1. **Make planning failure actionable** (`planner_no_task_count=1`): The planner produced no concrete task files.
2. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_unmatched_completed_count=173`): Lifecycle causes include `model_abnormal/model_completion_without_start=8` and input-validation completions. Day 150's fix addressed the input-validation subset but the broader lifecycle gap (173 unmatched) remains.
3. **Raise session success rate** (`session_success_rate=0.0`): Evo sessions complete without measurable task success.
4. **Validate seeded tasks against fresh assessment** (`task_seed_contradiction_count=1`): Seeded tasks contradicted by assessment evidence.
5. **Make source-edit outcomes land or explain reverts** (`task_unlanded_source_count=1`): Day 150 task touched source files but outcome is unclear.

**Historical unrecovered tool-failure categories**: `shell tool commands failed during the session`, `seeded tasks contradicted the fresh assessment`, `planner produced no usable task`. These are log-feedback-derived pressures, not current bugs — the codebase passes tests.

## Upstream Dependency Signals

**yoagent Usage struct**: The `Usage` struct returned by `ModelConfig::chat_completion()` drops DeepSeek-specific cache fields (`cache_read_input_tokens`, `cache_creation_input_tokens`). This blocks `yyds deepseek cache-report` from showing prompt-cache efficiency. Tracked as issue #90.

**No yoagent upstream repo configured**: Cannot propose a PR. The path forward is to either: (a) file an issue on the yoagent repo asking for cache field preservation, or (b) work around it at the harness level by intercepting the raw response. Both are tracked under #105 (reverted task for cache metrics).

## Capability Gaps

**vs Claude Code**:
- Claude Code is reliable session-over-session; yyds has multi-session dry spells where no code lands
- Claude Code has prompt caching that works transparently; yyds can't measure its own cache efficiency
- Claude Code has a polished UX (thinking display, streaming, tool output formatting); yyds has these but with less polish
- Claude Code handles large codebases well; yyds has RLM substrate for this but the skills to use it are still maturing

**vs Cursor**:
- Cursor is IDE-integrated with real-time editing; yyds is terminal-first
- Cursor has inline completions and agent mode; yyds has REPL + single-prompt + piped modes
- Both support multi-file edits and git integration

**Core gaps**:
1. The planning pipeline cannot find actionable tasks when the codebase is healthy — it defaults to empty sessions
2. Model zero-token failures are detectable but not preventable or recoverable
3. Prompt-cache efficiency is unmeasurable (yoagent limitation)
4. Session success rate is ~0% in recent window — the harness runs but doesn't produce

## Bugs / Friction Found

1. **[MEDIUM] Planning pipeline produces empty sessions**: When the codebase is healthy, the task picker finds nothing actionable. After 6+ sessions of this, it's a pattern, not an anomaly. The trajectory's `planner_no_task_count=1` understates the issue — the deeper problem is that the assessment→planning handoff has no "healthy codebase" path.

2. **[MEDIUM] Model zero-token failures persist**: Day 148 added diagnostic detail but not recovery. The `state why last-failure` shows this pattern recurring. When the model returns zero tokens, the harness records it and falls through to FailureObserved — no retry with backoff, no model-switching fallback.

3. **[LOW] DeepSeek cache metrics blocked by yoagent**: Issue #90, year-old gap. Cache efficiency is a cost driver but unmeasurable in the main prompt path.

4. **[LOW] `state graph hotspots --kind failure` returns empty**: The failure relations were added (Day 146 Task 1) but the kind exists with no hotspots — either failure events aren't being related properly or there haven't been enough failures to create hotspots. Verifying this requires a full state scan.

## Open Issues Summary

Three open `agent-self` issues, all reverted tasks:

| Issue | Title | Status |
|-------|-------|--------|
| #135 | Break self-referential planning fallback when analysis-only pressure is active | Reverted (Day 144, evaluator timeout) |
| #134 | Close harness-internal model lifecycle gap | Reverted (Day 143, agent blocked) |
| #105 | Record DeepSeek prompt cache metrics during prompt runs | Reverted (Day 137, agent blocked) |

These represent work that was attempted but didn't survive verification. The planning fallback (#135) and lifecycle gap (#134) are directly relevant to current trajectory pressure and could be re-attempted with narrower scope.

## Research Findings

The DeepSeek API ecosystem has matured significantly since this harness was created. Key developments:
- DeepSeek now offers native prompt caching with `ephemeral` cache types
- The `thinking` feature has evolved with `thinking_budget` parameter
- Multiple providers now offer DeepSeek-compatible APIs (Fireworks, Together, Groq)

The harness already models these in `src/deepseek.rs` (ThinkingPolicy, CachePolicy, PromptLayoutPolicy) but the runtime integration (yoagent's Usage struct) hasn't kept pace. The gap between what the harness *models* and what it can *measure* is widening — particularly around cache and thinking budget observability.
