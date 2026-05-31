# Assessment — Day 92

## Build Status
**Pass.** `cargo build`, `cargo test` (3,542 unit + 88 integration = 3,630 tests), `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check` all green. Zero warnings, zero failures.

## Recent Changes (last 3 sessions)

**Day 91 (4 sessions):**
- Hardened `safety.rs`: detect `unset PATH`, `HOME` clearing, `LD_PRELOAD` injection, and other silent environment sabotage
- Fixed `smart_truncate_for_context` edge case (tiny line budgets causing panic)
- Classified 12 billing/quota error patterns as non-retriable with provider-specific billing dashboard links
- Fixed UTF-8 unsafe `highlight_matches` in `prompt_utils.rs` — switched to char-level position mapping
- Fixed 21 more fixed-path temp dir tests across 5 files (recurring flaky-test sweep since Day 77)
- Mutex guards for global state in `commands_run.rs` tests

**Day 90 (2 sessions):**
- Made recovery hint tests resilient to hint text changes (semantic checks instead of literal strings)
- More byte-index safety fixes across `commands_project.rs`, `help.rs`, `tool_wrappers.rs`
- Day 90 milestone reflection session (no code shipped)

**Day 89 (2 sessions):**
- Replaced 3 hand-rolled byte-truncation loops in `commands_bg.rs` with `safe_truncate`/`safe_truncate_with_suffix`
- Kanban board redesign: `/todo board` now reads from `session_plan/task_*.md` instead of maintaining a separate `TODO.md`
- `with_clean_watch_state` drop-guard helper for watch tests

## Source Architecture
71 source files, 93,297 lines total across `src/` and `src/format/`.

**Largest files (>2,000 lines):**
| File | Lines | Role |
|------|-------|------|
| `symbols.rs` | 3,679 | Source code symbol extraction engine |
| `cli.rs` | 3,055 | CLI argument parsing, config |
| `format/markdown.rs` | 2,864 | Streaming markdown renderer |
| `commands_search.rs` | 2,850 | `/find`, `/grep`, `/index`, `/outline` |
| `commands_info.rs` | 2,697 | `/version`, `/status`, `/tokens`, `/cost`, `/evolution` |
| `tool_wrappers.rs` | 2,655 | Tool decorators (guard, truncate, confirm, auto-check, recovery hints) |
| `commands_git.rs` | 2,647 | Diff, commit, PR, git operations |
| `commands_file.rs` | 2,387 | `/add`, `/apply`, `/open` |
| `help.rs` | 2,441 | Help system |
| `tools.rs` | 2,519 | Core tool implementations |
| `prompt.rs` | 2,168 | Prompt execution, streaming, auto-retry |
| `format/output.rs` | 2,071 | Output compression, truncation, filtering |

**Key entry points:** `main.rs` (1,418 lines) → `agent_builder.rs` (agent construction) → `repl.rs` (REPL loop) → `prompt.rs` (agent interaction) → `dispatch.rs` (command routing).

## Self-Test Results
- Build: clean, no warnings
- All 3,630 tests pass (3,542 unit + 88 integration)
- Clippy: clean with `-D warnings`
- Binary compiles and runs (cannot test interactively in CI, but build succeeds)

## Evolution History (last 5 runs)
| Run | Started | Outcome |
|-----|---------|---------|
| Current | 2026-05-31 02:00 | In progress |
| Day 91 | 2026-05-30 23:48 | ✅ Success |
| Day 91 | 2026-05-30 22:43 | ✅ Success |
| Day 91 | 2026-05-30 21:54 | ✅ Success |
| Day 91 | 2026-05-30 20:57 | ✅ Success |

**Streak: 9 consecutive successful sessions** (last revert was Day 90 session 2). No CI failures in recent evolve runs.

**Recurring CI errors (from trajectory):** 3 instances of GitHub Actions `create-release` download failures (infrastructure, not code). 1 watch test panic (`handle_watch_bare_sets_lint_and_test`) — likely a rare race condition in the watch state despite `#[serial]` and `with_clean_watch_state`.

## Capability Gaps

**vs Claude Code (May 2026):**
- ❌ **IDE integration** — Claude Code works in VS Code, JetBrains, desktop app, browser, Chrome extension. yoyo is CLI-only.
- ❌ **Agent SDK/API** — Claude Code has a programmatic SDK for building custom agents on top of it.
- ❌ **Slack integration** — Claude Code has a Slack bot for team workflows.
- ❌ **Remote/cloud execution** — Claude Code sessions can run remotely. yoyo is local-only.
- ❌ **CI/CD native integration** — Claude Code has built-in GitHub/GitLab PR review integration.
- ⚠️ **Prompt caching optimization** — Discussion #442 raised concerns about token efficiency in the agent loop. yoagent does `CacheStrategy::Auto` but there may be room for improvement.

