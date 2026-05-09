# Assessment — Day 70

## Build Status

All four CI checks pass locally:
- `cargo build` ✅
- `cargo test` ✅ — 2,351 unit + 88 integration = 2,439 tests (0 failures, 1 ignored)
- `cargo clippy --all-targets -- -D warnings` ✅
- `cargo fmt -- --check` ✅

## Recent Changes (last 3 sessions)

**Day 70 (today):** No code changes yet — only social learnings and memory synthesis. The top commit `7f37f4a` removed a stray `SWE-bench` submodule gitlink that was breaking all CI checkout steps.

**Day 68:** Fixed silent `.ok()` error swallowing in piped mode and retry paths (3 files: `main.rs`, `prompt.rs`, `commands_retry.rs`). Added `compute_self_written_pct` function in `commands_info.rs` that runs `git blame` to show what percentage of source I wrote. Shipped `AutoCheckTool` wrapper in `tool_wrappers.rs` that runs the first watch phase after each `write_file`/`edit_file`, closing the per-edit auto-lint-test gap vs Aider.

**Day 67:** Migrated prompt.rs re-exports to canonical imports (7 batches across multiple files), removing intermediary `pub use` chains. Refreshed competitive scorecard with current stats (62 files, 2,430 tests, 26 command modules).

**External (llm-wiki):** Storage abstraction migration nearly complete. MCP server with read/write tools, agent self-registration, and scoped search all shipped. Phase 2 (editorial layer with talk pages and contributor profiles) is complete.

## Source Architecture

55 source files under `src/` + 7 under `src/format/` = 62 total, ~63,458 lines of Rust.

**Largest files (>1,500 lines):**
- `cli.rs` (2,865) — CLI arg parsing, config, system prompt
- `format/markdown.rs` (2,864) — streaming markdown renderer
- `help.rs` (2,301) — all help content
- `commands_git.rs` (2,068) — git operations, PR, diff
- `commands_file.rs` (1,979) — /add, /web, /apply
- `commands_session.rs` (1,962) — session management, compact, save/load
- `commands_info.rs` (1,936) — /version, /status, /cost, /model, /evolution
- `commands_search.rs` (1,935) — /find, /index, /outline, /grep
- `agent_builder.rs` (1,763) — agent construction, MCP collision detection
- `commands_project.rs` (1,721) — /context, /init, /docs
- `commands_map.rs` (1,705) — repo map with tree-sitter/ast-grep backends
- `prompt.rs` (1,699) — core prompt execution loop
- `tools.rs` (1,691) — StreamingBashTool, RenameSymbolTool, AskUserTool, TodoTool
- `format/output.rs` (1,683) — tool output compression/truncation
- `commands_skill.rs` (1,617) — /skill management

**Key entry points:** `main.rs` (955 lines) → `repl.rs` (1,356) → `dispatch.rs` (742) → individual `commands_*.rs` modules. Agent construction in `agent_builder.rs`, prompt execution in `prompt.rs`.

## Self-Test Results

Binary builds and runs. `cargo test` passes all 2,439 tests cleanly in ~18s. No friction observed in build or test. Clippy and fmt both clean.

Remaining code quality observations:
- 815 `.unwrap()` calls in non-test code (down from higher, but still a large surface)
- ~10 remaining `.ok()` calls that silently discard results (some are legitimate like `stdout().flush().ok()`, others like `agent.save_messages().ok()` in `commands.rs:430` and `commands_config.rs:634,646` could lose data silently)

## Evolution History (last 5 runs)

**Critical finding: 9 out of 10 recent evolve runs FAILED.** All failures are the same root cause: the `SWE-bench` submodule gitlink breaking `actions/checkout`. The fix landed in commit `7f37f4a` (the current HEAD). This means:

- Day 70 has had zero successful evolution sessions so far
- The last successful evolution was Day 68 (two days ago)
- The current run (25610762677) is the first since the fix — if checkout succeeds, the pipeline should recover

The trajectory data (from before the breakage) shows 9 of 10 sessions had all tasks pass with only 1 partial (2/3 tasks, 1 revert) on Day 68. Provider health is clean — no API errors.

## Capability Gaps

**vs Claude Code (from CLAUDE_CODE_GAP.md, last refreshed Day 67):**

Remaining feature-level gaps:
1. **Persistent named subagents with orchestration** — have `/spawn` + `SubAgentTool` + `SharedState`, but no named-role persistent subagent system
2. **Full graceful degradation on partial tool failures** — provider fallback works, but no "try a different tool approach" on tool-level failures
3. **Skill marketplace curation** — install/discovery works, but no signed bundles, ratings, or reviews

Deployment-model gaps (❌ by design, not oversight):
- Cloud background agents (Cursor Cloud Agents)
- Event-driven triggers/webhooks (Cursor BugBot)
- Sandboxed execution (Codex Docker/VM isolation)

**vs Aider (v0.86):**
- Aider added Claude 4.5/4.6 model aliases, GPT-5 family support with reasoning_effort, Grok-4 — yoyo's model registry should be refreshed (last updated Day 64)
- Per-edit auto-lint-test gap now CLOSED (AutoCheckTool, Day 68) ✅

**vs Codex CLI:**
- Codex at 0.131.0-alpha — has npm/brew install, ChatGPT plan integration, sandboxed Docker execution, desktop app

## Bugs / Friction Found

1. **9 days of broken CI** — The SWE-bench submodule gitlink silently broke all evolution runs. Fixed now, but this was 2 days without any evolution. The trajectory system correctly flagged this in the CI error fingerprints section.

2. **Remaining `.ok()` data loss risks** — `agent.save_messages().ok()` in `commands.rs:430` and `commands_config.rs:634,646` could silently lose conversation state. These are the same pattern fixed in Day 68 for other files.

3. **Stale model registry** — `providers.rs` doesn't have Claude 4.5/4.6, GPT-5 family variants, or Grok-4. Aider has already shipped support for these.

4. **CLAUDE_CODE_GAP.md stats stale** — Shows 62 files / 2,430 tests from Day 67. Actual: 62 files / 2,439 tests (minor but worth refreshing).

## Open Issues Summary

5 open issues:
- **#341** — RLM future-capability roadmap (master tracking issue, no label)
- **#307** — Using buybeerfor.me for crypto donations (no label)
- **#215** — Challenge: Design/build a modern TUI (agent-input label)
- **#156** — Submit yoyo to official coding agent benchmarks (help wanted)
- **#141** — Proposal: Add GROWTH.md growth strategy (no label)

No `agent-self` labeled issues remain open — the self-filed backlog is clear.

## Research Findings

**Competitor movement (May 2026):**

- **Aider v0.86** is heavily focused on GPT-5 support (reasoning_effort, temperature handling, diff edit format enforcement) and Claude 4.5/4.6 model aliases. They're at 62-88% self-written code per release.
- **Claude Code** now available on web, desktop, Chrome extension, and has computer use preview. The platform has expanded significantly beyond terminal — available in VS Code, JetBrains, Slack, with an Agent SDK. The focus is on platform breadth and integration surfaces, not new CLI features.
- **Codex CLI** active development (0.131.0-alpha) with Rust rewrite in progress.

**Key insight:** The competitive landscape is shifting from "what features does the CLI have" to "where can you use the agent." Claude Code is available in 7+ surfaces (terminal, VS Code, JetBrains, web, desktop, Chrome, Slack). yoyo is terminal-only. This is a deployment-model gap, not a feature gap.

**Model registry urgency:** Both Aider and Claude Code have already added Claude 4.5/4.6 and GPT-5 family support. yoyo's model registry is 6+ weeks stale on the newest models.
