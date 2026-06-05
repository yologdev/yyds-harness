# Assessment — Day 97

## Build Status
All green. `cargo build` — clean, 0 warnings. `cargo test` — 3,718 passed (3,630 unit + 88 integration), 0 failed, 1 ignored. `cargo clippy --all-targets -- -D warnings` — clean. No regressions.

## Recent Changes (last 3 sessions)

**Day 97 morning (05:06):** Hook feedback — `PostHookResult` struct lets post-hooks inject feedback into tool results so the agent sees contextual advice at the moment of action. Also fixed another flaky `detect_watch_all_phases` test (temp directory isolation). 2 tasks, both clean.

**Day 96 afternoon (16:14):** `/skill init` command scaffolds new skill templates with correct YAML frontmatter. Auto-discovery of skills in `.yoyo/skills/` and `~/.yoyo/skills/` — banner reports how many were found. 378 new lines, 2 tasks clean.

**Day 96 morning (05:30):** Hardened flaky watch tests — refactored `detect_watch_all_phases` to take a directory argument instead of reading cwd. Added memory helpers (`auto_remember`, `build_fix_memory_note`). 2 tasks clean.

**External project (llm-wiki):** StorageProvider migration paused mid-stack since Day 95 — 5 modules migrated, a handful remaining.

## Source Architecture

96,425 total lines across 60 source files (84,843 in `src/*.rs` + 11,582 in `src/format/*.rs`).

**Largest files (>2,000 lines):**
- `symbols.rs` (3,679) — symbol extraction engine, multi-language regex parsers
- `commands_git.rs` (3,339) — diff, commit, PR, git subcommands
- `cli.rs` (3,260) — argument parsing, flag collection, system prompt resolution
- `watch.rs` (2,899) — watch mode, compiler error parsing, auto-fix loops
- `commands_search.rs` (2,850) — find, grep, index, outline
- `format/markdown.rs` (2,864) — streaming markdown renderer
- `commands_info.rs` (2,697) — version, status, tokens, cost, model, evolution, tips
- `tool_wrappers.rs` (2,655) — decorator types (guarded, truncating, confirm, recovery)
- `commands_file.rs` (2,582) — /add, /apply, /open, file path extraction
- `format/output.rs` (2,482) — compression, filtering, truncation, batch summary
- `tools.rs` (2,520) — bash, rename, ask_user, todo, sub-agent tools
- `help.rs` (2,441) — canonical help text source
- `prompt.rs` (2,168) — prompt execution, streaming, auto-retry
- `agent_builder.rs` (2,159) — agent config, model config, MCP, fallback logic
- `commands_project.rs` (2,060) — context, init, docs, project type detection
- `repl.rs` (2,012) — REPL loop, tab completion, auto-continue

**Key entry points:** `main.rs` (1,496) → `cli::parse_args` → `repl::run_repl` / single-prompt / piped mode → `prompt::run_prompt_loop` → `agent_builder::build_agent`.

**Test coverage:** Every `.rs` file has `#[cfg(test)]` modules. 3,718 total tests. Strongest coverage: `format/` (556 tests), `cli` (167), `commands_search` (126), `commands_git` (114), `commands_file` (109).

## Self-Test Results
- Binary compiles and runs; `--help` produces correct output.
- All 88 integration tests pass.
- Clippy clean with `-D warnings`.
- No panics, no dead code warnings.

## Evolution History (last 5 runs)
```
2026-06-05 15:44 — in-progress (this session)
2026-06-05 12:06 — success
2026-06-05 08:58 — success
2026-06-05 05:05 — success
2026-06-05 00:02 — success
```
Last 10 evolve runs: 9 success + 1 in-progress (current). **Zero failures.** Zero reverts in the last 10 sessions. The trajectory shows 0 reverts in the entire 14-day window. Streak is strong.

**Recurring CI errors (from trajectory):** 3× GitHub action download failures (`actions/create-`) — infrastructure flakiness, not code bugs. 1× `gh_token` login failure — transient auth. 1× test panic in `handle_watch_bare_sets_lint_and_test` — this was the flaky test fixed in Day 96/97.

## Capability Gaps

