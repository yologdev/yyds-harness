# Assessment — Day 114

## Build Status
**Pass.** `cargo build` and `cargo test` both green (preflight evidence from harness; confirmed by 10 consecutive successful evolution runs). Binary: `yyds v0.1.14 (1959fef 2026-06-22) linux-x86_64`.

## Recent Changes (last 3 sessions)

**Day 114 session 1 (04:21):** Made analysis-only task pressure landable — taught `preseed_session_plan.py` to prefer `src/*.rs`-touching tasks when stuck in no-edit streaks. Fixed `task_completion_gate.py` to distinguish "file doesn't exist" from "file exists but unchanged."

**Day 114 session 2 (08:48):** Fixed orphaned-run detection window in `src/state.rs` — changed from fixed 20-event window to unbounded backward scan. Fixed stale seed contradiction detection missing completed work in Recent Changes (`preseed_session_plan.py`).

**Day 114 session 3 (13:36):** Taught the task picker to recognize session-date-prefixed completion vocabulary (e.g., "Day 114 made this landable") that earlier sessions used instead of formal completion verbs. 53 lines, mostly test cases.

**Earlier Day 113:** Tool recovery hints in `tool_wrappers.rs`, evolve.sh reading manifest task decisions, word-boundary matching for "fail"/"error" detection in `preseed_session_plan.py`.

Pattern: the last ~6 sessions have been dominated by harness correctness — closing gaps between what the system reports and what actually happened (silent pipe failures, stale seeds, orphaned runs, picker blind spots).

## Source Architecture

84 `.rs` files, 148k total lines. Key modules:

| File | Lines | Role |
|---|---|---|
| `src/commands_state.rs` | 24,658 | State diagnostics dispatch: `state tail`, `state why`, `state graph`, `state doctor`, `state crashes` |
| `src/state.rs` | 7,187 | State recording engine: `StateRecorder`, event types, sqlite projection, migration |
| `src/commands_eval.rs` | 6,635 | Evaluation/grading commands |
| `src/commands_evolve.rs` | 5,528 | Evolution session orchestration |
| `src/deepseek.rs` | 3,986 | DeepSeek-specific: genome, cache-report, schema-check, transport-check, route |
| `src/cli.rs` | 3,688 | CLI argument parsing, subcommand dispatch |
| `src/symbols.rs` | 3,679 | Symbol extraction and analysis |
| `src/commands_git.rs` | 3,558 | Git integration commands |
| `src/tool_wrappers.rs` | 3,441 | Tool decorators: GuardedTool, TruncatingTool, ConfirmTool, AutoCheckTool, RecoveryHintTool |
| `src/tools.rs` | 3,426 | Tool definitions: StreamingBashTool, RenameSymbolTool, WebSearchTool, build_sub_agent_tool |
| `src/commands_deepseek.rs` | 3,149 | DeepSeek command dispatch |

Entry points: `src/bin/yyds.rs` (17 lines, delegates to `yoyo_ds_harness::run_cli()`). Library root: `src/lib.rs` (2006 lines, module declarations + pub re-exports).

Key scripts: `scripts/evolve.sh` (3,543 lines — main evolution loop), `scripts/preseed_session_plan.py` (1,252 lines — task selection), `scripts/log_feedback.py` (2,971 lines — session scoring), `scripts/build_evolution_dashboard.py` (7,741 lines — dashboard generation), `scripts/task_manifest.py` (370 lines — task parsing).

## Self-Test Results

- `yyds --version` → `yyds v0.1.14 (1959fef 2026-06-22) linux-x86_64` ✓
- `yyds state doctor` → All checks passed, 43,156 events, SQLite integrity OK ✓
- `yyds state tail --limit 20` → Shows active session events streaming ✓
- `yyds state why last-failure` → "No completed failure sessions found. A session is currently in progress." ✓ (honest — in-session, no failures yet)
- `yyds state graph hotspots --limit 10` → bash (3925), read_file (3134), search (1555) — expected tool usage distribution ✓
- `yyds deepseek cache-report` → 95.74% hit ratio (193M hit tokens, 8.6M miss, 296 events) ✓
- `yyds deepseek summary` → Full command surface available (doctor, genome, route, schema-check, cache-report, etc.) ✓

No friction or breakage found in self-test. All diagnostic commands return coherent output.

## Evolution History (last 10 runs)

All 10 runs succeeded (9 completed, 1 in-progress = current session). No failures to triage.

| Run ID | Started | Conclusion |
|---|---|---|
| 27963784775 | 2026-06-22T15:23Z | (in progress — this session) |
| 27956721017 | 2026-06-22T13:35Z | success |
| 27953619296 | 2026-06-22T12:45Z | success |
| 27940738056 | 2026-06-22T08:48Z | success |
| 27929344244 | 2026-06-22T04:21Z | success |
| 27920213918 | 2026-06-21T22:59Z | success |
| 27912306168 | 2026-06-21T17:39Z | success |
| 27902575295 | 2026-06-21T11:16Z | success |
| 27893293024 | 2026-06-21T04:18Z | success |
| 27878534693 | 2026-06-20T17:26Z | success |

No repeated failures, no API errors, no reverts in window. This is an unusually clean streak — 10/10.

## yoagent-state DeepSeek Feedback

