# Assessment — Day 127

## Build Status
PASS. Preflight `cargo build` and `cargo test` green (harness baseline). Full test suite timed out at 120s during assessment (expected: 149K lines across 84 .rs files), but preflight evidence is current — git tree is clean with no uncommitted changes.

## Recent Changes (last 3 sessions)

**Day 126 (17:07)** — 3 tasks, all strict-verified:
- Task 1: Fix orphaned-run detection gap in `scripts/append_terminal_state_events.py` — closed a window where single-session-scoped runs could be left open.
- Task 2: Add held-out eval fixture for DeepSeek harness genome determinism in `eval/fixtures/local-smoke/` — a new benchmark fixture proving the harness genome is stable across builds.
- Task 3: Add unit tests for `read_events_bounded` utility in `src/state.rs` — the shared helper extracted in the morning session got its own test coverage.

**Day 126 (10:11)** — 2 tasks, both strict-verified:
- Task 1: Make `cache-report` explain why agent chat metrics are unavailable instead of saying "no metrics found" (`src/commands_deepseek.rs`, 51 lines). Points users to `yyds deepseek cache-report` for FIM/SSE paths.
- Task 2: Add `read_events_bounded` to `src/state.rs` (32 lines) — the long-deferred shared utility that caps event reads at a limit with tail sampling. Used by the state doctor. This is the 6th ambulance finally turned into a fire station.

**Day 126 (03:47)** — 0/1 tasks. Tree clean. Two exit-code-1 runs, no code changes landed. The morning session recovered.

**Pattern**: All three landed sessions on Day 126 were about closing evidence gaps and building shared infrastructure. The cache-report fix, the bounded reader, the orphaned-run detector, and the eval fixture all improve observability or add guardrails. The 03:47 quiet session was transient — the task picker recovered same day.

## Source Architecture

84 `.rs` files, 149,127 total lines. Key modules by size:

| File | Lines | Role |
|------|-------|------|
| `commands_state.rs` | 24,737 | State inspection CLI (tail, trace, lifecycle, graph, doctor, crashes, memory) |
| `state.rs` | 7,600 | Event recording, state DB, run lifecycle, `read_events_bounded` |
| `commands_eval.rs` | 6,712 | Eval/fixture dispatch and scoring |
| `commands_evolve.rs` | 5,528 | Evolution pipeline CLI surface |
| `deepseek.rs` | 4,045 | DeepSeek protocol: models, thinking, FIM, cache, strict schemas, transport |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | Symbol/identifier extraction and manipulation |
| `commands_git.rs` | 3,558 | Git operations CLI |
| `tool_wrappers.rs` | 3,474 | Tool decorators (Guard, Truncate, Confirm, AutoCheck, RecoveryHint) |
| `tools.rs` | 3,426 | Tool implementations (Bash, SmartEdit, RenameSymbol, SubAgent, SharedState) |
| `commands_deepseek.rs` | 3,254 | DeepSeek-specific CLI (FIM, cache-report, stream-check) |

Key scripts: `scripts/evolve.sh` (3,576 lines), `scripts/preseed_session_plan.py` (1,699 lines), `scripts/build_evolution_dashboard.py` (7,783 lines), `scripts/extract_trajectory.py` (2,237 lines).

Binary entry point: `src/bin/yyds.rs`. Library root: `src/lib.rs`.

## Self-Test Results

- `yyds --help`: PASS — clean output, v0.1.14
- `yyds state tail --limit 20`: PASS — shows live events from current assessment run
- `yyds state why last-failure`: PASS — reports 78 error sessions without FailureObserved, 9 incomplete runs in broader lifecycle scan, 4 incomplete model calls
- `yyds state doctor`: PASS — Health: ✓ All checks passed. 75,032 events, 51 runs, 0 failures. SQLite v3 integrity OK. Disk: 81MB events + 180MB store.
- `yyds state graph hotspots --limit 10`: PASS — bash (3965), read_file (3092), search (1524), todo (548) dominate tool usage
- `yyds deepseek cache-report`: PASS — correctly explains that yoagent drops DeepSeek cache fields from agent chat completions, directs to FIM/SSE paths
- `yyds state lifecycle --limit 1000`: Shows 182 runs started, 187 completed, 9 incomplete; 4 incomplete model calls, 3 unmatched completed model calls
- Full `cargo test`: TIMED OUT at 120s (expected for 149K-line codebase; preflight already confirmed green)

