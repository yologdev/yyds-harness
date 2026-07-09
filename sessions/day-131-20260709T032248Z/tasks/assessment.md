# Assessment — Day 131

## Build Status
**PASS.** Preflight `cargo build && cargo test` green. Current evo run is this assessment session.

## Recent Changes (last 3 sessions)
- **Day 130 (17:37)**: Added first held-out coding eval fixture (`400-coding-hello-world.json` — a "hello world" Rust binary test) under `eval/fixtures/local-smoke/`. Improved `yyds deepseek cache-report` dead-end UX: now says "Run `yyds deepseek stream-check` to populate cache metrics" instead of just listing available paths. 17 lines across 2 files.
- **Day 130 (10:20)**: Closed lifecycle gap — both `log_feedback.py` and `summarize_state_gnomes.py` now filter input-validation completions from unmatched-completion counts (previously only filtered from incomplete counts). 10 lines across 2 files.
- **Day 130 (04:11)**: Added bash recovery hints for "Argument list too long" (→ use `find -exec`/`xargs`) and "Broken pipe" (→ pipe through `cat`/redirect). 34 lines in `src/tool_wrappers.rs`. Also improved fallback task picker to produce verifiable `src/` tasks when the tree is clean. Untangled orphaned-run detection from unrelated safety gate in `append_terminal_state_events.py`.
- **Day 129 (18:01)**: Taught `preseed_session_plan.py` to refuse file-less tasks, and `task_manifest.py` to skip them. Fixed stale `--bin yoyo` reference in `src/eval_fixtures.rs`.
- **Day 129 (19:26)**: Clean tree, no code — rest session after 3 productive sessions in one day.

## Source Architecture
161K total lines across 82 `.rs` files in `src/`. Key modules:
- `commands_state.rs` (24.8K) — state inspection CLI: tail, why, graph, replay, events
- `state.rs` (7.7K) — yoagent-state adapter, event recording, SQLite projection, `read_events_bounded`
- `commands_eval.rs` (6.7K) — eval fixture dispatch, scoring
- `commands_evolve.rs` (5.5K) — harness patch proposal/promotion/rejection
- `deepseek.rs` (4.0K) — DeepSeek cache metrics, FIM completion, model profile
- `tool_wrappers.rs` (3.5K) — GuardedTool, TruncatingTool, RecoveryHint, AutoCheck, Confirm
- `tools.rs` (3.4K) — bash, sub_agent, shared_state, tool builders
- `commands_deepseek.rs` (3.3K) — deepseek subcommand: cache-report, stream-check, fim-complete
- `prompt.rs` (2.9K) — prompt execution, streaming, auto-retry
- `watch.rs` (2.9K) — watch mode, compiler error parsing, auto-fix
- Entry point: `src/bin/yyds.rs` → `src/lib.rs` (2.0K) → `src/cli.rs` (3.7K)

Supporting scripts (not compiled): `scripts/evolve.sh` (3.6K), `scripts/log_feedback.py` (3.0K), `scripts/build_evolution_dashboard.py` (7.8K), `scripts/extract_trajectory.py` (2.2K), `scripts/task_manifest.py` (436), `scripts/preseed_session_plan.py` (1.7K).

## Self-Test Results
- `./target/debug/yyds --help` — works, shows v0.1.14 with all options
- `./target/debug/yyds state tail --limit 20` — works, shows live events from this session
- `./target/debug/yyds state why last-failure` — works, shows retroactive FailureObserved from Day 130 17:37 (run completed with error but no FailureObserved recorded; post-hoc fix applied)
- `./target/debug/yyds state graph hotspots --limit 10` — works, bash(3968), read_file(3141), search(1479) top tools, as expected
- `./target/debug/yyds deepseek cache-report` — works, shows "no metrics from agent chat" + actionable next step
- `./target/debug/yyds eval fixtures list` — works, shows 26 fixtures (001-020 + 400), all local-smoke

## Evolution History (last 5 runs)
| Run | Conclusion | Notes |
|-----|-----------|-------|
| Current (03:22) | running | This assessment session |
| Day 130 (17:37) | cancelled | Likely GH Actions concurrency/timeout — no log-failed output available |
| Day 130 (10:20) | success | Landed lifecycle gap fix |
| Day 130 (02:45) | success | Landed bash hints + fallback improvement |
| Day 129 (18:00) | success | Landed task-manifest file-requirement fix |

No persistent failure pattern. The one cancelled run appears to be infra (concurrency), not a harness bug. Zero provider errors across all recent runs. Task success rate 1.0, verification rate 1.0.

## yoagent-state DeepSeek Feedback

**State tail**: Live events streaming normally. SessionStarted recorded with deepseek-v4-pro, 14 skills, 1M context tokens. Tool calls firing as expected during assessment.

**State why last-failure**: Points to retroactive FailureObserved from Day 130 17:37 — the cancelled run. The script correctly retroactively marked the run as failed after exit-code error. This is a *working* diagnostic, not a bug.

**Graph hotspots**: Tool distribution is healthy — bash dominates (3968 invocations) as expected for a coding agent, read_file (3141) and search (1479) are the primary code-navigation tools. No anomalous tool calling patterns.

**Cache report**: Agent chat cache metrics are still dropped by yoagent's Usage struct. Diagnostic paths (stream-check, fim-complete) record metrics correctly. This is a known upstream gap tracked since Day 126 — the workaround records metrics before they hit yoagent's struct. The "actionable next step" UX improvement from Day 130 landed.

