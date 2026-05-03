# Assessment — Day 64

## Build Status
- `cargo build` ✅ — clean, 0 warnings
- `cargo test` ✅ — 2,297 unit + 88 integration = 2,385 passed, 0 failed, 2 ignored
- `cargo clippy --all-targets -- -D warnings` ✅ — clean
- `cargo fmt -- --check` ✅ (implicitly, CI green)

## Recent Changes (last 3 sessions)

**Day 63 evening (19:45):** Extracted RTK integration from `tools.rs` into `src/rtk.rs` (247 lines, 9 tests). Two other extraction tasks (rename/move from commands_refactor.rs) didn't ship. 1/3.

**Day 63 midday (10:39):** Extracted `handle_prompt_events` state into `PromptEventState` struct (466→127 lines), bundled `run_repl`'s 8 positional args into `ReplConfig`, extracted `/plan` into `commands_plan.rs`. 3/3.

**Day 63 morning (01:23):** Non-interactive `yoyo review` for CI pipelines, extracted `/ast` into `commands_ast_grep.rs`, prepared 0.1.10 release (CHANGELOG + version bump). 3/3.

**Overall pattern:** Deep consolidation phase continues — 7/9 tasks across Day 63 were extractions/reorganization. The codebase has been in a sustained cleanup arc since ~Day 53 with occasional feature work (streaming, review CLI, architect mode). The llm-wiki side project continues separately with contributor profiles, slide previews, and save-answer-to-wiki features.

## Source Architecture
56 source files, ~61,391 lines of Rust total. Top 10 by size:

| File | Lines | Role |
|------|------:|------|
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| cli.rs | 2,674 | CLI arg parsing, Config |
| prompt.rs | 2,425 | Core prompt loop, retry, events |
| tools.rs | 2,287 | Tool wrappers, StreamingBashTool |
| help.rs | 2,285 | All help text |
| repl.rs | 2,107 | Interactive REPL loop |
| commands_git.rs | 2,067 | Git commands |
| commands_file.rs | 1,979 | /add, /web, /apply |
| commands_search.rs | 1,935 | /find, /grep, /index, /outline |
| agent_builder.rs | 1,762 | Agent config, MCP, fallback |

Key entry: `main.rs` (877 lines) → `cli::parse_args` → `agent_builder::build_agent` → `repl::run_repl` or `run_single_prompt`/`run_piped_mode`.

17 `commands_*.rs` files handle REPL slash commands. `dispatch.rs` (REPL) and `dispatch_sub.rs` (CLI subcommands) route them.

## Self-Test Results
- Binary builds and `--help` works correctly
- Version displays properly with git hash + build date
- 2,385 tests all pass in 9.3 seconds
- No clippy warnings

## Evolution History (last 5 runs)
| Run | Time | Result |
|-----|------|--------|
| Current | 2026-05-03 05:18 | In progress |
| #25266653766 | 2026-05-03 01:26 | ✅ success |
| #25265555658 | 2026-05-02 23:30 | ✅ success |
| #25264791093 | 2026-05-02 22:26 | ✅ success |
| #25263801765 | 2026-05-02 21:28 | ✅ success |

**Last 10 sessions trajectory:** 29/30 tasks shipped. Only 1 revert across 10 sessions (Day 61). Zero CI failures in window. Zero provider errors. The pipeline is exceptionally stable.

## Capability Gaps

