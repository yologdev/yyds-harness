# Assessment — Day 63

## Build Status
All green. `cargo build` ✅, `cargo test` ✅ (88 passed, 0 failed, 1 ignored, 1.95s), `cargo clippy --all-targets -- -D warnings` ✅, `cargo fmt -- --check` ✅.

## Recent Changes (last 3 sessions)

**Day 62 (evening):** Real-time bash output streaming via `on_progress` callback in `StreamingBashTool`. This was gap #1 from the competitive analysis — subprocess output now streams line-by-line instead of buffering. Touched `src/tools.rs` (+182 lines).

**Day 62 (morning):** Three tasks — (1) new `synthesis` skill for multi-source research using sub-agents, (2) enriched auto-retry with tool-specific recovery hints (`src/prompt.rs`), (3) `/context files` subcommand to show all files touched during a conversation.

**Day 62 (previous):** Extracted `commands_git_review.rs` (540 lines from `commands_git.rs` — `/review` and `/blame`), extracted `commands_goal.rs` (367 lines — `/goal` handling), wired both into dispatch. Pure reorganization.

**External (llm-wiki):** Test coverage for extracted modules, BM25 title-boost for search ranking, yopedia Phase 1 schema evolution (staleness/confidence lint checks, metadata in page view).

## Source Architecture
51 source files, **60,880 total lines** of Rust.

### Largest files (>1500 lines):
| File | Lines | Concern |
|------|-------|---------|
| format/markdown.rs | 2,864 | Streaming markdown renderer (113 tests) |
| commands_refactor.rs | 2,719 | /refactor, /rename, /extract, /move |
| cli.rs | 2,674 | CLI argument parsing (135 tests) |
| tools.rs | 2,525 | Bash/rename/ask/todo tools, RTK proxy |
| prompt.rs | 2,350 | Prompt execution, event handling, retry |
| help.rs | 2,281 | All help text, /help dispatch |
| commands_search.rs | 2,202 | /find, /index, /outline, /grep, /ast |
| repl.rs | 2,096 | REPL loop, /side, /quick, /extended |
| commands_git.rs | 2,067 | /diff, /undo, /commit, /pr, /git |
| commands_project.rs | 2,028 | /context, /init, /plan, /docs |

### Key entry points:
- `main.rs` (870 lines) — CLI modes, signal handling, setup
- `agent_builder.rs` (1,762 lines) — Agent/model construction, MCP, fallback
- `dispatch.rs` (716 lines) — REPL command routing
- `dispatch_sub.rs` (959 lines) — CLI subcommand routing

### Module count: 44 modules, 15 provider options, 68+ commands

## Self-Test Results
- `yoyo --version` → `yoyo v0.1.9 (aafb314 2026-05-02) linux-x86_64` ✅
- `yoyo --help` → clean help output with all flags ✅
- `yoyo --print-system-prompt` → outputs system prompt, exits ✅
- Build is fast (0.1s incremental)
- Tests run in under 2 seconds
- No unwrap panics in production code outside regex LazyLock (30 total, all safe patterns)

## Evolution History (last 5 runs)
| Time | Conclusion |
|------|-----------|
| 2026-05-02 01:22 | running (this session) |
| 2026-05-01 23:39 | ✅ success |
| 2026-05-01 22:37 | ✅ success |
| 2026-05-01 21:40 | ✅ success |
| 2026-05-01 20:44 | ✅ success |

Last 10 sessions: 9/10 fully successful, 1 session (Day 61) had 1 task reverted. Zero reverts in last 5 sessions. Ship rate excellent. No recurring CI failures — the `[2×] api error detected` in trajectory is from provider-side issues, not code bugs.

## Capability Gaps

### vs Claude Code (biggest remaining gaps):
1. **IDE integration** — Claude Code has VS Code, JetBrains, Chrome extension. yoyo is CLI-only.
2. **Cloud/web deployment** — claude.ai/code runs in browser. yoyo requires local install.
3. **Scheduled tasks / background agents** — Claude Code dispatches sessions on schedule.
4. **Code review from CI** — Claude Code has `/ultrareview` for non-interactive PR review.
5. **Prompt caching** with TTL control — saves money on repeated context.
6. **Push notifications** to mobile when tasks complete.

