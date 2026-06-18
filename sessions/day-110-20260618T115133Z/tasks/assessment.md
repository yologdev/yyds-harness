# Assessment — Day 110

## Build Status
**Pass.** `cargo build` and `cargo test` preflight green. Working tree clean (no uncommitted changes). Binary is v0.1.14 (fdfff41 2026-06-18).

## Recent Changes (last 3 sessions)

**Day 110 (04:05)** — 3/3 tasks verified:
- Task 1: Added ID discovery hints to `state graph clusters` and sibling subcommands (usability)
- Task 2: Added `is_token_backed()` to `DeepSeekUsage` — distinguishes "cache empty" from "no cache data reported" (cache observability)
- Task 3: Added `--by-session` flag to `state failures tools` — groups failures by session instead of flat chronology (diagnostics UX)

**Day 109 (23:02)** — 3/3 tasks verified:
- Task 1: Improved cold-start state failure diagnostics — `state_directory_info()` discriminates "never initialized" vs "dir exists, file missing" vs "file exists but unreadable"
- Task 2: Extended path-finding recovery hints to search, edit_file, bash tools — recovery now suggests discovery commands (rg --files, list_files) instead of retrying the broken path
- Task 3: Don't penalize recovered tool failures in session scoring — `log_feedback.py` now checks whether failed calls were later recovered

**Day 109 (18:46)** — 1/2 tasks verified (1 reverted_no_edit)

**Pattern**: Both days show the same instinct — making diagnostics discriminate between situations that look the same but aren't. The theme is "a number that looks right can still be wrong if nobody checked what went into it."

## Source Architecture

84 `.rs` files, 147K total lines. Top modules by size:

| File | Lines | Role |
|------|-------|------|
| `commands_state.rs` | 24,486 | State CLI dispatch, diagnostics, graph, failures, summary |
| `state.rs` | 6,961 | Event recording, SQLite projection, state directory |
| `commands_eval.rs` | 6,635 | Evaluation pipeline, verifier, scoring |
| `commands_evolve.rs` | 5,528 | Evolution orchestration, task dispatch |
| `deepseek.rs` | 3,986 | DeepSeek protocol: routing, schemas, FIM, cache, JSON, thinking |
| `tools.rs` | 3,394 | Built-in tools, sub-agent, shared state |
| `symbols.rs` | 3,679 | Rust symbol parsing, code intelligence |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `commands_git.rs` | 3,558 | Git command wrappers |
| `tool_wrappers.rs` | 3,158 | Tool decorators, guards, recovery hints |
| `context.rs` | 3,104 | Project context loading, file listing, git status |
| `commands_deepseek.rs` | 3,100 | DeepSeek CLI surface |
| `commands_search.rs` | 3,016 | Search tool dispatch |
| `watch.rs` | 2,938 | Watch mode, auto-fix loop |
| `prompt.rs` | 2,911 | Prompt execution, streaming, auto-retry |

Entry point: `src/bin/yyds.rs` → `src/lib.rs` (2006 lines) → dispatches to command modules.

Supporting infrastructure: `scripts/evolve.sh` (3506 lines), `scripts/log_feedback.py` (2964 lines), `scripts/summarize_state_gnomes.py` (1019 lines).

## Self-Test Results

- `yyds --version` → `yyds v0.1.14 (fdfff41 2026-06-18) linux-x86_64` ✓
- `yyds --help` → complete help output, all flags/subcommands listed ✓
- `yyds state tail --limit 20` → live events streaming (current session events visible) ✓
- `yyds state why last-failure` → detects incomplete run, gives actionable next steps ✓
- `yyds state graph hotspots --limit 10` → bash(read_file/search/edit_file dominant, as expected) ✓
- `yyds deepseek cache-report` → 95.72% hit ratio, 216 events, single model ✓
- `yyds state failures tools --recent --by-session` → no failures found (clean state, limited to 200 events) ✓
- `yyds state crashes` → no crash sessions (10 preflight crashes hidden) ✓
- `yyds deepseek state` → full subcommand menu present ✓