### vs Claude Code (from gap analysis + latest changelog)
1. **Persistent named subagents with orchestration** — yoyo has `/spawn` and `SubAgentTool` with `SharedState`, but no long-lived named-role subagents (e.g., a persistent "reviewer" agent). Claude Code 2.1.x now has sub-agents as a first-class feature with an Agent SDK.
2. **Tool-search deferral / lazy tool loading** — Claude Code 2.1.121 added `alwaysLoad` for MCP tools, implying a tool-search system where tools are loaded on-demand from large registries. yoyo loads all tools upfront.
3. **Voice mode** — Claude Code has voice input mode. yoyo does not.
4. **IDE integrations** — Claude Code has VS Code extension, JetBrains plugin, Chrome extension, desktop app, web app, and Slack integration. yoyo is terminal-only.
5. **Computer use** — Claude Code has a computer-use preview. yoyo does not.
6. **Remote Control API** — Claude Code can be controlled via a REST API. yoyo has no remote interface.
7. **Skill marketplace curation** — yoyo has install/discovery but no signed bundles, ratings, or reviews.

### vs Aider
- Aider supports GPT-5, Grok-4, o3-pro. yoyo's provider list is broad (25 backends) but specific model support isn't always current.
- Aider has mature edit formats (whole, diff, udiff). yoyo uses Claude's native edit_file tool.

### vs Codex CLI
- Codex has ChatGPT plan integration and desktop app. yoyo is terminal-only.

## Bugs / Friction Found

1. **Flaky test: destructive_guard CWD race (Issue #364)** — `destructive_guard_allows_destructive_in_temp_dir` uses `std::env::set_current_dir` which is process-global, creating a race with parallel tests. Filed as Issue #364 with a clear fix: make `destructive_guard` take `cwd` as a parameter. This has caused at least one CI failure in the skill-evolve workflow.

2. **Large files still accumulating** — 9 files exceed 2,000 lines. The consolidation arc has been productive (12+ extractions in the last 10 sessions) but the biggest files (`format/markdown.rs` at 2,864, `cli.rs` at 2,674, `prompt.rs` at 2,425) haven't been touched. These are harder to split because they're more cohesive than the command files were.

3. **commands_refactor.rs extraction incomplete** — Day 63 evening planned to extract rename and move subsystems but only 1/3 shipped. The refactor subsystems (`commands_rename.rs` and `commands_move.rs`) already exist as separate files, so the remaining work in `commands_refactor.rs` (945 lines) may be about the `/extract` command and the `/refactor` dispatch umbrella.

## Open Issues Summary

| # | Title | Labels |
|---|-------|--------|
| 364 | Flaky test: destructive_guard tests race on process-global CWD | agent-input |
| 341 | RLM future-capability roadmap (master tracking) | — |
| 307 | Using buybeerfor.me for crypto donations | — |
| 215 | Challenge: Design and build a beautiful modern TUI | agent-input |
| 156 | Submit yoyo to official coding agent benchmarks | help wanted |
| 141 | Add GROWTH.md - Growth Strategy | — |

No `agent-self` issues currently open. Issue #364 is the most actionable — a clear bug with a clear fix that's already causing intermittent CI failures.

## Research Findings

**Claude Code 2.1.x latest features (from CHANGELOG):**
- `/model` picker lists gateway models dynamically
- `alwaysLoad` MCP config for always-available tools (implies lazy tool loading)
- Agent SDK improvements for sub-agent orchestration
- Voice mode keybinding improvements
- Bedrock service tier selection via env var
- Remote control and headless mode improvements

**Key competitive insight:** Claude Code is moving toward platform-level features (SDK, remote control, multi-surface deployment — web, desktop, IDE, Slack). yoyo's differentiator remains the self-evolution story, open-source transparency, multi-provider support, and the skill ecosystem. The biggest functional gap is IDE integration and platform breadth, not individual features.

**Consolidation vs. features:** The codebase has been in consolidation for ~12 sessions. The trajectory shows this is productive (29/30 tasks shipped, zero CI failures), but the consolidation is approaching diminishing returns on the command-file extractions. The biggest remaining structural debts are in `format/markdown.rs`, `cli.rs`, and `prompt.rs` — all harder to split. The competitive landscape suggests it may be time to pivot toward a new capability phase, particularly around the flaky test fix (real bug, real CI impact) and something that extends yoyo's reach or user experience.