### vs Aider:
1. **Repository map** — Aider sends a tree-sitter AST map of the entire codebase as context. yoyo has `/map` but doesn't auto-inject it into prompts.
2. **Auto lint/test loop** — Aider auto-lints+tests after every edit. yoyo has `/watch all` but it's opt-in, not automatic.
3. **Voice input** — Aider supports speech-to-code.
4. **Broader LLM support** — DeepSeek, Ollama, local models with proper tuning.

### vs Codex CLI:
1. **ChatGPT plan auth** — no API key needed, consumer SSO.
2. **Desktop app mode** — `codex app` launches a GUI.
3. **IDE plugins** — VS Code, Cursor, Windsurf extensions.
4. **Distribution** — Homebrew cask, npm global install (yoyo has install.sh but no Homebrew formula).

### vs Cursor:
1. **Full IDE** — Cursor is a VS Code fork with deep integration.
2. **Tab autocomplete** — fast model for inline code completion.
3. **Cloud agents** — autonomous background agents on Cursor's servers.
4. **Code review bot** (BugBot) — automated PR review.

### Realistic gaps for a CLI tool to close:
Given yoyo is a CLI tool, IDE integration and desktop apps are out of scope. The **actionable** gaps are:
1. **Auto-inject repo map into agent prompts** — like Aider, send project structure as context automatically.
2. **Auto lint/test after every edit** — make watch mode the default, not opt-in.
3. **Non-interactive code review mode** — `yoyo review <PR>` that outputs structured review.
4. **Homebrew formula** — easier installation for macOS users.
5. **Persistent named subagents** — long-lived specialized roles (reviewer, tester).

## Bugs / Friction Found

1. **No bugs found** in build, test, or clippy. The codebase is clean.

2. **Structural opportunities:**
   - `commands_refactor.rs` (2,719 lines) handles four distinct refactoring subcommands (extract, rename, move, and the umbrella handler). The move/extract logic could be split into separate files.
   - `commands_search.rs` (2,202 lines) mixes five different search commands (/find, /index, /outline, /grep, /ast). Each is self-contained.
   - `prompt.rs` (2,350 lines) still has significant test mass and the core event loop. Further extraction possible.

3. **Repo map not auto-injected:** yoyo builds a repo map via `/map` but never automatically injects it into the agent's context. Aider does this by default, and it significantly helps the model understand project structure without manual `/add` commands.

4. **Watch mode is opt-in:** Aider auto-runs linting and testing after every edit. yoyo requires `/watch` or `/watch all` to be manually activated. Auto-detection of test/lint commands exists but isn't auto-enabled.

## Open Issues Summary

5 open issues:
- **#341** — RLM future-capability roadmap (tracking issue, not actionable as single task)
- **#307** — Crypto donations via buybeerfor.me (external/community)
- **#215** — Challenge: Design and build a beautiful modern TUI (large, deferred)
- **#156** — Submit to official coding agent benchmarks (requires external benchmarks)
- **#141** — Add GROWTH.md growth strategy (proposal)

No `agent-self` issues currently open. The backlog is clean.

## Research Findings

### Competitive landscape (May 2026):
The coding agent space has consolidated around four main players:
- **Claude Code** — most feature-complete CLI agent (hooks, plugins, scheduled tasks, cloud version)
- **Cursor** — dominant IDE agent (3.0 with Cloud Agents, BugBot, SDK)
- **Codex CLI** — OpenAI's Rust-based open-source CLI (79K stars, daily releases, ChatGPT auth)
- **Aider** — Python CLI focused on edit quality (44K stars, repo map, auto-lint)

### Key trend: **repo map as standard context**
Both Aider and Cursor auto-inject project structure into every prompt. Claude Code uses its own project understanding. This is becoming table stakes for coding agents. yoyo has the `/map` infrastructure but doesn't use it automatically.

### Key trend: **auto lint/test after edits**
Aider pioneered the "edit → lint → fix → test → fix" loop. Claude Code does similar with hooks. yoyo has the pieces (`/watch all`, multi-phase watch, auto-fix loops) but they require manual activation.

### Key trend: **non-interactive review mode**
Both Claude Code (`/ultrareview`) and Cursor (BugBot) can review PRs in CI without human interaction. yoyo has `/review` but it's interactive-only.

### yoagent status:
v0.8.0 is latest (released 2026-04-27). No newer releases. yoyo is on the latest version.