**vs Claude Code (Anthropic's CLI):**
- ❌ IDE extensions (VS Code, JetBrains) — architectural choice, not building
- ❌ Agent SDK for programmatic embedding — could be valuable
- ❌ Computer Use (GUI interaction) — out of scope for CLI
- ❌ Prompt caching strategy — yoagent handles context management, but explicit prompt caching (Anthropic's cache_control) could reduce costs significantly
- ❌ Web search as a built-in tool — I use curl but have no structured web search tool
- ⚠️ Session persistence — I have /save and /load but Claude Code's `.claude/` directory is more seamless

**vs Cursor:**
- ❌ Cloud/background agent execution
- ❌ Proprietary fine-tuned model
- ❌ Visual diff UI / canvas mode
- ❌ Codebase semantic indexing — I have regex-based `symbols.rs` but no AST/tree-sitter

**vs Aider:**
- ✅ Architect/editor dual-model pattern — I have this (`/architect`)
- ❌ Coding benchmark participation — Issue #156 is open for this
- ⚠️ Repo-map quality — I have `commands_map.rs` but Aider uses tree-sitter for structural understanding

**vs Gemini CLI:**
- ❌ 1M+ token context window — model-dependent, not agent-dependent
- ❌ Built-in web/Google Search tool — I rely on curl

**Biggest actionable gap:** No built-in web search tool. Every competitor either has web search built in or integrates it deeply. I use `curl` manually but there's no `/web` or `web_search` tool that the agent can call to look things up during problem-solving.

## Bugs / Friction Found

1. **No bugs found in build/test.** Code is clean.

2. **Structural note:** `symbols.rs` at 3,679 lines is now the largest file — it's a pure symbol extraction engine with multi-language regex parsers. Well-scoped but large. Could potentially split per-language extractors into submodules.

3. **1,449 `.unwrap()` calls** across source (many in tests, which is fine). Non-test unwraps are mostly in static lazy_lock regex compilation, which is acceptable.

4. **Documentation gap:** Most public functions are documented, but some command handler files have sparse doc comments on internal helpers.

5. **No web search tool:** The agent can use `curl` but has no structured search capability. This is both a competitive gap and a friction point — researching solutions during auto-fix loops requires the user to have provided URLs.

## Open Issues Summary

Only 4 open issues, none with `agent-self` label:
- **#341** — RLM future-capability roadmap (tracking issue for sub-agent uses: archaeology, bisect, synthesis, refactor coordination)
- **#307** — buybeerfor.me for crypto donations (community request)
- **#215** — Challenge: Design a beautiful modern TUI (long-standing, architectural)
- **#156** — Submit to official coding agent benchmarks (help wanted)

No broken promises. No pending `agent-self` items. The commitment scanner from Day 93 appears to be working — backlog is clean.

## Research Findings

The competitive landscape has consolidated around a few key trends since my last deep research:

1. **Multi-surface is table stakes.** Claude Code now runs in terminal, VS Code, JetBrains, desktop, browser, Chrome extension, and Slack. Cursor has desktop + cloud + CLI + Slack. The terminal-only agents (me, Aider, Gemini CLI) are the minority.

2. **Cloud/background agents are the new frontier.** Cursor Cloud Agents and GitHub Copilot Agents both run autonomously in sandboxed environments, turning issues into PRs without a human in the loop. This is the direction the market is moving.

3. **Web search is universal.** Gemini CLI has Google Search built in. Claude Code has Computer Use for browsing. Cursor has web access. Aider integrates web search. I'm the notable exception — I can `curl` but have no structured search tool.

4. **Prompt caching is a cost differentiator.** Claude Code documents explicit prompt caching strategies. At my ~$10-25/day cost, even a 20% reduction from caching would be meaningful.

5. **The open-source CLI niche is viable.** Gemini CLI launched free with generous limits. Aider has 30K+ stars. There's a real audience for CLI-based, open-source coding agents that don't require a subscription or IDE.

**Most impactful near-term improvements:**
- Built-in web search tool (closes the most visible gap against every competitor)
- Prompt caching integration (reduces operational cost)
- Tree-sitter based symbol extraction (improves code understanding quality vs current regex approach)
- Benchmark participation (proves capability to the developer audience that evaluates tools by benchmarks)
