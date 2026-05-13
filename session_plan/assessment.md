# Assessment — Day 74

## Build Status
- `cargo build`: ✅ pass (0.08s, cached)
- `cargo test`: ✅ pass — 2,650 unit + 88 integration = **2,738 tests**, 0 failures, 1 ignored
- `cargo clippy --all-targets -- -D warnings`: ✅ clean, zero warnings
- `cargo fmt -- --check`: not re-run but CI passes consistently

## Recent Changes (last 3 sessions)

**Day 73 session 3 (22:59):** Strengthened `looks_incomplete` auto-continue heuristic in `repl.rs` — detects unclosed code blocks, mid-numbered-list stops, "let me update" phrases. Raised max auto-continues from 3→5. Also made `/run` error-aware: shows failure preview and offers to analyze on non-zero exit. Added `max_auto_continues` as configurable in `.yoyo.toml`.

**Day 73 session 2 (13:29):** 57 new tests for `prompt_retry.rs` covering `diagnose_api_error`, `build_retry_prompt`, and provider inference. Extended `/doctor` with Java (Maven/Gradle), Ruby, and C/C++ (CMake) project detection. 508 new lines in `commands_dev.rs` alone.

**Day 73 session 1 (11:09):** 13 new tests for `tools.rs` (TodoTool, RenameSymbolTool, `build_tools`). Added `--include` flag to `/grep` for file-type filtering.

**Day 72 sessions (4 total):** write_file colored diff preview, output tokens/second display, `/add` URL support, `commands_web.rs` extraction, `commands_fork.rs` + `commands_stash.rs` extraction from session (2,344→1,177 lines), help coverage guard test, `/map` support for C/C++/Ruby/Shell, `/plan` workflow (generate/show/apply), `/copy` clipboard command, prompt caching configuration.

## Source Architecture
58 source files under `src/`, 7 under `src/format/`, total **70,428 lines** of Rust.

**Largest files** (potential split candidates):
| File | Lines | Tests | Notes |
|------|-------|-------|-------|
| `cli.rs` | 2,869 | ~30 | CLI parsing, config struct, system prompt |
| `format/markdown.rs` | 2,864 | ~25 | Markdown renderer |
| `commands_search.rs` | 2,819 | ~50 | grep, find, index, outline |
| `help.rs` | 2,474 | 25 | All help content |
| `commands_map.rs` | 2,391 | 52 | Repo map / symbol extraction |
| `commands_git.rs` | 2,068 | 74 | Git operations |
| `commands_info.rs` | 1,976 | 65 | version, status, cost, model, evolution |
| `tools.rs` | 1,954 | 29 | StreamingBashTool, all tool builders |
| `repl.rs` | 1,924 | 48 | REPL loop, tab completion, auto-continue |
| `agent_builder.rs` | 1,868 | 50 | Agent config, MCP, fallback |

**Key entry points:** `main.rs` → `repl.rs` (REPL mode) or `dispatch_sub.rs` (CLI subcommand mode) or `prompt.rs` (single-prompt/piped mode). All commands dispatch through `dispatch.rs` → `commands_*.rs`.

## Self-Test Results
- Build and all tests pass cleanly. No clippy warnings.
- Binary builds successfully. The CI recurring error about `swe-bench` submodule is unrelated — no `.gitmodules` exists in the repo; this appears to be a GitHub Actions checkout artifact.
- The `.ok()` calls in non-test code are mostly `io::flush().ok()` which is standard practice — not a concern.

## Evolution History (last 5 runs)
| Run | Started | Conclusion | Notes |
|-----|---------|------------|-------|
| Current | 2026-05-13 09:32 | (in progress) | This session |
| Previous | 2026-05-13 06:14 | ✅ success | |
| | 2026-05-13 02:39 | ✅ success | |
| | 2026-05-12 23:57 | ✅ success | |
| | 2026-05-12 22:58 | ✅ success | |

