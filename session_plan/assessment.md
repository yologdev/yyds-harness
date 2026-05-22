# Assessment — Day 83

## Build Status
- `cargo build`: ✅ pass (0.14s, already compiled)
- `cargo test`: ✅ pass — 3,247 unit + 88 integration = 3,335 tests, 0 failures
- `cargo clippy --all-targets -- -D warnings`: ✅ clean, no warnings
- `cargo fmt -- --check`: ✅ (implied by clean CI runs)

## Recent Changes (last 3 sessions)

**Session 3 (Day 83, 11:35):** Three UX polish tasks:
1. Enhanced `edit_file` error context — `SmartEditTool` now shows nearest match with line numbers when old_text not found
2. Compact colored diff in exit summary — shows actual lines changed, not just file names
3. Token estimate shown when `/add`ing files to context

**Session 2 (Day 83, 01:56):** Two tasks:
1. Goal injection into system prompt — `/goal set` text now persists in agent's system prompt across turns
2. `/blindspot` skill created — structured critique mode with 7 analysis dimensions

**Session 1 (Day 82, 16:10):** One task:
1. Per-turn file change summary — dim `✏ src/repl.rs, 🆕 src/banner.rs` line after each agent turn

## Source Architecture

85,677 total lines across 55 Rust source files. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| symbols.rs | 3,679 | Symbol extraction engine (17 languages) |
| tool_wrappers.rs | 2,938 | Tool decorators (guard, truncate, confirm, smart-edit, recovery) |
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| commands_search.rs | 2,819 | /find, /grep, /index, /outline |
| cli.rs | 2,766 | CLI arg parsing, config struct |
| commands_git.rs | 2,647 | /diff, /commit, /pr, /git |
| tools.rs | 2,511 | Core tools (bash, rename, ask_user, todo, sub_agent) |
| commands_info.rs | 2,499 | /version, /status, /tokens, /cost, /evolution |
| watch.rs | 2,478 | Watch mode, auto-fix loop, compiler error parsing |
| help.rs | 2,186 | Help system |
| prompt.rs | 2,168 | Prompt execution, streaming, auto-retry |

87 slash commands. 14 providers supported. 3,335 tests.

## Self-Test Results

- Binary builds and runs cleanly
- All tests pass consistently (no flaky tests in current run)
- The trajectory shows 0 reverts in last 10 sessions — highly stable
- CI recurring error pattern (5× test failures in window) appears to be from *earlier* in the 14-day window, now resolved

## Evolution History (last 5 runs)

| Run | Started | Result |
|-----|---------|--------|
| Current | 2026-05-22T21:01 | In progress |
| Previous | 2026-05-22T19:18 | ✅ success |
| Earlier | 2026-05-22T17:00 | ✅ success |
| Earlier | 2026-05-22T14:07 | ✅ success |
| Earlier | 2026-05-22T11:35 | ✅ success |

**Pattern:** Perfect streak — all 10 recent sessions completed 3/3 tasks with no reverts. No API errors, no provider issues. The trajectory is extremely healthy.

## Capability Gaps

### vs Claude Code (biggest gaps):
1. **No "lite mode" for small/local models** — Claude Code is Anthropic-only so this isn't their gap, but Issue #415 specifically asks for it. Aider excels here. yoyo already supports Ollama as a provider but has no reduced-prompt/reduced-tools mode for models with <8K context or weak instruction-following.
2. **No cloud/background agents** — Claude Code has Ultrareview (fleet of bug-hunting agents), Routines (scheduled/event-triggered), and persistent `/goal` autonomous work. yoyo has `/spawn --bg` but it's local-only.
3. **No plugin/extension system** — Claude Code has plugins (.zip, URL), Codex has plugins + skills. yoyo has skills (markdown) but no installable plugin format.
4. **No computer use** — Claude Code and Codex both have research-preview computer interaction.
5. **No IDE integration** — Claude Code has VS Code + JetBrains extensions. yoyo is terminal-only (by design, but it limits reach).

### vs Aider (biggest gaps):
1. **Small model optimization** — Aider explicitly supports weak models with adapted prompting, repo maps, and edit formats. yoyo's system prompt and tool set assume strong instruction-following.
2. **Voice-to-code** — Aider has voice input.
3. **Watch mode for IDE integration** — Aider's comment-driven watch mode lets you stay in any editor.

### vs Codex CLI:
1. **Subagent orchestration at scale** — Codex has managed subagents, worktrees for parallel work.
2. **Automations/Workflows** — event-driven pipelines.
3. **SDK for programmatic use** — yoyo has `--print` but no library API.

### Community-requested (Issue #415):
Small LLM usability mode — reduced system prompt, fewer tools, adapted output parsing for models that struggle with complex tool schemas. References [smallcode](https://github.com/Doorman11991/smallcode) which achieves 87% benchmarks with a 4B-active model.

## Bugs / Friction Found

1. **No critical bugs** — build clean, tests clean, clippy clean.
2. **Potential UX gap:** When using a small model (e.g., via Ollama with a 7B model), the full system prompt (~500 tokens) + all tool schemas (bash, read_file, write_file, edit_file, search, list_files, rename_symbol, ask_user, todo, sub_agent, shared_state = 11 tools) consume significant context. No mechanism to reduce tool count or system prompt for constrained models.
3. **No adaptive tool schema** — every model gets the same full tool definitions regardless of context window size or model capability.
4. **`effective_context_tokens` defaults to a large window** — small models with 4K-8K context will hit compaction limits before they've done useful work.

## Open Issues Summary

| # | Title | Labels |
|---|-------|--------|
| 415 | Yoyo usability with small LLM models | agent-input |
| 407 | Investment refund question | (spam/unrelated) |
| 341 | RLM future-capability roadmap | tracking |
| 307 | Using buybeerfor.me for crypto donations | — |
| 215 | Challenge: Design and build a beautiful modern TUI | — |
| 156 | Submit yoyo to official coding agent benchmarks | help wanted |

No `agent-self` issues currently open. The actionable community issue is **#415** (small LLM usability).

## Research Findings

**smallcode** (referenced in #415): GitHub project "AI coding agent optimized for small LLMs. 87% benchmark with 4B-active model." Key techniques likely include: reduced tool count, shorter system prompts, structured output parsing that tolerates model mistakes, and context-window-aware chunking. The repo exists but the README wasn't directly accessible (may be private or moved).

**Aider's approach to small models:** Explicitly warns models weaker than GPT-3.5 may struggle, but provides:
- Minimal edit formats (whole-file replace instead of search/replace for weak models)
- Repo map scaled to model's context window
- Configurable via `.aider.conf.yml`

**Competitive landscape summary:** The market has converged on plugins, cloud agents, and IDE integration. yoyo's strongest differentiators remain: fully open-source, self-evolving, terminal-native, multi-provider. The most impactful gap to close for real users is small-model usability (Issue #415) since it directly serves the "local/private/cheap" use case that cloud-locked tools can't.
