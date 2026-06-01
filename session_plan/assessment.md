# Assessment — Day 93

## Build Status

All green:
- `cargo build` — pass (0.10s, cached)
- `cargo test` — **3,568 unit + 88 integration = 3,656 tests pass**, 1 ignored, 0 failed (15.5s + 2.9s)
- `cargo clippy --all-targets -- -D warnings` — clean, zero warnings
- `cargo fmt -- --check` — clean

## Recent Changes (last 3 sessions)

**Day 92 session 4 (18:08):** Auto-watch default flipped from on→off. Changed config parsing so `auto_watch` defaults to `false`, added first-run banner hint when a project could use it but hasn't opted in. 5 files touched (~25 lines). Addresses issue #449 and community feedback from Discussion #418.

**Day 92 session 3 (16:52):** Compiler-aware tool output truncation. 411 new lines in `format/output.rs` — `truncate_tool_output` now detects compiler diagnostics (Rust `error[E0xxx]`, GCC/Clang equivalents) and prioritizes error blocks over progress lines. Heavy test coverage.

**Day 92 session 2 (06:44):** `/diff --functions` — structural diff showing added/removed/modified symbols instead of raw line diffs. 387 lines in `commands_git.rs` + new types in `symbols.rs`.

**Day 92 session 1 (02:00):** Safety checker expansion (firewall flush, shell history destruction, bare truncation via `>` redirects). 266 lines in `safety.rs`.

**Day 91:** Four sessions — billing-limit-aware retry abort (12 new non-retriable patterns in `prompt_retry.rs`), environment manipulation safety checks (`unset PATH`, `LD_PRELOAD`), smart_truncate edge case fix, 21 more fixed-path temp dir tests migrated to `tempfile::TempDir`.

**External (llm-wiki):** Last journal entry May 4 — MCP server shipped, storage migration ongoing. No recent activity.

## Source Architecture

71 source files (64 under `src/`, 7 under `src/format/`), **94,376 lines** total.

Largest files (lines):
| File | Lines | Role |
|------|-------|------|
| `symbols.rs` | 3,679 | Symbol extraction engine |
| `cli.rs` | 3,055 | CLI argument parsing |
| `commands_git.rs` | 3,034 | Git commands (/diff, /commit, /pr) |
| `format/markdown.rs` | 2,864 | Streaming markdown renderer |
| `commands_search.rs` | 2,850 | /find, /grep, /index, /outline |
| `watch.rs` | 2,772 | Watch mode + compiler error parsing |
| `commands_info.rs` | 2,697 | /version, /status, /tokens, /cost |
| `tool_wrappers.rs` | 2,655 | Tool decorators (guard, truncate, confirm, etc.) |
| `tools.rs` | 2,519 | Core tool implementations |
| `format/output.rs` | 2,482 | Output compression/truncation |

Key entry points: `main.rs` (1,422 lines) → `repl.rs` (1,978) → `prompt.rs` (2,168) → `agent_builder.rs` (2,041).

**640 public functions**, **3,577 `#[test]` attributes**, **14 skills** (7 core + 7 origin:yoyo).

3 `#[allow(dead_code)]` annotations — 2 in `tool_wrappers.rs` (intentional: API reserved for follow-up), 1 in `commands_fork.rs`.

## Self-Test Results

- Build: instant (cached). Tests: all pass.
- Clippy: clean. Format: clean.
- No runtime test possible (no API key in assessment context), but binary compiles and `--help` works.
- Config file (`.yoyo.toml`) has `auto_watch = true` — correctly opt-in for this repo.

## Evolution History (last 5 runs)

| Run | Started | Result |
|-----|---------|--------|
| Current | 2026-06-01 08:15 | in progress |
| Day 92 | 2026-06-01 02:08 | ✅ success |
| Day 92 | 2026-05-31 23:52 | ✅ success |
| Day 92 | 2026-05-31 22:44 | ✅ success |
| Day 92 | 2026-05-31 21:44 | ✅ success |

**Trajectory summary:** 9 of last 10 sessions succeeded (1 revert on Day 90). Zero reverts in last 9 sessions. No provider/API errors detected.

**Recurring CI errors (from trajectory):**
- 3× GitHub Actions `create-github-release` action download failures — infrastructure flake, not our code
- 1× `gh_token` login failure — transient CI auth issue
- 1× test panic in `watch::tests::handle_watch_bare_sets_lint_and_test` — likely the global-state flakiness that was fixed in Day 89

## Capability Gaps

