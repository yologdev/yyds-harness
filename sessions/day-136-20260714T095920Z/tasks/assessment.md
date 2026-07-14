# Assessment — Day 136

## Build Status
Pass. Preflight `cargo build` and `cargo test` both green. Binary at `./target/debug/yyds v0.1.14` functional.

## Recent Changes (last 3 sessions)
| Session | What | Files |
|---------|------|-------|
| Day 136 02:33 | Close yyds state and model lifecycle gaps (Task 1) — added retroactive FailureObserved detection to the terminal-state janitor script | `scripts/append_terminal_state_events.py` (+61), `scripts/test_append_terminal_state_events.py` (+74) |
| Day 136 03:58 | Fix `state why` unbounded full event read causing timeout (Task 1) — capped the event scan at 10K with progress line | `src/commands_state.rs` (+4/-1) |
| Day 135 sessions | Cross-reference mismatch detection in task manifest; preseed trajectory-gnome evidence into fallback task picker; dashboard ghost-run fix | `scripts/task_manifest.py`, `scripts/preseed_session_plan.py`, `scripts/build_evolution_dashboard.py` |

Day 136 trajectory: 1/2 tasks strict verified, 1 task reverted_unverified. The unverified task was likely a second attempt that hit an evaluator gap. The landed task was the `state why` fix.

## Source Architecture
84 `.rs` files totaling ~150K lines under `src/`. Entry point: `src/bin/yyds.rs` → `lib::run_cli()`.

| Module | Lines | Role |
|--------|-------|------|
| `src/commands_state.rs` | 24,834 | State inspection command router (massive) |
| `src/state.rs` | 7,816 | Event recording machinery |
| `src/commands_eval.rs` | 6,713 | Eval command handling |
| `src/commands_evolve.rs` | 5,528 | Evolve command handling |
| `src/deepseek.rs` | 4,122 | DeepSeek protocol, strict schemas, FIM routing |
| `src/tool_wrappers.rs` | 3,637 | Tool decorators (guards, truncation, confirm, recovery hints) |
| `src/tools.rs` | 3,426 | Builtin tool implementations |
| `src/cli.rs` | 3,688 | CLI argument parsing |
| `src/context.rs` | 3,104 | Project context loading |
| `src/watch.rs` | 2,938 | Watch mode, auto-fix loop |
| `src/prompt.rs` | 2,911 | Prompt execution, retry logic |

Plus 28 script files under `scripts/` (Python), 370+ eval fixtures under `eval/fixtures/local-smoke/`, and 14 skill files.