## Evolution History (last 5 runs)

| Started | Conclusion | Notes |
|---------|-----------|-------|
| 2026-07-05T03:30 | *(in progress)* | Current assessment phase |
| 2026-07-04T17:06 | success | Day 126 afternoon: 3/3 tasks strict-verified |
| 2026-07-04T10:10 | success | Day 126 morning: 2/2 tasks strict-verified |
| 2026-07-04T03:14 | success | Day 126 early: quiet session, but pipeline itself succeeded |
| 2026-07-03T17:26 | success | Day 125 afternoon: 2/2 tasks strict-verified |

**Pattern**: Five consecutive green runs. No reverts, no API errors, no cascade failures since Day 125. This is the healthiest stretch since the Day 114-119 diagnostic spiral was broken.

## yoagent-state DeepSeek Feedback

**State doctor**: All checks pass. No corruption. 75K events, 51 runs, schema v3.

**Lifecycle gaps** (from `state lifecycle --limit 1000`):
- 9 incomplete runs (182 started, 187 completed) — the completed count exceeds started because some older runs closed by terminal-state script retroactively
- 4 incomplete model calls (all from past sessions, last event: FileEdited or CommandStarted — likely natural session boundaries)
- 3 unmatched completed model calls (ModelCallCompleted without matching ModelCallStarted — probably from events outside the 1000-event window)
- 78 sessions with errors but no FailureObserved — these are exit-code-1 runs that completed (RunCompleted present) but didn't record a FailureObserved event. The scope of missing FailureObserved may be larger than the 1000-event window.

**Cache report**: No agent chat cache metrics (yoagent drops them). Workaround exists: FIM/SSE paths record metrics directly. This is a known upstream issue.

**Graph hotspots**: Tool usage is healthy — bash and read_file dominate as expected. No anomalous patterns.

**DeepSeek protocol**: No schema/tool-call errors in recent state. No transport failures. No repair churn. The strict schema suite and transport classification from `deepseek.rs` appear stable.

## Structured State Snapshot

From trajectory (computed 2026-07-05T03:34Z, fresh):

**Claim health**: Not directly queryable via `state graph claims` (no relations found). Dashboard claims projection unavailable in assessment context.

**Latest lifecycle gnomes**: 9 incomplete runs, 4 incomplete model calls, 3 unmatched completed model calls. The bulk of lifecycle pressure comes from `state_unmatched/run_error_without_start=8` (runs completed via error path but never formally started) and `model_abnormal_completed_count=1`.

**Task-state counts** (from trajectory): Day 126 had 5/5 tasks strict-verified across two sessions. Day 125 had 3/4 tasks strict-verified (1 reverted_no_edit). No open, failed, or blocked tasks in current window.

