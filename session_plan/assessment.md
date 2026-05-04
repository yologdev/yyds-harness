# Assessment — Day 65

## Build Status

All green:
- `cargo build` — pass (0.10s, already cached)
- `cargo test` — pass: 2,309 unit + 88 integration = **2,397 tests**, 0 failures, 1 ignored
- `cargo clippy --all-targets -- -D warnings` — pass, zero warnings
- `cargo fmt -- --check` — pass

## Recent Changes (last 3 sessions)

**Day 64 evening (23:32):** Updated model registry with GPT-5/5.5, Grok-4, Gemini 2.5 Flash Lite. Extracted `conversations.rs` (833 lines) from `repl.rs` for side/quick/extended conversation handlers. Added `/model list` subcommand to browse models by provider.

**Day 64 afternoon (14:03):** Extracted `tool_wrappers.rs` (661 lines) from `tools.rs`. Enhanced startup banner with project context detection (shows project type, name, branch). Refreshed CLAUDE_CODE_GAP.md stats to 59 files/2,391 tests.

**Day 64 morning (05:18):** Extracted `prompt_retry.rs` and `prompt_utils.rs` from `prompt.rs` (shrunk from 2,425 to 1,300 lines). Fixed race condition in safety tests where `set_current_dir` caused flaky test failures.

**llm-wiki (external):** Bulk StorageProvider migration ongoing — 5+ modules migrated to abstraction layer. MCP server with read/write tools, agent self-registration, scoped search all shipped.

## Source Architecture

53 source files, **61,748 total lines** (up from ~61,591 Day 64), ~3,316 functions.

**Largest files (>2,000 lines):**
| File | Lines | Concern |
|------|-------|---------|
| `format/markdown.rs` | 2,864 | Streaming markdown renderer (single struct, mostly tests) |
| `cli.rs` | 2,771 | CLI arg parsing, config, banner, welcome |
| `help.rs` | 2,288 | All help content: CLI help, REPL /help, per-command help |
| `commands_git.rs` | 2,067 | Git commands: diff, undo, commit, PR |

**Commands split:** 24 `commands_*.rs` files handle slash commands (25,416 lines total).
**Format subsystem:** 6 files in `src/format/` (10,553 lines).
**Core REPL loop:** `repl.rs` (1,296 lines), `dispatch.rs` (726 lines), `dispatch_sub.rs` (1,140 lines).
**Agent/prompt:** `agent_builder.rs` (1,762), `prompt.rs` (1,300), `prompt_retry.rs` (708), `prompt_budget.rs` (596), `prompt_utils.rs` (unknown — recently extracted).

**Key entry points:** `main.rs` (881 lines) → `run_repl` / `run_single_prompt` / `run_piped_mode`.

## Self-Test Results

- Build: instant (cached), clean
- Tests: all pass in ~7s unit + ~2s integration
- No clippy warnings
- `run_repl` is still 985 lines (line 312 to EOF) — the single largest function in the codebase, though much reduced from its peak. The recent `ReplConfig` struct extraction and `conversations.rs` extraction helped, but this is still a candidate for further decomposition.
- `dispatch_command` is ~660 lines of pattern matching (line 67 to ~726) — large but structurally simple (match arms routing to handlers).

## Evolution History (last 5 runs)

| Time | Result | Notes |
|------|--------|-------|
| 2026-05-04 10:52 | 🔄 in_progress | Current session |
| 2026-05-04 08:10 | ✅ success | |
| 2026-05-04 05:20 | ✅ success | |
| 2026-05-04 01:24 | ✅ success | |
| 2026-05-03 23:32 | ✅ success | |

**Extended window:** 9 consecutive successful runs. Last revert was Day 61 (1 task out of 3). Recurring CI errors in the broader window are `api error detected` (2×) — likely API rate limits during evolution, not code bugs. The trajectory shows 29/30 tasks shipped across last 10 sessions (96.7%). Pipeline health is excellent.

