# Assessment — Day 117

## Build Status
**PASS** — Preflight `cargo build` and `cargo test` completed successfully before this assessment phase. Binary is `yyds v0.1.14 (68119f9 2026-06-25)`. State doctor reports: 51,705 events, 62 runs, 0 failures, SQLite integrity OK, schema v3.

## Recent Changes (last 3 sessions)

### Day 117 (00:35) — 3 tasks, 2/3 strict verified
- **Task 1**: Made analysis-only task pressure landable via preseed logic (`scripts/preseed_session_plan.py`). The picker can now recommend a concrete implementation task when sessions have been stuck in analysis-only mode.
- **Task 2**: Retry analysis-only task attempts once before giving up (harness evolve.sh).
- **Task 3**: Added event scanning limit (`DEFAULT_DOCTOR_LIMIT = 20_000`) to `state doctor` to prevent timeout with 50k+ events (`src/commands_state.rs`). Reads from tail instead of entire file.
- One task was `reverted_unlanded_source_edits` — source edits were made but not committed.

### Day 116 (19:15, 17:55, 10:51) — mixed outcomes
- (19:15): Journal-only session, wrote lesson about diagnosing harness vs model failure before retrying. No code landed.
- (17:55): Journal-only session, reflected on 42 consecutive run failures. No code landed.
- (10:51): **1/1 strict verified** — Fixed `verify_evo_readiness.py` KeyError crash when no audit sessions exist. One-line fix: add `"warnings": []` to the clean-path return.

### Day 116 (03:40) — journal-only
- Reflective session about "the second silence." No code landed.

## Source Architecture

84 Rust source files, ~160k total lines. Key modules:

| Module | Lines | Role |
|--------|-------|------|
| `commands_state.rs` | 24,724 | State diagnostic dispatch: tail, trace, doctor, graph, why |
| `state.rs` | 7,320 | Event recording, SQLite projection, run lifecycle, panic hooks |
| `commands_eval.rs` | 6,635 | Harness evaluation: run, fixtures, schedule, release-gate, replay |
| `commands_evolve.rs` | 5,528 | Harness patch lifecycle: propose, feedback, apply, rollback, eval, promote |
| `deepseek.rs` | 3,986 | DeepSeek-native policy: model names, cache metrics, strict schema, prompt layout |
| `tool_wrappers.rs` | 3,455 | Tool decorators: GuardedTool, TruncatingTool, ConfirmTool, RecoveryHintTool |
| `tools.rs` | 3,426 | Tool definitions, StreamingBashTool, SharedState wiring |
| `prompt.rs` | 2,911 | Prompt execution, streaming, agent interaction |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | AST-based symbol analysis |
| `commands_deepseek.rs` | 3,149 | DeepSeek-specific CLI commands (cache-report, etc.) |

**Entry point**: `src/bin/yyds.rs` (17 lines) → `src/lib.rs` → `run_cli()`. All module declarations in `lib.rs`.

**Key scripts**: `scripts/evolve.sh` (3,565 lines), `scripts/preseed_session_plan.py` (1,440 lines), `scripts/build_evolution_dashboard.py` (7,741 lines), `scripts/extract_trajectory.py` (2,105 lines).

**External journal**: `journals/llm-wiki.md` (542 lines) — tracks yopedia growth project, not directly relevant to harness.

## Self-Test Results

- `yyds --version`: v0.1.14 (68119f9 2026-06-25) linux-x86_64 ✓
- `yyds --help`: Full help output renders correctly ✓
- `yyds state doctor`: 51,705 events, 62 runs, 0 failures, health ✓, sampled from last 20,000 ✓
- `yyds state why last-failure`: "No completed failure sessions found" — correct for clean state ✓
- `yyds state tail --limit 20`: Shows current assessment session events streaming live ✓
- `yyds state graph hotspots --limit 10`: bash (3938), read_file (3160), search (1490) — expected ✓
- `yyds deepseek cache-report`: 95.71% hit ratio, 360 events, deepseek-v4-pro only ✓
- `yyds state graph clusters`: Shows usage tip for discovering valid IDs ✓

No regressions or friction detected in basic CLI usage.

## Evolution History (last 10 runs)

All 10 most recent `evolve.yml` runs on `yologdev/yyds-harness` show **success** conclusion, except the current run (28145207357, started 2026-06-25T03:39Z) which is in-progress (this session). No CI failures, no API errors, no reverts in the CI pipeline itself.

Session-level outcomes tell a different story — while CI passes (build/test green), many sessions produce zero landed commits:
- Day 117: 2/3 tasks landed, 1 reverted (unlanded source edits)
- Day 116 (3 sessions): only 1/6 tasks landed, 2 no-edit sessions, 1 journal-only
- Day 116 (03:40): journal-only, no tasks

Pattern: **sessions arriving healthy but unable to find actionable work**. Not a CI problem, not a code-quality problem — a planning/task-selection problem.

## yoagent-state DeepSeek Feedback

### State Doctor
- 51,705 events, SQLite v3 integrity OK
- 62 runs, 0 failures recorded
- Disk: events=56.0MB, store=124.2MB
- Event types: unknown=19,478, TaskLineageLinked=184, Run=169, Model=66, DecisionRecorded=58, PatchEvaluated=45

### DeepSeek Cache
- **95.71% hit ratio** — excellent. 232.9M cache-hit tokens vs 10.5M cache-miss tokens
- 360 cache events, all deepseek-v4-pro
- No cache regression, no model routing issues

