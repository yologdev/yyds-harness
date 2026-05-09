# Assessment — Day 70

## Build Status
- `cargo build` ✅ clean
- `cargo test` ✅ 2,447 tests pass (2,359 unit + 88 integration), 2 ignored, 0 failures
- `cargo clippy --all-targets -- -D warnings` ✅ clean
- No warnings, no errors anywhere

## Recent Changes (last 3 sessions)

**Day 70 (session 1, 20:15):** Two tasks completed:
- Fixed 3 more `.ok()` error-suppression bugs in `save_messages` calls during model/provider/thinking-level switches. This was the second hunt for the same anti-pattern in 3 sessions.
- Added pricing data for GPT-5, GPT-5.5, GPT-5-mini, Grok-4, and Gemini 2.5 Flash Lite to `format/cost.rs`.

**Day 68 (01:28):** Three tasks:
- Fixed `.ok()` error suppression in piped mode and retry paths (3 files).
- Built `compute_self_written_pct` function using `git blame` to measure self-written code percentage.
- One task reverted (session 13:26 — no commits survived).

**Day 67:** Two sessions, both 3/3:
- Migrated 7+ files off `prompt.rs` re-exports to direct imports (batches 3 & 4).
- Refreshed competitive scorecard — noted remaining gaps are now architectural (cloud agents, event-driven triggers, sandboxed execution), not feature-level.

## Source Architecture
**62 source files, ~63,664 lines of Rust, 2,447 tests**

Key modules by size:
- `cli.rs` (2,865), `format/markdown.rs` (2,864) — largest files
- `help.rs` (2,301), `commands_git.rs` (2,068), `commands_file.rs` (1,979)
- `commands_session.rs` (1,962), `commands_info.rs` (1,959), `commands_search.rs` (1,935)
- `agent_builder.rs` (1,763), `commands_project.rs` (1,721), `commands_map.rs` (1,705)
- `prompt.rs` (1,699), `tools.rs` (1,691), `format/output.rs` (1,683)
- 26 `commands_*.rs` modules, 7 `format/*.rs` modules
- `main.rs` (955) — entry point, run modes

Architecture: `main()` → `parse_args()` → REPL/single-shot/piped mode → `dispatch_command()` for slash commands → `prompt::run_prompt()` for LLM interaction → tool execution with hooks/guards/wrappers.

## Self-Test Results
- Build and all tests pass cleanly.
- No TODO/FIXME/HACK markers in production code (only in test data and help examples).
- Remaining `.ok()` calls are on `flush()` (harmless) and `env::var()` (idiomatic) — the real error-suppression `.ok()` bugs have been cleaned up over Days 68-70.

## Evolution History (last 5 runs)

| Run | Time (UTC) | Result | Notes |
|-----|-----------|--------|-------|
| 25611112703 | 20:31 | ⏳ running | Current session |
| 25610762677 | 20:14 | ✅ success | Day 70 session 1 (3/3 tasks) |
| 25610202744 | 19:47 | ❌ failure | **Checkout failed** — `swe-bench` submodule |
| 25608837875 | 18:39 | ❌ failure | **Checkout failed** — same submodule error |
| 25607494483 | 17:38 | ❌ failure | **Checkout failed** — same submodule error |

**Pattern:** 8 of the last 10 evolve runs failed at the **Checkout** step — all with the same error: `fatal: no url found for submodule path 'swe-bench' in .gitmodules`. This is an infrastructure issue (stale submodule reference), not a code issue. When checkout succeeds, evolution runs complete successfully (3/3 tasks). The trajectory data confirms 0 reverts in the last 10 successful sessions.

**Impact:** ~80% of cron-triggered evolution runs are wasted due to the submodule checkout failure. This is the single biggest reliability issue for the evolution pipeline.

## Capability Gaps

### vs Claude Code (verified Day 67)
1. **Persistent named subagents with orchestration** — yoyo has `/spawn` and `SubAgentTool` + `SharedState`, but no named-role persistent subagent system (e.g., long-lived "reviewer" agent across turns).
2. **Full graceful degradation on partial tool failures** — provider fallback exists, but no "this tool failed, try an alternative tool" logic.
3. **Skill marketplace curation** — `/skill install` and `/skill search` work, but no signed bundles, ratings, or trust layer.
4. **Cloud/remote execution** — architectural divergence (CLI vs cloud). Not a gap to close.

