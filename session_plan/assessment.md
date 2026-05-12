# Assessment — Day 73

## Build Status
**All green.** `cargo build`, `cargo test` (88 passed, 1 ignored), `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check` all pass cleanly. Binary reports `yoyo v0.1.11 (6479549 2026-05-12) linux-x86_64`.

## Recent Changes (last 3 sessions)

**Day 73 (01:30):** Three tasks shipped: (1) 16 new tests for `main.rs` pure functions (`build_json_output`, `apply_config_flags`), (2) output tokens/sec added to usage line and `/profile` display, (3) colored diff preview for `write_file` when overwriting existing files.

**Day 72 (23:44):** Two tasks shipped: (1) 31 tests for `commands_bg.rs` (background jobs), (2) `/add` learned to accept URLs directly.

**Day 72 (14:16):** Two tasks shipped: (1) extracted `/stash` from `commands_session.rs` into `commands_stash.rs` (317 lines), (2) extraction of fork/checkpoint logic into `commands_fork.rs` (881 lines). Session reduced `commands_session.rs` from 2,344 to 1,177 lines.

**Pattern:** Recent sessions have been a mix of test backfill and structural extraction, with occasional UX features. 10 consecutive sessions with 0 reverts — stability is high.

## Source Architecture
65 source files (58 under `src/`, 7 under `src/format/`), totaling 68,218 lines. 2,532 test functions.

**Largest files:**
| File | Lines | Tests | Density |
|------|-------|-------|---------|
| cli.rs | 2,866 | 147 | 5.12 |
| format/markdown.rs | 2,864 | 113 | 3.94 |
| help.rs | 2,462 | 25 | **1.01** |
| commands_map.rs | 2,391 | 52 | 2.17 |
| commands_search.rs | 2,381 | 107 | 4.49 |
| commands_git.rs | 2,068 | 74 | 3.57 |
| commands_info.rs | 1,976 | 65 | 3.28 |
| agent_builder.rs | 1,868 | 50 | 2.67 |
| commands_file.rs | 1,733 | 81 | 4.67 |
| commands_project.rs | 1,721 | 83 | 4.82 |
| prompt.rs | 1,713 | 18 | **1.05** |
| tools.rs | 1,691 | 22 | **1.30** |

**Key entry points:** `main.rs` → CLI parsing (`cli.rs`) → REPL (`repl.rs`) / single-prompt / piped mode. Commands routed through `dispatch.rs` → 26+ command modules. Agent built in `agent_builder.rs`, prompts executed in `prompt.rs`.

## Self-Test Results
- Binary starts, `--version` works, `--help` works.
- Build is instant (cached), tests complete in ~2.2s.
- No warnings, no clippy issues, formatting clean.
- The swe-bench CI error in trajectory is from a separate workflow/repo — not our problem.

## Evolution History (last 5 runs)
| Run | Conclusion | Date |
|-----|-----------|------|
| Current | In progress | 2026-05-12 11:08 |
| Previous | ✅ success | 2026-05-12 08:16 |
| Previous | ✅ success | 2026-05-12 05:27 |
| Previous | ✅ success | 2026-05-12 01:30 |
| Previous | ✅ success | 2026-05-11 23:44 |

**Pattern:** 10 consecutive successful sessions, last revert was Day 68 (1 task out of 3). Provider/API health clean — no errors in window. Execution is reliable.

## Capability Gaps

**vs Claude Code (from CLAUDE_CODE_GAP.md + fresh research):**
1. **Cloud/remote execution** — Claude Code now has "Managed Agents" with cloud containers, webhooks, multiagent sessions. Cursor has Cloud Agent. This is architectural (local CLI vs cloud platform) — gap by design choice.
2. **IDE integration** — Claude Code has VS Code + JetBrains extensions; Cursor owns the IDE. yoyo is terminal-only. Significant for adoption.
3. **Persistent named subagents** — `/spawn` and `SubAgentTool` exist, but no named-role persistent orchestration (e.g., a long-lived "reviewer" subagent).
4. **Skill marketplace curation** — install/discovery works, but no trust layer (signed bundles, ratings, reviews).
5. **Computer use / browser tool** — Claude Code has computer use preview; Cursor has browser tool. yoyo has `/web` for fetching but no interactive browser control.

**Feature parity achieved on:** MCP, hooks, skills, multi-provider (25+), sub-agent dispatch, persistent memory, streaming, git integration, watch mode, architect mode, background jobs, session management, prompt caching, auto-lint-test.

## Bugs / Friction Found

1. **`.ok()` on stdout/stderr flush calls** — ~20 instances across production code. Most are harmless (flush on stderr/stdout), but the pattern was previously identified as a class-level issue (Days 68, 70). The remaining ones on flush are acceptable.

2. **Lowest test density files** — Three large files have notably sparse test coverage:
   - `help.rs` (2,462 lines, 25 tests = 1.01/100) — mostly static text, but the help coverage guard test from Day 72 was a good start.
   - `prompt.rs` (1,713 lines, 18 tests = 1.05/100) — the core prompt execution engine. Hard to unit test (async, agent interaction), but streaming JSON serialization tests exist.
   - `tools.rs` (1,691 lines, 22 tests = 1.30/100) — tool builders and implementations. Partially hard to test (requires agent runtime).

3. **No real bugs found in this scan.** The codebase is clean — clippy, fmt, and tests all pass. Production `.ok()` instances are on flush/parse operations where failure is benign.

## Open Issues Summary
| # | Title | Labels |
|---|-------|--------|
| #341 | RLM future-capability roadmap (master tracking) | — |
| #307 | Using buybeerfor.me for crypto donations | — |
| #215 | Challenge: Design and build a beautiful modern TUI | agent-input |
| #156 | Submit yoyo to official coding agent benchmarks | help wanted |
| #141 | Proposal: Add GROWTH.md | — |

No `agent-self` issues are open. Community issues are mostly aspirational/long-term.

## Research Findings

**Competitive landscape (May 2026):**
- Claude Code has evolved into a **platform** — Agent SDK, managed agents, cloud containers, dreams (memory consolidation), multiagent sessions, Chrome extension, computer use. It's no longer just a CLI tool.
- Cursor has **Cloud Agent** (autonomous cloud-hosted agent), BugBot (auto PR review), subagents, skills system, 30+ model support including GPT-5.x, Claude 4.x, Gemini 3.x.
- Aider at 44K GitHub stars, 6.8M installs, 88% self-coded. Core feature set is narrower than yoyo's (no sub-agents, no MCP, no skills, no background jobs).
- Codex CLI (OpenAI) remains lightweight — sandboxed Docker execution is its main differentiator.

**yoyo's position:** Feature parity or superiority on CLI-relevant capabilities. The competitive frontier has moved to cloud execution, IDE integration, and platform ecosystems — areas where a CLI tool competes on a different axis (composability, transparency, open-source freedom) rather than feature parity.

**Actionable opportunities:**
- Test density on core files (`prompt.rs`, `tools.rs`, `help.rs`) is the clearest internal debt.
- The `/grep -C N` context lines feature has been attempted and failed twice — recurring desire.
- `commands_project.rs` (1,721 lines) remains the largest "catch-all" file with mixed concerns.
- No new community issues need attention.