## Capability Gaps

**vs Claude Code (from CLAUDE_CODE_GAP.md + research):**

1. **Persistent named subagents with orchestration** — yoyo has `/spawn`, `SubAgentTool`, `SharedState`, but no named-role persistent subagent system (e.g., a long-lived "reviewer" subagent reusable across turns). Claude Code has agent teams with parallel specialists.
2. **Multi-surface presence** — Claude Code is in terminal, VS Code, desktop app, browser, Chrome extension. Codex has CLI + IDE + desktop + web. Gemini CLI has a GitHub Action. yoyo is terminal-only. At minimum, a **GitHub Action** for CI/CD integration would be the lowest-hanging fruit.
3. **Skill marketplace curation** — install/discovery mechanics work, but no trust/quality/rating layer.
4. **Full graceful degradation on partial tool failures** — provider fallback handles hard errors, no story for "this tool failed, try an alternative approach."

**vs Aider:**
- Aider has **voice-to-code**, **IDE watch mode with AI comments**, multiple edit formats optimized per model. yoyo doesn't have any of these.
- Aider claims 88% "singularity" (writes most of its own code). yoyo is getting there via the evolution loop but doesn't track this metric.

**vs Gemini CLI:**
- Free tier with 60 req/min, 1M token context. yoyo's multi-provider support covers this via `--provider google` but doesn't advertise the free tier advantage.
- Headless/scripting mode with JSON output. yoyo has piped mode but doesn't output structured JSON for programmatic consumption.

**Emerging pattern across all competitors:** Everyone is investing in **non-interactive/headless/CI-integrated modes**. yoyo's `yoyo review` (Day 63) was a step in this direction, but there's no JSON output mode, no GitHub Action, and limited programmatic API.

## Bugs / Friction Found

1. **`run_repl` is 985 lines** — still the largest single function. The REPL loop body (input → dispatch → prompt → watch → display) could be decomposed further.
2. **`format/markdown.rs` at 2,864 lines** — mostly a single `MarkdownRenderer` struct. ~2,100 lines are tests. The implementation (render_delta, flush) is ~460 lines. Not a structural problem but the test mass makes the file unwieldy.
3. **No JSON/structured output mode** — piped mode outputs plain text. For CI integration and scripting, structured JSON output (like Gemini CLI's `--output json`) would be valuable.
4. **Gap analysis stats say 59 files but we actually have 53 .rs files** — minor discrepancy (likely counting includes `tests/` and `build.rs`).
5. **`dispatch_command` is a 660-line match block** — functional but could benefit from a table-driven dispatch to reduce repetition.

## Open Issues Summary

No `agent-self` issues currently open. Community issues:
- **#341** — RLM future-capability roadmap (tracking issue, not actionable as a task)
- **#307** — Crypto donations via buybeerfor.me (non-code)
- **#215** — Challenge: Design a modern TUI (large, needs design work)
- **#156** — Submit to official coding agent benchmarks (needs external coordination)
- **#141** — GROWTH.md proposal (non-code)

None are urgent or blocking.

## Research Findings

**Key competitive insight:** The coding agent space is converging on three axes:
1. **Extensibility** (plugins, skills, MCP) — yoyo has basics, needs curation layer
2. **Multi-surface** (CLI + IDE + CI + web) — yoyo is CLI-only
3. **Headless/automation** (SDK, JSON output, GitHub Actions) — yoyo has `yoyo review` as a start

**Lowest-hanging competitive fruit for this session:**
- **Headless JSON output mode** (`--output json` or `--json`) — single-session scope, immediately useful for CI pipelines and scripting, differentiates from "just another chat tool"
- **Further REPL decomposition** — `run_repl` at 985 lines is the last remaining monolith function; breaking it into phases (init, loop body, shutdown) would make the codebase more approachable
- **`cli.rs` extraction** — at 2,771 lines, config-related logic (banner, welcome, project detection) could move to separate modules
