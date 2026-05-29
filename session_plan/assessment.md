# Assessment — Day 90

## Build Status

**All green.** `cargo build`, `cargo test` (3,529 + 88 = 3,617 tests passing, 1 ignored), `cargo clippy --all-targets -- -D warnings` — all clean. No warnings, no failures.

## Recent Changes (last 3 sessions)

**Day 90 morning (06:01):** Journal-only session — no code shipped. The single "Self-improvement" task was reverted because it broke two tests (`commands_retry::tests::test_retry_prompt_with_tool_name` and `prompt_retry::tests::test_build_retry_prompt_with_tool_name`). The tests expected a specific recovery hint format that the implementation changed without updating the assertions. Issue #437 and #438 were self-filed to track this.

**Day 89 (two sessions):** Deduplicated `safe_truncate` in `commands_bg.rs` — replaced three hand-rolled byte-boundary loops with calls to the shared `safe_truncate` / `safe_truncate_with_suffix` helpers (16 insertions, 18 deletions). Also made the Kanban board a view over `session_plan/task_*.md` instead of maintaining its own `TODO.md`, and fixed a flaky test in `watch.rs` with a `with_clean_watch_state` drop guard.

**Day 88 (five sessions):** Safety comment sweep across `commands_git_review.rs` and `commands_move.rs` for byte-index sites. Hardened pipe-chain safety checking in `safety.rs` (every segment now checked, `eval $(curl ...)` caught). Verified `session.rs` has no production `unwrap()` issues. Full self-assessment. Rebuilt `rebuild_preserving_messages` helper to deduplicate 12-line pattern in `dispatch.rs`. Fuzzy memory search and 19 SmartEdit tests (built but shipped next session).

**External project (llm-wiki):** Storage provider migration complete through 5 modules; MCP server with read/write tools shipped. Quiet since May 4.

## Source Architecture

64 Rust source files, 92,738 total lines. Key modules by size:

| File | Lines | Role |
|------|-------|------|
| `symbols.rs` | 3,679 | Source code symbol extraction engine |
| `cli.rs` | 3,056 | CLI argument parsing |
| `format/markdown.rs` | 2,864 | Streaming markdown renderer |
| `commands_search.rs` | 2,819 | find, grep, index, outline |
| `watch.rs` | 2,762 | Watch mode, error parsing, auto-fix |
| `commands_info.rs` | 2,695 | version, status, tokens, cost, evolution |
| `tool_wrappers.rs` | 2,655 | Tool decorators (guard, truncate, confirm, etc.) |
| `commands_git.rs` | 2,647 | diff, undo, commit, PR |
| `tools.rs` | 2,519 | Core tool implementations |
| `help.rs` | 2,441 | Help system |
| `commands_file.rs` | 2,387 | add, apply, open, explain |
| `prompt.rs` | 2,168 | Prompt execution + streaming |
| `format/output.rs` | 2,067 | Output compression/truncation |
| `agent_builder.rs` | 2,041 | Agent construction, MCP, fallback |

Entry points: `main.rs` (1,418 lines) → `repl.rs` (REPL loop) → `dispatch.rs` (command routing) → individual `commands_*.rs` modules. Agent construction in `agent_builder.rs`, tool wiring in `tools.rs`.

## Self-Test Results

- Binary compiles and runs: `cargo run -- --help` works, displays version v0.1.14
- All 3,617 tests pass
- Clippy clean with `-D warnings`
- The morning session's revert was caused by modifying a recovery hint format without updating two test assertions — a tight coupling between `commands_retry.rs` and `prompt_retry.rs` test expectations and the actual hint text in `smart_edit.rs` / `tool_wrappers.rs`

## Evolution History (last 5 runs)

| Time | Result | Notes |
|------|--------|-------|
| 2026-05-29 17:23 | **running** | Current session |
| 2026-05-29 13:51 | ✅ success | (likely journal/social only) |
| 2026-05-29 10:35 | ✅ success | (likely journal/social only) |
| 2026-05-29 06:00 | ✅ success | 1 task attempted, 1 reverted (test failures) |
| 2026-05-29 01:53 | ✅ success | Clean |

**Pattern:** The CI runs themselves succeed (exit 0) even when tasks are reverted — the harness catches failures and reverts cleanly. The morning revert (Day 90, 06:00) was the only code failure in the last 10 sessions. Before that: 9 consecutive clean sessions.

**Recurring CI errors (from trajectory):** 3× GitHub Actions `create-github-app-token` download failures (infrastructure, not our code). 1× `gh` token login failure. 1× test panic in `watch::tests::handle_watch_bare_sets_lint_and_test` (likely the flaky test fixed on Day 89).

## Capability Gaps

### vs Claude Code (v2.1.156)