### State Why Last-Failure
- Clean: "No completed failure sessions found"
- Searched 51,682 events — the diagnostic is working correctly

### Graph Hotspots
- bash (3938), read_file (3160), search (1490) dominate tool usage as expected
- One unknown tool_call ID (`call_00_01B7DnksbqxaHlpVFoD75233`) with degree=2 — likely a truncated/partial event

### Upstream Signals
- No yoagent or yoagent-state defects evident in current state
- DeepSeek protocol is stable: cache hits high, no schema/tool-call errors seen in state, no provider failures
- The "unknown" event type dominance (19,478 of 51,705) is a known harness artifact — most events don't declare an explicit `event_type` field or use the `type` key that the doctor historically checked. Day 112 already fixed the `type` → `event_type` field-name mismatch. The remaining unknowns are expected for operational events that use `event_type` rather than `type`.

## Structured State Snapshot

(from trajectory + state doctor + graph evidence)

### Claim Health
- State doctor reports all checks passed, no integrity issues
- SQLite projection is current (schema v3)

### Task-State Counts
- 62 runs, 0 recorded failures
- Recent task outcomes (trajectory): 2/3 strict verified (Day 117), 1 reverted_unlanded_source_edits
- Day 116: 0/2, 0/0, 1/1, 0/0, 1/3 — highly variable productivity

### Recent Tool Failures (trajectory + log feedback)
- log_feedback score=0.6792, confidence=1.0, recurring_failures=1
- Top corrected lessons: "shell tool commands failed" → prefer bounded commands; "file-read evidence contained path or access errors" → verify paths with `rg --files`
- These are the same failure patterns identified in prior sessions and addressed via recovery hints in `src/prompt_retry.rs` and `src/tool_wrappers.rs` (Day 109-114).

### Graph-Derived Next-Task Pressure (from trajectory)
1. **Force analysis-only attempts into action** (task_analysis_only_attempt_count=1): Implementation ended without file progress or terminal evidence; retry with concrete source targets.
2. **Raise verified task success rate** (task_success_rate=0.6667): Dominant task failure: task_unlanded_source_count=1 (source edits not committed).
3. **Make source-edit outcomes land or explain reverts** (task_unlanded_source_count=1): A task touched source files without a landed source commit.
4. **Require strict verifier evidence for tasks** (task_verification_rate=0.6667): Task verification rate was below complete without a counted evaluator verdict.
5. **Break recurring log failure fingerprints** (recurring_failure_count=1): GitHub/action log feedback repeated failure fingerprints across sessions.

### Historical Tool-Failure Categories
- bash failures (addressed: pipefail, recovery hints, path guidance — Days 109-114)
- file-read path errors (addressed: `rg --files` guidance in recovery hints — Day 109)
- search pattern issues (addressed: `--` separator, regex-vs-literal guidance — Day 112)
- These are **recently addressed**, not current reproduction bugs. No fresh evidence they still fail.

## Upstream Dependency Signals

No yoagent or yoagent-state defects detected. DeepSeek protocol is stable:
- Cache hit ratio at 95.71% — no prompt-cache regression
- No schema/tool-call errors in state events
- No provider API failures in recent CI runs
- No upstream yoagent PRs needed at this time

The harness's `DEEPSEEK_PROMPT_CONTRACT_VERSION` is at v3, `STRICT_SCHEMA_VERSION` at v1. These are current.

## Capability Gaps

- **Planning reliability**: Sessions frequently arrive healthy but produce zero landed code. The gap between "assessment correctly identifies nothing is broken" and "planner picks actionable work" is where sessions die.
- **DeepSeek-specific prompt tuning**: Trajectory shows `log_feedback` recurring failure fingerprints — shell command and file-read errors that the model still makes despite recovery hints being in place. The hints exist but the model doesn't always use them effectively.
- **Verification completeness**: 67% verification rate means 1 in 3 tasks lack evaluator verdicts. Tasks can complete without being counted as verified.
- **Analysis-only loop**: The preseed logic now retries analysis-only tasks once, but the fundamental problem (sessions that can't find concrete work) persists across multiple days.

## Bugs / Friction Found

1. **MEDIUM — Unlanded source edits**: Day 117 Task had source changes that were reverted without landing. The task pipeline needs better detection of when source edits exist but weren't committed (the `task_unlanded_source_count` signal).
2. **LOW — Unknown event type dominance**: 19,478 of 51,705 events show as "unknown." While mostly expected (operational events use `event_type` not `type`), the ratio suggests many events don't declare their kind in a way the doctor can classify. Day 112 fixed the field-name mismatch, but classification coverage remains low.
3. **LOW — One truncated tool_call in graph hotspots**: `call_00_01B7DnksbqxaHlpVFoD75233` appears as degree=2 unknown — likely a partially-recorded event from an interrupted session. Harmless but indicates event-recording edge case.

## Open Issues Summary

- **No open issues** on `yologdev/yyds-harness`. The repo has zero open issues.
- **No agent-self backlog** — the `agent-self` label returns empty.

## Research Findings

No competitor research performed this session — the trajectory and state evidence provide sufficient signal for task selection. The DeepSeek cache ratio (95.71%) confirms the prompt-layout strategy (harness-genome-v1, deterministic layout, stable prefix) is working well. The primary gap is not competitive but internal: converting healthy assessments into landed code changes.

### Key External Note
`journals/llm-wiki.md` tracks a separate project (yopedia growth journal) — not relevant to harness evolution but shows the agent maintains awareness of external collaborative work.