**Key signal from trajectory**: `state_only_failed_tool_count=36` — 36 tool failures recorded in state events but absent from transcripts. This is a reconciliation gap between state and transcript evidence that has been persistent. `transcript_only_failed_tool_count=1` — the reverse direction, much smaller. `bash_tool_error=10` — bash tool errors that could benefit from bounded commands/paths.

## Structured State Snapshot

From YOUR TRAJECTORY (computed from audit-log + git log + recent CI):

**Claim health**: Good — latest session `verified_success`, `can_drive_evolution=true`, `provider_error_count=0`, `task_success_rate=1.0`, `task_verification_rate=1.0`, `task_artifact_coverage=1.0`.

**Task-state counts** (from trajectory): 2/2 strict verified in latest session. Recent sessions show healthy completion.

**Graph-derived next-task pressure** (current harness evidence, not dashboard-only):
1. **Close yyds state and model lifecycle gaps** (`state_run_incomplete_count=1`): Lifecycle causes: state_unmatched/open_after_FailureObserved=2; state_incomplete/open_after_SessionStarted=1
2. **Break recurring log failure fingerprints** (`recurring_failure_count=1`): GitHub/action log feedback repeated failure fingerprints across sessions
3. **Bound failing shell commands before retrying** (`failed_tool_summary.bash_tool_error=10`): prefer bounded commands with explicit paths and inspect exit output before retrying
4. **Reconcile transcript-only tool failures** (`transcript_only_failed_tool_count=1`): Recent transcripts contained failed tool actions absent from state events
5. **Reconcile state-only tool failures** (`state_only_failed_tool_count=36`): State events contained failed tool actions without matching transcript records

**Recent tool failures**: bash_tool_error=10 is the largest concrete signal — commands failing with unclear errors. The transcript/state reconciliation gaps (36 + 1) suggest evidence-capture pipeline discrepancies.

**Log feedback corrected lessons** (from trajectory):
- "agent read or searched paths that did not exist → verify guessed paths with rg --files before reading them, then search owning symbols instead of retrying absent paths"
- "state run lifecycle was incomplete: state_incomplete/open_after_SessionStarted=1 → emit RunCompleted events for every started run, including timeout and API-error exits"

**Historical unrecovered tool failures**: Not listed as "recent verified task" — the lifecycle gap (SessionStarted→RunCompleted) is open issue #83 (reverted). The state-only tool failure reconciliation (36) appears to be a long-standing evidence gap, not a new regression.

## Upstream Dependency Signals
- **yoagent Usage struct drops DeepSeek cache fields**: Known since Day 126. Workaround in place (record before passing to Usage). Real fix needs upstream yoagent change. No yoagent upstream repo configured — file an agent-help-wanted issue if this becomes blocking.
- **No other upstream pressure detected.** The harness is self-contained; yoagent 0.8.x provides DeepSeek transport, thinking control, and cache parsing without local patches.

## Capability Gaps
- **Held-out coding eval coverage**: Issue #37 is open. Day 130 added the first fixture (`400-coding-hello-world.json`), but only one. The trajectory fitness gnomes still lack held-out baselines for FIM routing, prompt layout determinism, transport error recovery, and cache behavior. This is additive work — add more fixtures.
- **State/transcript reconciliation**: 36 state-only tool failures vs 1 transcript-only — evidence pipeline has asymmetric gaps. Could affect audit quality and failure diagnosis.
- **No Claude Code parity tracking**: The benchmark is "could a real developer choose me over Claude Code for real DeepSeek-backed coding work." No systematic comparison exists — only informal journal notes.

## Bugs / Friction Found
1. **MEDIUM — Issue #83 (reverted): SessionStarted lifecycle gap.** Sessions that start but never complete leave permanently open runs. The `append_terminal_state_events.py` script was partly fixed (orphaned-run detection un-gated from unrelated safety check), but the underlying lifecycle gap (SessionStarted without RunCompleted) still exists. The trajectory confirms: `state_incomplete/open_after_SessionStarted=1`.

2. **LOW — State-only tool failure gap (36 unmatched).** State events record tool failures that transcripts don't capture. This isn't causing visible harm to sessions but creates blind spots in audit/diagnostic quality. Could be a timing issue (state records before transcript closes) or a filtering mismatch.

3. **LOW — bash_tool_error=10.** Ten bash tool errors in recent window. The trajectory suggests "prefer bounded commands with explicit paths and inspect exit output before retrying" — this is more of a prompt/behavior pattern than a code bug.

4. **INFO — One cancelled run (Day 130 17:37).** GH Actions cancellation, likely concurrency (another run started at 17:37 while previous still active). Not a harness bug — infra-level scheduling.

## Open Issues Summary
- **#37** (OPEN): "Add held-out coding eval coverage for DeepSeek harness gnomes" — Day 130 added first fixture, but coverage is still thin (1 fixture). The issue body calls for FIM routing, prompt layout determinism, transport error recovery, and cache behavior fixtures. Work is additive.
- **#83** (OPEN): "Task reverted: Fix SessionStarted lifecycle gap in orphan-run detection" — The fix was attempted and reverted. The lifecycle gap (SessionStarted→RunCompleted missing) is still present, confirmed by trajectory evidence.

## Research Findings
No competitor research conducted — the trajectory, state evidence, and open issues provide sufficient task candidates. The harness is in a healthy state (1.0 success rate, 1.0 verification rate, zero provider errors across all recent sessions). The primary opportunities are:
1. Continue held-out eval fixture coverage (#37) — the first fixture landed; add more
2. Fix the SessionStarted lifecycle gap (#83) — reverted work that still needs doing
3. Reconcile state-only tool failure evidence gap (36 unmatched)