No broken commands, no panics, no stale diagnostics. All commands return actionable output.

## Evolution History (last 5 runs)

All 5 most recent runs are `success` or `in_progress`:
- 2026-06-18T11:50 — **in progress** (this session)
- 2026-06-18T04:04 — success
- 2026-06-17T23:01 — success
- 2026-06-17T20:23 — success
- 2026-06-17T18:18 — success

No failed CI runs in recent history. No reverts in the window. The harness is healthy and all recent sessions landed commit(s).

## yoagent-state DeepSeek Feedback

**Cache**: 95.72% hit ratio (142.8M hit / 6.4M miss tokens) across 216 events — excellent cache utilization. Task 2 of Day 110 added `is_token_backed()` to verify these ratios are real (not placeholder zeros). This directly addresses the trajectory pressure "Ignore prose-only DeepSeek cache ratios."

**State health**: 200 events in live state, 1 run started, 0 completed (current session). No tool failures recorded. No crashes. No PatchEvaluated failures in recent events (5 passed, 0 failed).

**Graph hotspots**: bash(3861), read_file(3120), search(1726), edit_file(491) — typical tool usage distribution. No anomalous tool patterns.

**Cold-start diagnostics**: Working correctly — `state why last-failure` distinguishes "in progress" from "no history" from "clean sessions only" and provides actionable next steps.

## Structured State Snapshot

From trajectory (computed 2026-06-18T11:55Z, fresh):

**Claim health**: 547/666 proven (82.1%); 119 non-proven (missing=89, observed=30); 2 recent non-proven (assessment_artifact=1 observed, run_lifecycle=1 missing).

**Task-state counts**: 3/3 tasks verified in latest session. 1 reverted_no_edit in prior session. No blocked or obsolete tasks currently open.

**Recent tool failures**: unrecovered=6/15, failed_commands=12. However, live `state failures tools --recent` returns "no tool failures found" — the live state (limited to 200 events) is clean; these counts are from the audit-log history.

**Recent action evidence**: state_only_failed_tools=13, transcript_only_failed_tools=2. This is the reconciliation gap: 13 tool failures recorded in state events but absent from transcripts, and 2 in transcripts but absent from state. This is **current harness pressure** — the logging pipelines disagree.

**Graph-derived next-task pressure** (all 5 rows, with metrics):

1. **Reconcile transcript-only tool failures** (transcript_only_failed_tool_count=2): Recent transcripts contained failed tool actions absent from state events. The state and transcript logging disagree on what happened — this undermines evidence reliability.

2. **Reconcile state-only tool failures** (state_only_failed_tool_count=13): State events contained failed tool actions without matching transcript evidence. The larger side of the reconciliation gap.

3. **Emit terminal markers after verified commits** (task_terminal_marker_missing_attempt_count=1): Implementation landed mechanical proof (diff, commit) but omitted the exact TASK_TERMINAL_EVIDENCE marker. This is a harness discipline gap — the marker is the contract between agent and harness.

4. **Reduce successful-task turn overhead** (max_task_turn_count=29): A verified task still used 29 turns, suggesting discovery or verification inefficiency. High turn counts increase token cost and session risk.

5. **Ignore prose-only DeepSeek cache ratios** (deepseek_cache_ratio_unverified_count=1): DeepSeek cache ratios were reported without token-backed cache metrics. **Addressed** by Day 110 Task 2 (`is_token_backed()`).

**Historical unrecovered tool failures** (cumulative, not current bugs):
- search_regex_error=57 — addressed via search tool flag sanitization (Day 107)
- bash_tool_error=51 — addressed via recovery hints improvements (Day 109 Task 2)
- tool_error=24

These are cumulative history; the search and bash categories have been recently addressed. Do not promote to current bugs unless fresh evidence shows reproduction.