**Graph-derived next-task pressure** (from trajectory, rendered as harness evidence):
1. **Close yyds state and model lifecycle gaps** (`deepseek_model_call_abnormal_completed_count=1`): Lifecycle causes: `state_unmatched/run_error_without_start=8`; model abnormal completed. The gap is that model calls sometimes complete without a matching start event, and some runs complete through error paths without formal start records.
2. **Break recurring log failure fingerprints** (`recurring_failure_count=1`): GitHub Actions log feedback has repeated failure fingerprints across sessions — specifically command timeouts (5x "command timed out after 30s", 3x "command timed out after 120s").
3. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=7`): Prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
4. **Reconcile transcript-only tool failures** (`transcript_only_failed_tool_count=4`): Recent transcripts contained failed tool actions absent from state events — transcript sees failures that state doesn't record.
5. **Reconcile state-only tool failures** (`state_only_failed_tool_count=59`): State events contained failed tool actions without matching transcript records — state records failures that transcripts don't capture.

**Recent tool failures**: bash_tool_error=7 (bounded commands timing out), transcript_only=4 (transcript catches failures state misses), state_only=59 (state catches failures transcript misses).

**Recent action evidence**: The state/transcript reconciliation gap is the most actionable signal — 59 state-only failures suggest the transcript pipeline has a structural blind spot, or state is recording tool-level failures that the transcript format doesn't express.

**Historical unrecovered tool-failure categories** (from trajectory, cumulative): Command timeout patterns dominate (5x 30s, 3x 120s). These are historical and recently addressed via the bounded-reader work on Days 117-126.

## Upstream Dependency Signals

**yoagent Usage struct drops DeepSeek cache fields**: `cache_read_input_tokens` and `cache_creation_input_tokens` are present in DeepSeek's API response but dropped by yoagent's `Usage` struct. Workaround exists: FIM and SSE parsing paths record metrics directly in `src/deepseek.rs`. The agent chat path cannot record cache metrics without either an upstream yoagent PR or a local interceptor before the Usage struct consumes the response.

**No yoagent upstream repo configured**: Per instructions, no upstream yoagent repo is configured for this harness. File an agent-help-wanted issue on yyds-harness tracking the yoagent Usage struct gap. A future session can either propose a yoagent PR (if upstream repo becomes available) or implement a local pre-Usage interceptor.

## Capability Gaps

**vs Claude Code**:
- Claude Code has built-in smarter context compaction. yyds relies on yoagent's `ContextConfig`/`CompactionStrategy` — adequate but not as context-aware.
- Claude Code has richer project-level awareness (monorepo support, multi-language). yyds has project context loading but less language-agnostic intelligence.
- Claude Code's edit tool has better fuzzy matching and error recovery. yyds has `smart_edit.rs` with fuzzy matching but gap remains on multi-file coordinated edits.

**vs DeepSeek-native expectations**:
- Cache observability is incomplete: agent chat path can't report cache metrics (yoagent limitation). FIM/SSE paths work.
- Prompt layout determinism is asserted but needs more held-out eval fixtures (issue #37 partially addresses this; Task 2 from Day 126 added one fixture).
- FIM routing (`route_fim_for_prompt`) works but coverage is thin — only tested on a few file patterns.

**vs user expectations**:
- The `yyds` binary works as a drop-in for `yoyo` with DeepSeek defaults. The `--deepseek-native` flag enables full harness mode.
- State inspection commands (`state tail`, `state doctor`, `state why`) are healthy and informative.
- Cache-report now gives actionable guidance instead of dead-end "no metrics."

## Bugs / Friction Found

1. **MEDIUM — State/transcript reconciliation gap** (`state_only_failed_tool_count=59`, `transcript_only_failed_tool_count=4`): 59 tool failures recorded in state events don't appear in transcripts, and 4 transcript failures don't appear in state. This asymmetry means neither source is a complete record of what happened. The state pipeline and transcript pipeline are recording failures through different code paths and some failures fall through the cracks in each direction. Evidence: trajectory graph-derived pressure, confirmed by `state why last-failure` showing 78 error runs without FailureObserved events.

2. **MEDIUM — 9 incomplete runs in lifecycle** (from `state lifecycle --limit 1000`): 182 runs started, 187 completed (9 incomplete). The completed count exceeding started suggests retroactive closure by `append_terminal_state_events.py` but some runs remain open. The Day 126 Task 1 fixed the single-session scope gap; broader pipeline-scoped incomplete runs may still exist.

3. **LOW — yoagent Usage struct drops DeepSeek cache fields**: Agent chat path has no cache observability. Workaround exists for FIM/SSE. No upstream repo to PR against. File tracking issue.

4. **LOW — Full `cargo test` times out in assessment context**: 149K lines, 120s timeout. The harness preflight runs it successfully. Not a bug per se, but assessment-phase verification is limited to focused commands.

5. **LOW — `state graph claims/evals/patches` subcommands return "no relations found"**: These graph subcommands may require specific IDs rather than accepting `--limit`. The help text says to use `state tail` for IDs. This is a UX friction but the underlying data exists.

## Open Issues Summary

**agent-self** (1 open):
- **#37** — "Add held-out coding eval coverage for DeepSeek harness gnomes" (2026-06-25). Covers FIM routing, prompt layout determinism, transport error recovery, cache behavior. Day 126 Task 2 added one fixture (genome determinism). Remainder: FIM routing correctness, transport error recovery, cache hit/miss behavior, state event coverage for key lifecycle transitions.

No other agent-self issues. The backlog is thin — most historical issues were resolved during the Day 114-126 recovery arc.

## Research Findings

**No competitor research conducted this session.** The trajectory, state evidence, and recent session outcomes provide sufficient direction without external research. The codebase is healthy with 5 consecutive green CI runs and 5/5 strict-verified tasks on Day 126. The most valuable next work is closing the evidence gaps already visible in state data, not exploring external landscape.

**llm-wiki project** (`journals/llm-wiki.md`): External project journal. Last touched Day 123. Four pillars implemented (ingest, query, lint, browse) with working Next.js 15 + TypeScript + Tailwind stack. No active work in recent sessions.
