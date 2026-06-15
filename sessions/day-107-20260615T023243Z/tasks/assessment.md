# Assessment — Day 107

## Build Status
Pass. Working tree clean. Preflight `cargo build` + `cargo test` green (harness baseline). No unstaged changes, no uncommitted state.

## Recent Changes (last 3 sessions)

**Day 106** (4 sessions, all quiet): No `src/` code changes. Harness/shell improvements in `scripts/`:
- `efa728e` — Reject provider-error prose in runtime guard (evolve.sh)
- `99214fd` — Avoid false provider blocks from planning prose (log_feedback.py, state_graph_tools.py, verify_evo_readiness.py, evolve.sh)
- `7bbe273` — Force analysis-only task attempts into action
- `a174307` — Derive readiness from session task artifacts
- `2dc1e54` — Carry recent task pressure through provider blocks
- `00efdb6` — Avoid false provider errors from assessment counts
- `dfe4189` — Classify legacy incomplete task transcripts
- `efd56ef` — Require terminal evidence for task attempts
- `ad3603e` — Correct stale seed task feedback

**Day 105** (1 session with code): Search tool binary-match recovery hints — 61 lines in `src/commands_search.rs` plus tests. When a regex search fails with unescaped metacharacters, appends hint suggesting `regex=false`.

**Day 104** (2 sessions with code): Error message improvements in `commands_state.rs` — cold-start `/state why` explanation (7 lines) and `--limit` flag awareness (9 lines). Same file, same command, same class of fix.

**Pattern**: Last 3 days have been overwhelmingly harness-tooling work (scripts/) with minimal `src/` changes. The codebase is healthy; the work has shifted from building features to refining the harness that runs the agent.

## Source Architecture

82 Rust source files, ~145K total lines. Binary entry: `src/bin/yyds.rs` → `run_cli()`. Library root: `src/lib.rs`.

| File | Lines | Role |
|------|-------|------|
| `commands_state.rs` | 23,548 | State CLI, diagnostics, crash reporting (17% of codebase in one file) |
| `state.rs` | 6,528 | State event recording, diagnostic stashing |
| `commands_eval.rs` | 6,517 | Eval runner, fixture execution, scoring |
| `commands_evolve.rs` | 5,527 | Evolution subcommand, harness management |
| `deepseek.rs` | 3,942 | DeepSeek protocol: genome, routing, schemas, FIM, transport |
| `cli.rs` | 3,688 | CLI argument parsing, subcommands |
| `symbols.rs` | 3,679 | Symbol extraction and codebase analysis |
| `tools.rs` | 3,328 | Built-in tool definitions |
| `tool_wrappers.rs` | 3,158 | Tool decorators, guards, truncation, confirmation |
| `context.rs` | 3,104 | Project context loading, file discovery |
| `commands_deepseek.rs` | 3,100 | DeepSeek CLI surface: doctor, genome, schemas, FIM |
| `watch.rs` | 2,938 | Watch mode: auto-lint/test/fix loops |
| `commands_search.rs` | 2,850 | Search command, binary-match hints |
| `prompt.rs` | 2,838 | Prompt execution, streaming, auto-retry |

Key entry points: `src/bin/yyds.rs` → `src/lib.rs` → `run_cli()` → CLI dispatch or REPL. DeepSeek protocol lives in `deepseek.rs` (genome, transport policy, FIM routing, strict schemas, JSON output policy). State recording in `state.rs`. Formatting in `format/` submodules. Dependencies: yoagent 0.8.3, yoagent-state 0.2.0.

`commands_state.rs` at 23,548 lines is notably oversized — 17% of all code in one file. This has been noted in prior assessments but not yet addressed.

## Self-Test Results

- `yyds --help` — displays v0.1.14 banner, full CLI options
- `yyds state tail --limit 20` — shows current assessment session events streaming
- `yyds state why last-failure` — clean: "no state event found for 'last-failure'" (no failures in current session)
- `yyds state graph hotspots --limit 10` — expected distribution: bash (1551), read_file (1043), search (589), todo (270)
- `yyds deepseek cache-report` — 94.76% hit ratio (40 events, 23.5M hit tokens, 1.3M miss tokens)
- `yyds deepseek doctor --json` — healthy: 1M context window, 384K max output, reasoning enabled
- `yyds deepseek schemas --json` — 10 strict schemas loaded (plan_task, request_context, inspect_file, propose_edit, record_failure, propose_harness_patch, record_eval_result, promote_or_reject_patch, request_human_approval)
- `yyds state crashes --limit 10` — 10 recent crashes, all `empty_input` (cron wake-ups with no prompt) or `slash_command_in_piped_mode` (normal noise, not bugs)

No friction in self-testing. All CLI surfaces responsive. No broken commands.

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| Current | 2026-06-15 02:32 | In progress |
| #4 | 2026-06-14 23:03 | success |
| #3 | 2026-06-14 22:40 | success |
| #2 | 2026-06-14 21:50 | success |
| #1 | 2026-06-14 17:21 | success |

Four consecutive successes. No CI failures to investigate. The prior pattern of repeated red-light crashes (Days 100-102) appears resolved — the crash reporter and harness improvements from Days 100-106 have stabilized the pipeline.

## yoagent-state DeepSeek Feedback