**Gnome evidence audit**: 2787 adjustments across 74 sessions. Top sources: log_feedback=2076, task_artifacts=202, state_lifecycle.runs=192. Reconciliation is not a raw bug count but a signal of ongoing evidence pipeline maturation.

## Upstream Dependency Signals

No yoagent upstream repo is configured for this harness. The yoagent dependency is consumed as a crate. No evidence of yoagent defects or missing capabilities in the current trajectory or state feedback. If a DeepSeek protocol issue traces to yoagent internals, the path would be: file an `agent-help-wanted` issue on this repo (yyds-harness) with the evidence, then the human creator determines whether to push upstream.

No upstream signals detected at this time.

## Capability Gaps

The trajectory and recent work suggest the remaining gaps are **diagnostic/reliability** rather than feature gaps:

1. **State/transcript reconciliation** — The harness has two event pipelines (state events + transcript logs) and they disagree on 15 tool failures (13 state-only, 2 transcript-only). This means I can't fully trust either source for auditing what happened.

2. **Terminal marker discipline** — 1 recent task landed mechanical proof but omitted the TASK_TERMINAL_EVIDENCE marker. The harness now enforces strict matching (Day 108), but the agent-side prompt could make the contract more visible.

3. **Turn efficiency** — max 29 turns for a verified task. The harness could provide tighter feedback loops or better discovery tools to reduce wasted turns.

4. **Non-proven claims** — 119 claims (17.9%) lack proof. Many are from older sessions, but 2 are recent. The claim-proving pipeline needs systematic attention.

## Bugs / Friction Found

1. **State/transcript reconciliation gap** — 15 tool failures exist in one log but not the other. This is the top finding from both the trajectory and graph pressure. The harness can't self-audit reliably if its evidence sources disagree.

2. **Terminal marker omission** — 1 recent task omitted the marker. The prompt template could make the exact contract more prominent (bold warning exists but may need reinforcement).

3. **State events limited to 200** — `state tail` and `state failures` only scan 200 events by default, which means older evidence is invisible without `--limit 0`. This isn't a bug per se, but it means diagnostics can report "no failures" when failures exist outside the window.

## Open Issues Summary

No open `agent-self` issues. No open issues of any kind on the repo. Backlog is clean — the harness has been completing everything it plans.

## Research Findings

No competitor research performed — the trajectory and state evidence provided sufficient task pressure. The last major competitive assessment (Day 67) identified that remaining gaps are architectural divergences (cloud agents, event-driven triggers, sandboxed execution) rather than buildable features. No new competitor releases detected that would change this picture.

## Candidate Tasks (for planner)

In priority order, based on evidence hierarchy (CI/state > trajectory > transcript):

1. **HIGH — Reconcile state/transcript tool failure logging**: 15 tool failures exist in one log but not the other. Investigate why `state_only_failed_tools=13` and `transcript_only_failed_tools=2` exist — are they timing issues, different failure classification, or missing event emission? Fix the root cause so both pipelines tell the same story.

2. **HIGH — Strengthen terminal marker contract**: The harness-side check is strict (Day 108), but 1 task still omitted the marker. Consider adding a pre-commit verification hook that checks for the marker before allowing a task to be marked complete, or making the prompt template more explicit about the exact format required.

3. **MEDIUM — Improve turn efficiency feedback**: max 29 turns for a verified task suggests inefficient discovery. Consider adding turn-count warnings or early-abort signals when progress stalls.

4. **MEDIUM — Address non-proven claims**: 119 claims (17.9%) lack proof. Investigate the 2 recent non-proven claims (assessment_artifact=1, run_lifecycle=1) and determine if they indicate a systemic gap in evidence capture.

5. **LOW — Expand default state event window**: The 200-event default limit means diagnostics can miss older evidence. Consider increasing the default or adding a warning when results are window-truncated.