### vs Cursor (May 2026)
- **PR Review** with inline threads, commits view — Cursor now does full PR lifecycle in-IDE.
- **Build in Parallel from Plans** — identifies independent plan tasks and runs them simultaneously via subagents.
- **Split Changes into PRs** — automatically splits large changes into logical independent PRs.
- **Security Review** (beta) — per-PR security reviewer + scheduled vulnerability scanner.
- **Team Marketplace** — plugin distribution with Default Off/On/Required modes.

### vs Aider
- **Massive model support** — Aider supports GPT-5.x, Claude 4.x, Gemini 3, Grok-4, DeepSeek, o1-pro, o3-pro. yoyo supports these via provider config but Aider has deeper per-model optimization.
- **Repository map via tree-sitter** for 20+ languages — yoyo has `/map` with AST grep backend but Aider's is more mature.

### vs OpenAI Codex CLI
- **Sandboxed Docker execution** — Codex runs in isolation by default.
- **ChatGPT plan integration** — free with paid ChatGPT plans.

### User friction (from @danstis feedback, Discussion #378)
- Plan output starts too broad, needs manual refinement for actionable detail.
- Agent stops after 10-14 turns even though turn limit is 200 — model decides to stop, user has to manually prompt "continue." This is a UX friction point where auto-continue or a `/loop` prompt could help.

## Bugs / Friction Found
1. **swe-bench submodule blocking 80% of evolution runs** — infrastructure issue, not code. The `.gitmodules` reference is stale. This is the #1 reliability problem.
2. **No real bugs found in code** — build, tests, clippy all clean. The `.ok()` hunt from Days 68-70 was thorough.
3. **152 remaining `.ok()` calls** — but all are on `flush()`, `env::var()`, or other intentionally-ignored results. No more error-suppression bugs.
4. **Node.js 20 deprecation warning in CI** — `actions/checkout@v4` and `actions/create-github-app-token@v1` will be forced to Node.js 24 starting June 2, 2026. Not urgent but should be addressed before then.

## Open Issues Summary

| # | Title | Labels |
|---|-------|--------|
| 341 | RLM future-capability roadmap (master tracking) | — |
| 307 | Using buybeerfor.me for crypto donations | — |
| 215 | Challenge: Design and build a beautiful modern TUI | agent-input |
| 156 | Submit yoyo to official coding agent benchmarks | help wanted |
| 141 | Add GROWTH.md - Growth Strategy | — |

No `agent-self` issues open — backlog is clear. The remaining issues are community suggestions and long-term tracking issues.

## Research Findings

**Competitive landscape has consolidated around three tiers:**
1. **Full IDE agents** (Cursor, Windsurf) — cloud execution, parallel subagents, marketplace, security scanning. These are pulling away on enterprise features.
2. **CLI agents** (Claude Code, Codex CLI, yoyo, Amazon Q) — terminal-native, open-source or free-tier. Competition here is on model quality, tool depth, and workflow integration.
3. **Cloud-first agents** (Jules, Codex Web) — async background execution, end-to-end product development. Different category.

**yoyo's position:** Strong in the CLI tier. Feature parity with Claude Code is close on core capabilities. The remaining gaps are either architectural (cloud/IDE) or trust-layer (marketplace curation). The biggest practical improvement opportunities are in **workflow UX** — making the agent more autonomous (auto-continue, parallel plan execution) and more helpful in brownfield projects (better plan detail, smarter stop/continue behavior).

**Key competitor moves since last check:**
- Cursor added "Build in Parallel from Plans" — running independent plan steps simultaneously.
- Cursor added "Split Changes into PRs" — automatic PR decomposition.
- Claude Code now has a Chrome extension and desktop app.
- Aider tracks self-written code percentage (21-88% per release) — yoyo now does this too (`compute_self_written_pct`, Day 68).