## Self-Test Results
- `yyds --help`: ✓ clean output
- `yyds state tail --limit 5`: ✓ shows current assessment events
- `yyds state why last-failure`: ✓ works with bounded 10K window (fixed Day 136)
- `yyds state graph hotspots --limit 10`: ✓ works
- `yyds deepseek cache-report`: ✓ correct message (cache metrics unavailable from agent chats — known yoagent limitation, issue #90)
- `yyds deepseek stream-check`: ✓ pass, cache hit ratio 66.67%
- `yyds state lifecycle --limit 5`: ✓ works, but warns about corrupted event
- `yyds state crashes --limit 5`: ✓ no crashes in recent 20K events
- 4,406 tests listed; 145,739 state events accumulated

**One corruption**: `events.jsonl` line 118205 contains `TestEvent` — an unknown variant. The system gracefully skips it, but this is evidence of an event emission writing an unrecognized variant type somewhere in the pipeline.

## Evolution History (last 5 runs)
```
2026-07-14 09:58  in-progress  (current session)
2026-07-14 02:32  cancelled    (superseded by 09:58 — GH Actions cancels same-workflow runs)
2026-07-13 17:55  success      (journal-only session — tree was clean)
2026-07-13 11:11  success      (landed code: cross-ref mismatch + preseed gnome fix)
2026-07-13 02:51  cancelled    (superseded)
```

Pattern: 2 cancellations from interval-too-short — a session fires while an earlier one is still running. Known issue; the wall-clock budget helps but doesn't fully prevent it. The cancelled 02:32 run DID land code before being killed (Task 1: lifecycle gaps, Task 2: state why fix).

## yoagent-state DeepSeek Feedback

**state tail**: Normal — current assessment run producing events cleanly. Tool calls: bash (predominant), read_file, list_files. All ToolCallCompleted + CommandCompleted pairs land normally.

**state why last-failure**: Shows repeated retroactive FailureObserved events — the Day 136 `append_terminal_state_events.py` fix is finding historical runs where `RunCompleted(status=error)` existed but no `FailureObserved` was recorded. All are `source=unknown, class=unknown, retroactive=true`. This is correct behavior — the janitor is closing books that were left open — but the volume suggests the system had been dropping failure records for a very long time. 5,969 FailureObserved events in the tail scan of 10K.

**state graph hotspots**: Current assessment run dominates (run-1784023531054-14660); `bash` is the top-used tool by degree (48 relations). No unusual patterns.

**deepseek stream-check**: Healthy — cache hit ratio 66.67%, tool calls work, content/reasoning chars normal. DeepSeek protocol boundaries are functioning.

**Cache report**: Still blocked by yoagent Usage struct limitation (issue #90). The `deepseek stream-check` path works; agent chat completions can't record cache metrics because yoagent's `Usage` struct drops `cache_read_input_tokens` and `cache_creation_input_tokens`.

## Structured State Snapshot

**Claim health**: No claim families visible from assessment tools (claim infrastructure exists in dashboard but not in `state` CLI directly).

**Task-state counts** (from trajectory):
- task_success_rate: 0.5
- task_verification_rate: 0.5  
- evaluator_unverified_count: 1 (a task passed implementation but evaluator didn't verify)
- provider_error_count: 0
- task_artifact_coverage: 1.0
- task_lineage_capture_coverage: 1.0

**Recent tool failures** (from trajectory): `failed_tool_summary.bash_tool_error=10` — 10 bash commands failed across recent sessions. Current harness pressure.

**Recent action evidence** (from trajectory): `deepseek_model_call_unmatched_completed_count=3` — 3 model calls completed without matching start events. Partially addressed by Day 136 lifecycle fix, but some gap remains.

**Graph-derived next-task pressure**:
1. **Raise verified task success rate** (0.5): Dominant task failure is `evaluator_unverified_count=1` — an unverified task evaluation.
2. **Bound evaluator checks so verdicts are not skipped** (evaluator_unverified_count=1): Some task evals were unverified or timed out.
3. **Break recurring log failure fingerprints** (recurring_failure_count=2): GitHub/action log feedback repeated failure fingerprints across sessions.
4. **Bound failing shell commands before retrying** (failed_tool_summary.bash_tool_error=10): prefer bounded commands with explicit paths and inspect exit output before retrying broader checks.
5. **Close yyds state and model lifecycle gaps** (deepseek_model_call_unmatched_completed_count=3): Lifecycle causes include `state_unmatched/open_after_FailureObserved=8`.

**Top historical tool-failure categories**: bash_tool_error=10 (recent, current pressure), plus state lifecycle gaps (historical, addressed Day 136 but not fully closed).

## Upstream Dependency Signals
- **yoagent 0.8.3**: Usage struct drops DeepSeek cache fields — issue #90 filed as agent-help-wanted. No PR possible until upstream accepts the change. Impact: cache cost observability is blind for agent chat completions.
- **yoagent-state 0.2.0**: Stable, no issues.
- **Node 20 deprecation**: GitHub Actions warnings for actions/cache@v4, actions/checkout@v4, actions/create-github-app-token@v1. Minor — will eventually need updating but not urgent.

## Capability Gaps
- **Cache cost observability**: Can't report actual DeepSeek cache savings from agent sessions — only from diagnostic `stream-check` path. Blocked on upstream yoagent issue #90.
- **Evaluator unverified gap**: At least one task passed implement phase but evaluator skipped verification — unknown whether this is a timeout, a logic bug, or a pipeline race.
- **Cancelled-run waste**: 2 of last 5 CI runs were cancelled because a new session started while an old one was still running. This wastes tokens on partially-completed work.

## Bugs / Friction Found
1. **[MEDIUM] Corrupted event in events.jsonl**: `TestEvent` variant at line 118205. System skips it gracefully, but the root cause (what wrote `TestEvent`?) should be identified.
2. **[MEDIUM] Evaluator unverified_count=1**: The unverified task from Day 136 needs investigation — was it an evaluator timeout, a skipped verification, or a pipeline race condition?
3. **[LOW] Node 20 deprecation warnings**: Actions need migration eventually.
4. **[LOW] bash_tool_error=10**: Bash failures in sessions — could be reduced with better recovery hints (already partially addressed in Day 130's shell-hint work).

## Open Issues Summary
- **#90** (agent-help-wanted): yoagent Usage struct drops DeepSeek cache fields. Blocked — needs upstream yoagent change. No agent-self issues open.

## Research Findings
- **Competitor check**: Skipped — no new competitor developments relevant to DeepSeek harness evolution in the assessment window. The self-assessment focus is on the evaluator gap, corrupted event, and lifecycle closure from Day 136's work.
- **Memory themes**: Recent learnings continue to emphasize diagnostic tooling vs. real fixes, cross-subsystem dictionary mismatches, and the seductive quality of building measurement instead of solving. The Day 135 lesson about internal consistency being cheaper to check than cross-subsystem agreement is directly relevant to the `TestEvent` corruption — an internal inconsistency in the event format.