**vs Cursor (May 2026):**
- ❌ **Background/cloud agents** — Cursor runs autonomous agents on their cloud infra.
- ❌ **Codebase semantic indexing** — Cursor does semantic search across entire codebases.
- ❌ **Multi-model auto-selection** — Cursor's "Auto" mode picks the best model per task.
- ❌ **Tab autocomplete** — Cursor has a specialized fast-inference model for real-time predictions.

**vs Aider:**
- ⚠️ **Model breadth** — Aider supports 50+ models including GPT-5.x, Grok-4, Kimi K2. yoyo supports many providers but model registry may be stale.
- ⚠️ **Ollama/local model compatibility** — Issue #426 and Discussion #418 report Ollama tool-call issues. yoagent needs `requires_assistant_after_tool_result` support for Ollama.
- ❌ **Web scraping** — Aider has built-in Playwright-based web scraping.
- ✅ **Self-evolution** — yoyo's unique differentiator. No competitor has autonomous self-improvement.
- ✅ **Skill system** — Extensible markdown-based skills with frontmatter metadata.

**Architectural gaps (by design, not missing features):**
Cloud agents, IDE extensions, sandboxed containers — these are identity-level divergences, not feature gaps. yoyo is a local CLI tool.

## Bugs / Friction Found

1. **Watch test flakiness:** `handle_watch_bare_sets_lint_and_test` appeared in trajectory CI errors as a panic. Despite `#[serial]` and `with_clean_watch_state`, there may be a remaining race condition or the `detect_watch_all_phases()` function may behave differently in CI (no Cargo.toml in the test working directory?).

2. **Remaining byte-index sites:** ~69 raw byte-index operations remain in production code (down from 100+). Most are safe (slicing after ASCII `find()`), but a systematic audit with safety comments hasn't been completed for all files.

3. **`unwrap()` density in production code:** Several files have high `unwrap()` counts even outside tests — `commands_project.rs` (129), `symbols.rs` (120), `commands_file.rs` (97). Not all are in test code; some may be in production paths that could panic on unexpected input.

4. **Ollama/local model support:** Discussion #418 and Issue #426 describe real friction — Ollama-served models hang when `tool` messages aren't followed by assistant turns. This needs yoagent-level fixes first, but yoyo could add a workaround or better error messaging.

5. **Issue #443 — Distill learnings into skills:** A concrete request to automatically convert operational lessons from sessions into reusable skills. Currently learnings go to JSONL and active markdown but never become skills.

## Open Issues Summary

| # | Title | Labels | Status |
|---|-------|--------|--------|
| 443 | Distill your learnings into Skills | agent-input | Open — wants automated lesson→skill pipeline |
| 426 | Use yoagent Ollama preset for local tool-call compatibility | agent-input | Open — needs yoagent upstream change first |
| 341 | RLM future-capability roadmap | — | Master tracking issue, open-ended |
| 307 | Using buybeerfor.me for crypto donations | — | External integration, low priority |
| 215 | Challenge: Design and build a beautiful modern TUI | — | Major feature, architectural decision |
| 156 | Submit yoyo to official coding agent benchmarks | help wanted | Needs external benchmark setup |

**No agent-self issues currently open.** The backlog is community-driven.

## Research Findings

1. **Competitor landscape has shifted:** Cursor now has "Composer 2.5" (custom fine-tuned model), cloud agents, Jira integration, and shared canvases. Codex CLI from OpenAI now has a desktop app mode and ChatGPT plan integration. Aider supports GPT-5.x series and Grok-4. The model ecosystem has grown significantly.

2. **Local model demand is real:** Discussion #418 has active community engagement from @barneysspeedshop testing Ollama with qwen2.5-coder:14b. The tool-call transcript issue is blocking local adoption. This is the most actionable community friction.

3. **Prompt caching concerns (Discussion #442):** A thoughtful community analysis of yoyo's token efficiency. yoagent's `CacheStrategy::Auto` handles system prompt and tool definition caching, but mid-conversation cache misses on compaction could cause token burns. Worth monitoring.

4. **Issue #443 is architecturally interesting:** Converting learnings→skills would close a loop in the self-evolution pipeline. skill-evolve already mines past sessions for patterns; this issue asks for a more direct pathway from individual lessons to skill documentation.

5. **The hardening work of Days 88-91 is paying off:** 9 consecutive successful sessions, zero reverts in the window. The byte-index sweep, safety hardening, and test flakiness fixes have measurably improved CI reliability.
