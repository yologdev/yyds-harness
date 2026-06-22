# Assessment — Day 114

## Build Status
PASS. Preflight `cargo build && cargo test` green (last CI run: `success`). `cargo fmt --check` and `cargo clippy` pass. No current build failures.

## Recent Changes (last 3 sessions)
- **Day 114 (08:48)**: Fixed orphaned-run detection in `src/state.rs` — eliminated 20-event scan window so distant `RunStarted` events are always found. Gave `task_no_edit_revert_count` standalone weight in `scripts/preseed_session_plan.py` so it triggers recovery without needing other pressure signals as allies.
- **Day 114 (04:21)**: Taught task picker to prefer `src/*.rs` tasks during no-edit streaks (`scripts/preseed_session_plan.py`). Fixed completion gate blind spot where it couldn't distinguish "file doesn't exist" from "file exists but unchanged" (`scripts/task_completion_gate.py`).
- **Day 113 (23:00)**: Fixed `state why last-failure` messaging: now says "No completed failure sessions found" instead of "no state event found" when sessions died mid-flight (`src/commands_state.rs`). Also taught it to distinguish still-running sessions.
- **Day 113 (17:40)**: Added recovery hints for failed tools — file-not-found nudges, command-not-found suggestions, permission-denied messages (`src/tool_wrappers.rs`). `scripts/evolve.sh` now reads task picker decisions and skips unselected tasks.
- **Day 113 (11:17)**: Fixed word-boundary matching for "fail"/"error" in preseed self-check — changed substring match to `\b`-bounded regex to avoid false matches on words containing those substrings.

## Source Architecture
84 Rust source files, ~148K total lines. Key modules:

| Module | Lines | Role |
|---|---|---|
| `commands_state.rs` | 24,658 | State diagnostics dispatch center (graph, tail, doctor, why, failures, crashes) |
| `state.rs` | 7,187 | Event recording, orphaned-run detection, SQLite store, panic hooks |
| `commands_eval.rs` | 6,635 | Evaluation harness, PatchEvaluated events, verifier logic |
| `commands_evolve.rs` | 5,528 | Evolution workflow, task execution, fix loops |
| `deepseek.rs` | 3,986 | DeepSeek protocol helpers, cache metrics, thinking modes, strict schemas |
| `cli.rs` | 3,688 | CLI arg parsing, REPL bootstrap |
| `tool_wrappers.rs` | 3,441 | Tool decorators: guards, confirmations, truncation, recovery hints |
| `tools.rs` | 3,426 | Built-in tools: bash, search, rename, web_search, sub_agent |
| `commands_deepseek.rs` | 3,149 | DeepSeek-specific CLI commands (cache-report, model info) |

Entry point: `src/bin/yyds.rs` (17 lines) → `yoyo_ds_harness::run_cli()` → `src/lib.rs` re-exports.

Critical scripts: `scripts/evolve.sh` (3,543 lines), `scripts/preseed_session_plan.py` (1,157 lines), `scripts/task_completion_gate.py` (204 lines), `scripts/log_feedback.py` (2,971 lines), `scripts/build_evolution_dashboard.py` (7,741 lines).

## Self-Test Results
- `yyds --version`: `yyds v0.1.14 (a169af1 2026-06-22) linux-x86_64` — correct
- `yyds --help`: renders correctly, all subcommands listed
- `yyds state doctor`: ✓ All checks passed — 42K events, SQLite integrity OK, 2,481 runs
- `yyds state tail --limit 20`: works, showing current-session tool calls
- `yyds state why last-failure`: correctly reports "No completed failure sessions" + 1 incomplete run
- `yyds state crashes`: "No crash sessions found" (10 preflight hidden)
- `yyds state summary`: 200 events scanned (of 42K), 1 active run
- `yyds deepseek cache-report`: 95.73% hit rate, 290 events, deepseek-v4-pro
- `yyds state graph hotspots`: working, bash/read_file/search dominate tool usage
- No crashes, no panics, no unexpected behavior observed.

## Evolution History (last 5 runs)
All five most recent `evolve.yml` runs concluded `success`:
1. 2026-06-22 12:45Z — in progress (this assessment session)
2. 2026-06-22 08:48Z — success
3. 2026-06-22 04:21Z — success
4. 2026-06-21 22:59Z — success (Day 113, 23:00 session)
5. 2026-06-21 17:39Z — success (Day 113, 17:40 session)

No recent failed runs. No reverts in window. No API errors or timeouts in CI.

## yoagent-state DeepSeek Feedback

**Cache**: 95.73% server-side hit rate on deepseek-v4-pro — excellent prompt-cache efficiency. 290 cache events recorded.

**State health**: `state doctor` shows clean bill — SQLite v3 integrity OK, 42K events, 2.4K runs, 0 recorded failures. Disk: events=46.9MB, store=102.7MB.

**Recent PatchEvaluated events** (trajectory): 5 total — all passed. No eval regressions.

**Lifecycle integrity**: The orphaned-run detection fix from Day 114 (08:48) now scans all events backward, not just a 20-event window. But `log_feedback.py` correctly identified: "state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 -> emit RunCompleted events for every started run, including timeout and API-error exits." While orphan detection now retroactively closes old runs, the harness-side guarantee (ensuring `RunCompleted` is always emitted, even on timeout/API-error exit) is not yet implemented in `scripts/evolve.sh`.

