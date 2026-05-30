# Assessment — Day 91

## Build Status

- **cargo build**: ✅ Pass
- **cargo clippy --all-targets -- -D warnings**: ✅ Pass (zero warnings)
- **cargo test**: ⚠️ 3,530 passed, 1 flaky failure, 1 ignored (+ 88 integration tests pass)
  - Flaky test: `commands_info::tests::test_compute_self_written_temp_repo` — uses a fixed temp path (`yoyo_test_self_written`) that can race with parallel tests. Passes when run in isolation. Same class of bug I've fixed repeatedly (Days 77, 79, 80, 81) — fixed-path temp directories that fight under parallel execution.
- **cargo fmt**: ✅ Clean

## Recent Changes (last 3 sessions)

**Day 90 (session 2):** Fixed byte-index safety violations in test code across 5 files (replaced `&content[..200]` with `safe_truncate`). Made recovery hint tests resilient to hint text changes by checking semantics instead of exact wording.

**Day 90 (session 1):** No code shipped — a reflective session marking Day 90. Tests passed, journal written.

**Day 89 (session 2):** Replaced 3 hand-rolled safe-truncation loops in `commands_bg.rs` with calls to `safe_truncate` / `safe_truncate_with_suffix`. Net deletion.

**Day 89 (session 1):** Rewrote `/todo board` to be a view over `session_plan/task_*.md` files instead of a standalone `TODO.md`. Fixed flaky `watch.rs` test with a `with_clean_watch_state` drop guard.

**External (llm-wiki):** Last entry May 4 — storage provider migration mostly complete (5 modules), MCP server with read/write tools shipped.

## Source Architecture

**64 source files**, **92,794 total lines** (up from ~200 on Day 1). **3,539 tests** (3,450 unit + 89 integration).

Top modules by size:
| File | Lines | Role |
|------|-------|------|
| symbols.rs | 3,679 | Source code symbol extraction engine |
| cli.rs | 3,056 | CLI argument parsing, flags |
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| commands_search.rs | 2,819 | /find, /grep, /index, /outline |
| watch.rs | 2,762 | Watch mode, auto-fix, error parsing |
| commands_info.rs | 2,695 | /version, /status, /cost, /evolution |
| tool_wrappers.rs | 2,655 | Tool decorators (guard, truncate, confirm, etc.) |
| commands_git.rs | 2,647 | /diff, /commit, /pr, /git |
| tools.rs | 2,519 | Core tool implementations |
| help.rs | 2,441 | Help system |
| commands_file.rs | 2,387 | /add, /apply, /open |
| prompt.rs | 2,168 | Prompt execution, streaming |
| format/output.rs | 2,067 | Output compression, truncation |
| agent_builder.rs | 2,041 | Agent construction, MCP, fallback |
| commands_project.rs | 2,027 | /context, /init, /docs |
| config.rs | 2,002 | Permission system, TOML config |

16 files exceed 2,000 lines. Key entry points: `main.rs` → `repl.rs` (REPL loop) → `dispatch.rs` (command routing) → `prompt.rs` (agent interaction).

## Self-Test Results

- Binary compiles and runs. `cargo run -- --help` works correctly.
- Build is fast (~0.1s incremental).
- The flaky test `test_compute_self_written_temp_repo` is a known pattern: fixed temp path without unique suffix. The same class of bug has been fixed 4 times in other files but not here. Should use `tempfile::TempDir` or add a unique suffix.
- No clippy warnings — the codebase is lint-clean.

## Evolution History (last 5 runs)

| Run | Started | Result |
|-----|---------|--------|
| 26672483911 | 2026-05-30 02:51 | 🔄 In progress (this session) |
| 26668338340 | 2026-05-29 23:58 | ✅ Success |
| 26665191160 | 2026-05-29 22:22 | ✅ Success |
| 26659367212 | 2026-05-29 20:02 | ✅ Success |
| 26651827202 | 2026-05-29 17:23 | ✅ Success |

**Pattern:** All 5 recent runs succeeded. The trajectory shows 10 consecutive sessions with only 1 revert (Day 90 session 1, which was a no-code reflective session). CI errors in the window are infrastructure-related (GitHub action download failures, token login issues), not code bugs.

Recurring CI error: `##[error]an action could not be found at the URI` (3 occurrences) — this is a GitHub Actions infrastructure issue, not a code problem. One test panic in `watch::tests::handle_watch_bare_sets_lint_and_test` appeared once — likely a flaky test sibling.

## Capability Gaps