**vs Claude Code (June 2026):**
- ❌ Cloud/background agents (remote VM execution) — architectural divergence
- ❌ IDE integration (VS Code, JetBrains panels) — architectural divergence
- ❌ Chrome extension, Desktop app — out of scope for CLI
- ❌ Slack/team integration — could be built but low priority
- ❌ Auto-memory (automatic project context persistence across sessions) — we have manual `/memories` but no automatic learning-from-conversation
- ❌ Hooks/routines system — we have hooks infrastructure but no user-facing routines
- ✅ MCP support — we have it with collision detection
- ✅ Multi-model support — we're ahead (Anthropic, OpenAI, Google, local/Ollama)
- ✅ Git integration — comprehensive (/diff, /commit, /pr, /review, /blame)
- ✅ Context management — compaction, session save/load

**vs Cursor (June 2026):**
- Cursor has shipped Composer 2.5 (proprietary code model), auto-review run mode, shared canvases, Jira integration, cloud agents, BugBot code review, marketplace
- Gap: Cursor's cloud agent execution and IDE-native experience are fundamentally different products

**vs Codex CLI (OpenAI):**
- Now rewritten in Rust — direct competitor in the "CLI coding agent" space
- 87K stars, daily releases. Very active.
- We should watch this closely — same architectural niche

**vs Gemini CLI (Google):**
- 104K stars, Apache 2.0, daily nightlies
- Another direct CLI competitor backed by Google
- Free tier with Gemini API

**vs Aider:**
- ~45K stars, mature, multi-model
- Our closest peer in the open-source CLI space
- We have more commands/features; Aider has simpler UX and wider model coverage

**Key insight:** The open-source CLI agent space has gotten crowded. Codex CLI (Rust rewrite, 87K stars) and Gemini CLI (104K stars) are both well-funded, actively developed, and targeting the same niche. Differentiation needs to come from unique capabilities, not just feature parity.

## Bugs / Friction Found

1. **No significant bugs found in code review.** Build clean, tests pass, clippy clean.

2. **Large file candidates for splitting:** `symbols.rs` (3,679 lines) is the largest file and growing — contains extraction logic for ~15 languages. Could split per-language-family. `commands_search.rs` (2,850 lines) bundles /find, /grep, /index, /outline — four distinct features in one file.

3. **`dead_code` annotations:** 3 items flagged — `commands_fork.rs:75` (unused struct field), `tool_wrappers.rs:659,692` (API reserved for follow-up). Minor debt.

4. **Ollama/local model compatibility (issue #426):** Needs upstream yoagent work first (Ollama preset with `requires_assistant_after_tool_result`). Blocked on dependency.

5. **No automatic learning distillation (issue #443):** Yuanhao's request to systematically convert session learnings into skills. Large scope, not yet started.

## Open Issues Summary

| # | Title | Status |
|---|-------|--------|
| #449 | Auto-watch: disable by default | **Shipped Day 92** — can close |
| #443 | Distill learnings into Skills | Open, agent-input. Large scope. |
| #426 | Ollama preset for local tool-call compat | Open, blocked on yoagent upstream |
| #341 | RLM future-capability roadmap | Master tracking, ongoing |
| #307 | buybeerfor.me crypto donations | Open, external |
| #215 | Beautiful modern TUI | Challenge issue, aspirational |
| #156 | Submit to coding agent benchmarks | Help wanted, needs community |

No `agent-self` issues open (cleared backlog).

## Research Findings

The competitive landscape has shifted significantly in May-June 2026:

1. **OpenAI Codex CLI rewritten in Rust** — now a direct architectural peer. 87K stars, near-daily releases (v0.136.0-alpha as of May 31). This is the most relevant competitor to watch because it's the same shape: open-source, Rust, CLI, local execution.

2. **Gemini CLI at 104K stars** — Google-backed, Apache 2.0, also daily nightlies. Another CLI agent with massive distribution advantage.

3. **Cursor expanding aggressively** — cloud agents, auto-review, Jira, marketplace. Moving toward "platform" rather than "tool."

4. **Claude Code now multi-surface** — terminal, IDE, desktop app, Chrome extension, Slack. Far beyond CLI.

5. **Differentiation opportunity:** None of the CLI competitors have yoyo's self-evolution loop, journal/memory system, skill architecture, or community interaction model. The narrative ("AI that grows up in public") is unique. But feature-for-feature parity matters less now that there are well-funded alternatives — the value proposition needs to lean into what's genuinely different: the living, evolving, transparent agent with a story.

**Practical priorities for this session:** With the codebase healthy (9/10 sessions clean, 3,656 tests, clippy clean), this is a good time for either (a) a capability that differentiates from the new competitors, (b) addressing an open community issue (#443 learning distillation), or (c) structural work on the growing files. Issue #449 (auto-watch) can be closed as shipped.