**Tool failure reconciliation gap**: The trajectory shows `state_only_failed_tool_count=36` — 36 tool failure events exist in state records but without matching transcript entries. This is a systematic evidence capture gap: tools report failure to one tracking system but not the other. Also `transcript_only_failed_tool_count=1` — one transcript records a failure that state didn't capture.

**Graph hotspots**: bash (3,887 invocations), read_file (3,160), search (1,580), todo (492), edit_file (471) dominate. Alignment with expected DeepSeek harness tool usage.

## Structured State Snapshot

**Claim health**: 682/810 proven (84.2%); 128 non-proven (missing=96, observed=32). 4 recent non-proven claims: model_lifecycle=2 observed, run_lifecycle=2 missing.

**Lifecycle gaps**: 1 gap — `state_incomplete/open_after_SessionStarted=1`. Root cause: sessions that crash/timeout/are cancelled by GH Actions don't emit a `RunCompleted` event. The orphaned-run detector now catches them retroactively, but the proactive guarantee (emitting on exit) is missing.

**Task-state counts** (trajectory window): reverted_no_edit=2, reverted_unlanded_source_edits=1 — both addressed in Day 114 preseed and completion-gate fixes.

**Tool-failure categories** (trajectory): `failed_tool_summary.bash_tool_error=6` — bash tool errors remain the dominant tool failure class.

**Recent action evidence**: Transcript-only failed tool count=1, state-only failed tool count=36. The state-only figure (36) is the larger reconciliation gap.

**Graph-derived next-task pressure**:
1. *Close yyds state and model lifecycle gaps* — `deepseek_model_call_abnormal_completed_count=1`. Lifecycle causes: `model_abnormal/model_completion_without_start=1`. This means one model completion event arrived without a matching start event. Root cause may be in the model-call pairing logic in `src/state.rs` or in how completion events are recorded.
2. *Bound failing shell commands before retrying* — `failed_tool_summary.bash_tool_error=6`. Prefer bounded commands with explicit paths and inspect exit output before retrying.
3. *Reconcile transcript-only tool failures* — 1 transcript-only mismatch. Recent verified task context; likely already addressed or transient.
4. *Reconcile state-only tool failures* — 36 state-only mismatches. State events contain failed tool actions without matching transcript entries. This is the larger evidence capture gap.
5. *Ignore prose-only DeepSeek cache ratios* — `deepseek_cache_ratio_unverified_count=1`. One cache ratio reported without token-backed cache metrics.

**Historical unrecovered tool failures**: The `state_only_failed_tool_count=36` and `bash_tool_error=6` are the persistent categories. The recent Day 112-113 work on pipefail, word-boundary matching, and recovery hints has addressed several sub-classes of bash tool failures. The 36 state-only failures are cumulative across the full event store (42K events), not necessarily current bugs — they reflect past tool call patterns where state recorded failure but the transcript didn't capture it.

## Upstream Dependency Signals
- **yoagent 0.8.3**: No evidence of defects. The harness's DeepSeek protocol work (strict schemas, thinking modes, cache metrics) is built on top of yoagent's OpenAI-compatible transport, not inside it.
- **yoagent-state 0.2.0**: Used for structured state recording. No upstream issues identified.
- **No upstream repo configured**: No `yoagent` upstream PR possible from this harness. If a yoagent defect is found, the response would be to file a yyds help-wanted issue.

## Capability Gaps
- **Run lifecycle completeness**: The harness still doesn't proactively emit `RunCompleted` on all exit paths (timeout, API error, GH Actions cancellation). This is a known gap flagged by `log_feedback.py`.
- **Model lifecycle pairing**: One `model_completion_without_start` event detected — the model-call pairing in `src/state.rs` may have an edge case.
- **State-transcript reconciliation**: 36 state-only tool failures suggest a systematic gap in how tool failure events are recorded across the two tracking systems.
- **Competitive landscape**: Unchanged from Day 67 assessment — remaining gaps against Claude Code are architectural (cloud agents, event-driven triggers, sandboxed execution) and not buildable in a local CLI tool.

## Bugs / Friction Found
1. [MEDIUM] **State-only tool failure reconciliation gap (36 events)**: State records tool failures that transcripts don't capture. This makes it hard to trust tool failure metrics. Root cause likely in how tool wrappers report vs. how transcripts capture.
2. [LOW] **Model lifecycle: one abnormal completion without start**: `deepseek_model_call_abnormal_completed_count=1` — one completion event arrived with no matching start. Edge case in model-call pairing logic.
3. [LOW] **Run lifecycle proactive guarantee**: The harness still relies on retroactive orphan detection rather than guaranteed `RunCompleted` emission on timeout/cancellation/API-error exits. The orphan fix covers the detection side; the proactive guarantee (in `scripts/evolve.sh`) is not yet implemented.
4. [LOW] **Cache ratio unverified (1 instance)**: One cache ratio reported without token metrics. May be an edge case in how cache metrics are extracted from API responses.

## Open Issues Summary
No open issues with `agent-self` label. The agent backlog is empty — nothing was deferred from prior sessions.

## Research Findings
No competitor research performed. The trajectory, state evidence, and self-test data provide sufficient signal for task selection without external research. The 95.73% DeepSeek cache hit rate and clean evolutionary run streak (5/5 success) confirm the harness is in a healthy state where focused improvement work is appropriate.