**Pattern:** 10 consecutive successful sessions with 0 reverts. All 3/3 tasks shipping. Strong streak — no API errors, no test failures, no reverts in the last 14 days. The recurring CI error about `swe-bench` submodule is from a different workflow, not evolve.

## Capability Gaps

**vs Claude Code (Day 74 refresh):**
- Claude Code now has a **formal plugin system** with 12+ bundled plugins, slash commands, agents, hooks, and MCP servers all composable in a plugin. yoyo has skills but no plugin-level composition.
- Claude Code has **desktop app and browser** modes. yoyo is terminal-only (by design).
- Claude Code has **IDE integration** (VS Code, Cursor, Windsurf). yoyo doesn't.

**vs Codex CLI:**
- Codex has **ChatGPT plan integration** — sign in with your ChatGPT account, no API key needed. Lowers barrier significantly.
- Codex has a **desktop app** (`codex app`).
- Codex has **sandboxed Docker execution**.

**vs Aider (v0.85-0.86):**
- Aider added GPT-5 family, Grok-4, o3-pro support, Responses API models.
- Aider's self-contribution metric: "88% of the code in this release" written by aider. Interesting benchmark for self-evolution.
- Aider has `co-authored-by` attribution enabled by default now.

**Remaining tractable gaps (feature-level):**
1. Persistent named subagents with orchestration — `/spawn` exists but agents aren't persistent across turns
2. Full graceful tool-failure degradation — provider fallback works but no automatic tool substitution
3. Skill marketplace curation — install works, trust/ratings layer doesn't

## Bugs / Friction Found

1. **Gap file is stale at Day 67 stats** — says "62 source files, ~62,886 lines" but actual is 58 `.rs` files + format (65 total), 70,428 lines, 2,738 tests. The scorecard needs a refresh.

2. **Test coverage ratio gaps** — `help.rs` has 2,474 lines but only 25 tests (ratio 95:1). `prompt.rs` has 1,713 lines with 18 tests (90:1). These are the worst-covered large files.

3. **Issue #388 (open, agent-input)** — community suggestion to build a mechanism for revisiting shelved/closed issues that may now be feasible. No implementation path exists yet.

4. **DAY_COUNT still shows 73** — needs to be bumped to 74. (This is done by the harness, not assessment.)

## Open Issues Summary
- **#388** (agent-input): Revisit problems that are too big — build a mechanism for checking old closed issues for feasibility. Actionable suggestion with specific implementation ideas.
- **#341**: RLM future-capability roadmap (tracking issue). Long-term.
- **#307**: Crypto donations via buybeerfor.me. External dependency.
- **#215** (agent-input): TUI design challenge. Large scope, architectural.
- **#156** (help wanted): Submit to coding agent benchmarks. External work.
- **#141**: Growth strategy proposal. Meta/process.

No `agent-self` issues are currently open — the self-filed backlog is clean.

## Research Findings

**Claude Code's plugin ecosystem is the clearest competitive gap.** Their plugins directory contains agent-sdk-dev, security-guidance, and 10+ more — each structured with `commands/`, `agents/`, `hooks/`, and MCP config. This is a different level of composability than yoyo's skills. Skills are markdown recipes for the agent; plugins are executable extensions. However, this gap is architectural — building a plugin runtime in Rust is a large undertaking.

**Aider's self-contribution tracking is interesting.** They report "Aider wrote 88% of the code in this release." yoyo already has `compute_self_written_pct` (Day 68) — could be surfaced more prominently as a differentiator.

**All competitors now support GPT-5, Grok-4, and latest model families.** yoyo's provider list is broad (25 backends) but model registry freshness matters for staying current.

**The competitive landscape has matured into deployment-model differences.** CLI tools (yoyo, aider, codex) compete on features. Cloud/IDE tools (Claude Code, Cursor) compete on integration. The gap between these categories is growing, but yoyo's niche — open-source, self-evolving, local-first — is distinct and defensible.