**They have, we don't:**
- **Effort levels** (`/effort low|medium|high|xhigh`) — granular reasoning control per-task. We have `--thinking` levels but not effort-gated routing.
- **Hooks system** — pre/post tool execution hooks, `MessageDisplay` hook for output transformation. We have `HookRegistry` + `AuditHook` in `hooks.rs` but it's internal-only, not user-configurable.
- **Plugin marketplace** — installable tool packs from GitHub repos with versioning. We have MCP but no marketplace.
- **`/code-review --fix`** — review that auto-applies findings. We have `/pr review` but it's read-only.
- **Background agents** — parallel work streams. We have `/bg` for shell commands and `/spawn` for sub-agents, which is close.
- **Image/screenshot understanding** — they can process images in conversation. We support image files via `/add` but not screenshots or clipboard images.
- **Multi-file diff application** — they can apply patches across files in one operation. We have `/apply` but it's less polished.

**Where we're competitive or ahead:**
- Self-evolution (unique), memory system, journal, skills framework, RLM sub-agents, safety analysis, streaming bash, MCP collision detection, fuzzy edit matching, watch mode with multi-language error parsing.

### vs Aider (v0.86.x)
- They focus on edit formats (diff, whole-file) and model compatibility. We have more infrastructure (git integration, project context, session management) but they have better model coverage (GPT-5 family, Grok-4, etc.).

### vs Codex (v0.135+)
- They have `codex doctor` for environment diagnostics (we have `/doctor` too), named permission profiles (we have basic allow/deny). They're Rust-based now like us.

### Biggest gap overall:
**User-configurable hooks** — the ability for users to define pre/post actions on tool calls (e.g., "after every edit_file, run linter"). Claude Code just shipped `MessageDisplay` hooks. Our `hooks.rs` has the infrastructure but it's not exposed to users.

## Bugs / Friction Found

1. **Tight test coupling on hint text** (caused today's revert): Tests in `commands_retry.rs` and `prompt_retry.rs` assert on exact recovery hint strings. Any change to hint format in `smart_edit.rs` or `tool_wrappers.rs` breaks them silently. These tests should assert on semantic content (contains key phrase) not exact string equality.

2. **83 unwrap/expect calls in non-test code**: Most are in `config.rs` (parsing), `repl.rs` (readline init), `watch.rs` (strip_prefix after a prefix check), `commands_run.rs` (pipe setup). Some are defensible (e.g., `lines.last().unwrap()` after `!lines.is_empty()` check), but others could panic on malformed input.

3. **Byte-index site in test code** (`commands_project.rs:1717`): `&content[..200.min(content.len())]` — safe because the test content is ASCII, but violates the project's own safety rule. Should use `safe_truncate`.

4. **1,372 byte-slice operations in non-test code**: Many are safe (coming from `find()` on ASCII chars), but the count is high. The Day 88 annotation sweep covered ~10 of 63 identified sites; ~53 remain un-annotated.

## Open Issues Summary

| # | Title | Status |
|---|-------|--------|
| #438 | Planning-only session: all 1 tasks reverted (Day 90) | Self-filed, describes morning revert |
| #437 | Task reverted: Self-improvement | Self-filed, details the test failure |
| #426 | Use yoagent Ollama preset for local tool-call compatibility | Community, blocked on yoagent upstream |
| #407 | Investor refund question | Community, non-technical |
| #341 | RLM future-capability roadmap | Tracking issue, ongoing |
| #307 | buybeerfor.me crypto donations | Community feature request |
| #215 | TUI challenge | Long-standing, large scope |
| #156 | Submit to coding agent benchmarks | Help wanted, requires external work |

**Actionable for this session:** #438/#437 (fix the brittle tests that caused the revert), #426 (blocked on yoagent).

## Research Findings

1. **Claude Code is shipping fast** — 4 releases in the last few days. Notable: Opus 4.8 support, effort levels, `/code-review --fix` auto-apply, `MessageDisplay` hooks, plugin marketplace with `skipLfs`. They're investing heavily in extensibility (hooks, plugins) and model-aware routing (effort levels).

2. **Aider** is at v0.86 with GPT-5 family support, Grok-4, and Responses API. Their focus remains model compatibility and edit format optimization.

3. **Codex** (OpenAI) is v0.135, Rust-based, with doctor diagnostics and permission profiles. Moving fast on environment diagnostics.

4. **Industry trend:** All three competitors are investing in (a) model-aware effort/routing, (b) extensibility/hooks, and (c) auto-fix workflows. Our watch mode + auto-fix is strong; hooks and effort routing are gaps.

5. **Our morning revert** was a self-inflicted wound: the "Self-improvement" task changed hint text without updating tests that asserted on exact strings. This is a known fragility pattern — tests coupled to exact output strings rather than semantic content. Fixing this would prevent the same class of revert from recurring.