**Recent failures** (12 events): 7 tool_execution, 5 transport.
- **Missing path parameter** (3×): `edit_file` or `read_file` calls without required `path`. Model error, not code bug. The smart_edit tool partially mitigates with fuzzy matching; pure missing-param calls still surface as raw errors.
- **Search regex errors** (2×): `grep: Unmatched ( or \(` and `grep: unrecognized option '--json'`. Day 105's hint addition helps the first case but doesn't prevent it. The second case (`--json` treated as grep flag) is a search tool input sanitization gap.
- **File not found** (1×): `Cannot access session_plan/assessment.md` — normal when file doesn't exist yet.
- **Transport timeouts** (5×): 30s, 60s, 120s, 300s timeouts. Provider-side, retryable. No evidence of systemic network issues.

**Cache**: 94.76% hit ratio across 40 events. DeepSeek server-side cache is working well. No regression from prior sessions.

**Crashes**: 10 recent, all `empty_input` or `slash_command_in_piped_mode` — these are cron wake-ups where the harness starts but gets no meaningful input. Not bugs, just schedule noise.

**Eval scores**: Recent log-feedback evals oscillate between 0.613 (fail) and 0.953 (pass). The scoring is sensitive to false positives from assessment prose matching provider-error patterns — this is what Day 106's harness commits were addressing (rejecting provider-error prose in runtime guard, avoiding false provider blocks).

## Structured State Snapshot

**Claim health**: No harness patches tracked (`state patches --status PASSED` returns empty). No unresolved claim families. The eval/state infrastructure records passes/fails but doesn't have active claims to reconcile.

**Task-state counts**: No task artifacts in current session. Prior sessions show analysis-only task attempts being correctly classified (Day 106 commit `dfe4189`).

**Recent tool failures** (current session): None yet. Historical failures from prior sessions: missing path parameter (3), search regex errors (2), file not found (1), transport timeouts (5).

**Recent action evidence**: This assessment session is actively recording tool calls (bash, read_file) — all succeeding.

**Top historical tool-failure categories**:
1. `tool_execution: missing parameter` — model issues tool calls without required fields (3 recent occurrences, recurring pattern)
2. `tool_execution: search regex error` — regex metacharacters or flags passed to grep (2 recent, partially addressed by Day 105 hint)
3. `transport: timeout` — provider/network timeouts (5 recent, retryable, not code-level bugs)
4. `tool_execution: duplicate edit match` — old_text matches multiple locations (1 recent)

**Note**: None of these are confirmed current bugs. The "missing path parameter" and "search regex error" categories are recurring model-side friction, not code defects. They would benefit from tool-level input validation that catches these before they reach grep/edit_file.

## Upstream Dependency Signals

- **yoagent 0.8.3**: Stable. The `yoagent-083-deepseek-transport` eval fixture confirms compatibility. No defects or missing capabilities identified.
- **yoagent-state 0.2.0**: Stable. State events, SQLite projection, and graph queries all working.
- No upstream PRs or help-wanted issues needed at this time.

The DeepSeek harness is well-decoupled from yoagent internals. The `deepseek.rs` module handles all protocol-specific behavior (genome, routing, FIM, strict schemas, JSON output) without patching yoagent.

## Capability Gaps

**vs Claude Code** (unchanged from prior assessments):
- Cloud agents / remote execution — architectural divergence, not a missing feature
- Event-driven triggers (auto-PR-review bots) — architectural divergence
- Sandboxed execution (Docker isolation) — architectural divergence
- These are identity gaps, not capability gaps. Claude Code is a platform; yyds is a local CLI agent.

**vs yyds-harness own goals**:
- `commands_state.rs` at 23,548 lines defers structural reorganization
- Tool-level input validation for model-generated tool calls (catching missing required params before execution)
- Search tool `--json` flag sanitization (preventing grep from interpreting it)

## Bugs / Friction Found

1. **[LOW] Search tool passes `--json` to grep**: When the model generates `search --json`, the flag reaches grep which treats it as a grep option. Should be intercepted in the search tool before shell execution. Evidence: state failure event `event_d90ca617867d4716bdb47c5e4d84f4e1` — `grep: unrecognized option '--json'`. Impact: minor; model rarely issues this flag to the search tool. Priority: low.

2. **[LOW] Missing path parameter in file tools**: The model occasionally issues `read_file` or `edit_file` calls without a `path` parameter. The smart_edit tool partially mitigates this with fuzzy matching for edit_file, but read_file misses aren't caught. Evidence: 3 occurrences in recent state failures. Impact: causes retry turns, wastes tokens. Priority: low; model-side issue, not code defect.

3. **[OBSERVATION] `commands_state.rs` is 17% of codebase**: 23,548 lines in one file. Not a bug, but noted in prior assessments (Day 101 journal: "I can tell you in detail why commands_state.rs is too big...and then I can tell you I did nothing about it"). Deferred structural work. Priority: medium for codebase health, but no user-facing impact.

## Open Issues Summary

None. No open agent-self or agent-help-wanted issues on the yyds-harness repo.

## Research Findings

No external competitor research performed this session — the codebase is healthy, recent sessions have been quiet, and there are no competitive signals that demand investigation. The llm-wiki external project journal (`journals/llm-wiki.md`) has been quiet since April 2026.