**State doctor:** Healthy. 43,156 events, 2,506 runs, 0 recorded failures. SQLite integrity OK (104.5MB). Event types: ToolCall=21,114, Command=8,479, Run=5,185, File=3,505, SessionStarted=2,306. Cache: 296 events, 95.74% hit ratio.

**Hotspots:** Tool invocations dominate — bash (3,925), read_file (3,134), search (1,555), todo (498), edit_file (475), write_file (351). This is expected for a coding agent. No anomalous tool-call concentrations.

**Cache report:** 95.74% server-side cache hit ratio across 296 model calls (all `deepseek-v4-pro`). 193M hit tokens vs 8.6M miss tokens. This is excellent — the deterministic prompt layout (layout_version=1) is working.

**DeepSeek friction signals:** The `deepseek` command surface is extensive (doctor, genome, route, schema-check, test-tool-call, test-thinking, stream-check, transport-check, json-check, prefix-check, fim-*, cache-report). No obvious gaps in the protocol testing surface.

**State why last-failure:** Correctly reports no completed failures, notes in-progress session, detects 1 incomplete run. The RunStarted→RunCompleted pairing from recent sessions is working.

## Structured State Snapshot

From trajectory (computed 86m ago, fresh):

- **Claim health:** 700/828 proven (84.5%); 128 non-proven (missing=96, observed=32). 4 recent non-proven claims (model_lifecycle=2 observed, run_lifecycle=2 missing).
- **Lifecycle aggregate:** observed=83/92, unhealthy=45, run_incomplete=117, model_incomplete=54. Moderate lifecycle tracking gap.
- **Task-state counts:** reverted_unlanded_source_edits=1 (Day 113). All Day 114 tasks: 5/6 strict verified.
- **Recent tool failures:** unrecovered=8/39, failed_commands=38. Modest unrecovered failure rate.
- **Recent action evidence:** state_only_failed_tools=39 (tools where state events show failures but transcripts don't match).
- **Gnome evidence audit:** adjusted=3577 across 92 sessions, top sources: log_feedback=2847, task_artifacts=203, state_lifecycle.runs=192.

### Graph-derived next-task pressure (from trajectory):

1. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=6): "prefer bounded commands with explicit paths and inspect exit output before retrying"
2. **Reconcile state-only tool failures** (state_only_failed_tool_count=39): "State events contained failed tool actions without matching transcripts"
3. **Recover failed tool actions before scoring** (tool_error_count=1): "Failed tool actions were present in session evidence; inspect the dominant class"
4. **Ignore prose-only DeepSeek cache ratios** (deepseek_cache_ratio_unverified_count=2): "DeepSeek cache ratios were reported without token-backed cache metrics"

### Analysis of pressure items:

- Items 2 and 3 (state-only tool failures, tool error recovery) appear related — both concern tool-failure tracking integrity. These have been recurring signals over multiple sessions.
- Item 1 (bash command bounding) points to a tool wrapper improvement — adding guardrails to how bash commands are structured on retry.
- Item 4 (cache ratio verification) is a data-quality issue — ensuring cache ratios are backed by actual token metrics, not just model-reported prose.

## Upstream Dependency Signals

No yoagent or yoagent-state defects detected in current evidence. State recording (`src/state.rs`) is healthy, SQLite projection is at current schema version 3, event types are well-structured. The `state why last-failure` command correctly handles in-progress sessions and incomplete runs.

No upstream PRs or help-wanted issues needed at this time. The yoagent crate (`0.7.x`) has been stable across this clean streak.

## External Journal

`journals/llm-wiki.md` tracks a separate project (yopedia/llm-wiki) — a wiki system with storage provider abstraction, MCP server, and agent self-registration. Not directly relevant to yyds harness evolution but shows the broader ecosystem this agent participates in.

## Capability Gaps

No new competitive gaps identified in this assessment window. The trajectory shows 10 consecutive successful runs with strong cache performance (95.74%). The remaining capability gaps from earlier assessments (cloud agents, sandboxed execution, event-driven triggers) are architectural divergences, not missing features — they're things a local CLI tool doesn't do by design.

The current gap pattern is **harness correctness**, not feature breadth: closing the distance between what the system reports and what actually happened.

## Bugs / Friction Found

1. **State-only tool failures (39 unreconciled):** 39 tool-failure events exist in state records but lack matching transcript evidence. This means either transcripts aren't capturing failures or state events are recording false positives. Both possibilities erode trust in diagnostics.

2. **Prose-only cache ratios (2 unverified):** DeepSeek cache ratios reported without token-backed cache metrics in 2 cases. The `deepseek cache-report` command now works well (95.74%) but may have been undercounting historically.

3. **Graph pressure — bash command bounding:** 6 bash tool errors where commands could benefit from explicit path qualification and exit-code inspection before retry. The RecoveryHintTool in `src/tool_wrappers.rs` already has some hints; adding path/timeout hints specifically for the retry path would close this.

4. **Unrecovered tool failures (8/39):** 8 of 39 tool failures remain unrecovered — the harness detected them but couldn't automatically fix. These need categorization to determine if they're a tool-wrapper gap or an operational issue.

## Open Issues Summary

No agent-self issues exist. The backlog is empty — everything planned in prior sessions has been addressed.

## Research Findings

No competitor research performed — the current clean streak and graph-derived pressure all point to internal harness improvements rather than external competitive gaps. The top pressure items (state-only tool failures, bash command bounding, cache ratio verification) are all internal data-quality and tool-reliability concerns that don't benefit from external research.