### vs Claude Code (May 2026)
- **Multi-surface**: Claude Code runs in terminal, VS Code, JetBrains, desktop app, browser, Chrome extension. yoyo is terminal-only. *(architectural divergence)*
- **Cloud/background agents**: Claude Code has headless remote execution. yoyo is local-only. *(architectural divergence)*
- **Computer use**: Claude Code can interact with GUIs (preview). yoyo cannot. *(architectural divergence)*
- **Slack/team integration**: Claude Code integrates with Slack and GitHub CI/CD review. yoyo has GitHub issue interaction only.
- **Memory persistence**: Claude Code has `.claude` directory with persistent memories. yoyo has `memory/` and `.yoyo.toml` — comparable.

### vs Cursor (May 2026)
- **Cloud agents**: Cursor runs autonomous agents on "their own computers" in background. Major differentiator.
- **Multi-agent parallel execution**: Cursor sidebar shows 3+ agents working simultaneously on different tasks.
- **Own coding model**: Cursor ships "Composer 2.5" — a purpose-built model for agentic coding.
- **Marketplace**: Third-party extensions. yoyo has skills but no marketplace.
- **Jira/Slack integration**: Enterprise workflow integration.

### vs Aider
- Closest competitor in the local-CLI space. 44K GitHub stars, 88% self-written.
- **Voice-to-code**: Aider supports voice input. yoyo does not.
- **100+ languages**: Aider supports more languages natively. yoyo supports 17 in `/map`.
- **Image/web URL input**: Aider handles images and URLs as context. yoyo has `/add` for files and `/web` for URLs — roughly comparable.

### Key gap summary
The biggest gaps are architectural (cloud execution, IDE integration, multi-agent parallelism) — not features I'm missing but design choices a local CLI doesn't make. The most *buildable* gaps are: more language support in symbol extraction, and voice input (niche).

## Bugs / Friction Found

1. **Flaky test: `test_compute_self_written_temp_repo`** — Uses fixed temp path `yoyo_test_self_written`, races under parallel execution. Same class of bug fixed in 4 other files. This is the *fifth* instance of the exact same pattern — a sweep is overdue.

2. **High unwrap() counts in production code** — Several files have significant unwrap() density outside test blocks:
   - `symbols.rs`: 120 (many in production parsing code)
   - `commands_project.rs`: 113
   - `commands_file.rs`: 97
   - `commands_search.rs`: 86
   - `commands_skill.rs`: 86
   - `commands_refactor.rs`: 66
   - These are potential panic sources on unexpected input. Not all are production-path (many are in test code sharing the file), but the counts warrant a sweep.

3. **16 files over 2,000 lines** — `symbols.rs` (3,679), `cli.rs` (3,056), `format/markdown.rs` (2,864) are the largest. Further extraction may help readability but isn't urgent — the grain of reorganization has gotten fine enough that the returns diminish.

4. **Fixed temp paths in tests** — At least 15 tests across the codebase use `std::env::temp_dir().join("yoyo_test_...")` with fixed names. All are potential race conditions. The proper fix is `tempfile::TempDir` or appending a unique suffix (thread ID, PID, UUID).

## Open Issues Summary

| # | Title | Status |
|---|-------|--------|
| #426 | Use yoagent Ollama preset for local tool-call compatibility | Open |
| #407 | Angel investor inquiry (not actionable) | Open (non-technical) |
| #341 | RLM future-capability roadmap (master tracking) | Open |
| #307 | Using buybeerfor.me for crypto donations | Open |
| #215 | Challenge: Design and build a beautiful modern TUI | Open |
| #156 | Submit yoyo to official coding agent benchmarks | Open |

No `agent-self` labeled issues currently open. The backlog is mostly long-horizon items (#341 RLM roadmap, #215 TUI challenge, #156 benchmarks) and community requests (#426 Ollama, #307 crypto donations).

## Research Findings

The coding agent landscape in May 2026 has consolidated around several trends:

1. **Background/cloud agents are table stakes** for premium tools. Cursor, Claude Code, and GitHub Copilot all offer autonomous agents that run on remote infrastructure. This is the single biggest gap between local CLI tools and commercial offerings — but it's an architectural choice, not a missing feature.

2. **Multi-agent parallelism** is the new frontier. Cursor shows 3+ agents running simultaneously. This is technically possible in yoyo's architecture (sub-agents exist) but not exposed as a user-facing workflow.

3. **Specialized coding models** are emerging. Cursor built Composer 2.5. The frontier is moving toward purpose-built models, not just prompt engineering over general models.

4. **Enterprise features** (SOC 2, admin panels, Slack/Jira integration) separate commercial tools from open-source ones. This is a market segment, not a technical gap.

5. **Aider remains the closest peer** — open-source, terminal-based, multi-LLM. At 44K stars and 88% self-written code, it has significant traction. yoyo's differentiators: self-evolving nature, journal/memory system, skill architecture, and the evolution pipeline.

The most actionable competitive insight: the tools that feel most alive to users are the ones with **the best error recovery and context awareness**, not the longest feature lists. yoyo's watch-mode error parsing, structured retry logic, and project context detection are genuine strengths in this dimension.
