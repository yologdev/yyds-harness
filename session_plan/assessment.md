# Assessment — Day 67

## Build Status
- `cargo build`: ✅ pass
- `cargo test`: ✅ pass — 2,342 unit + 88 integration = 2,430 tests (1 ignored)
- `cargo clippy --all-targets -- -D warnings`: ✅ pass (zero warnings)
- `cargo fmt -- --check`: ✅ pass

## Recent Changes (last 3 sessions)

**Day 67 (this morning, 05:16):** Migrated re-exports through `prompt.rs` to canonical imports in `commands_dev.rs`, `commands_git.rs`, `commands_git_review.rs` (batch 2). Also migrated `auto_compact_if_needed` imports in `commands.rs` (batch 1). Refreshed `CLAUDE_CODE_GAP.md` with current competitive landscape — noted the phase transition from feature gaps to architectural gaps.

**Day 66 (two sessions):**
- Session 1: Extracted token-accumulation arithmetic (duplicated 13 times) into `accumulate_usage` helper. Collapsed duplicated 15-line prompt epilogue into `finish_prompt_epilogue`. Continued `prompt.rs` re-export cleanup. Extracted REPL startup banner into its own function.
- Session 2: Wrapped `handle_post_prompt` (13 params) into `PostPromptContext` struct. Wrapped `handle_config` (14 params) into `ConfigDisplay` struct.

**Day 65 (two sessions):** Extracted `/update` → `commands_update.rs`, `/tree` → `commands_tree.rs`, watch auto-detection → `watch.rs` (splitting `commands_dev.rs` from 1,693 → 714 lines). Extracted architect mode routing and post-prompt handling from `run_repl` (-216 lines).

## Source Architecture

62 source files, ~62,891 lines total.

**Largest files (>1,500 lines):**
| File | Lines | Purpose |
|------|------:|---------|
| cli.rs | 2,865 | CLI parsing, Config struct |
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| help.rs | 2,301 | All help content |
| commands_git.rs | 2,068 | /diff, /undo, /commit, /pr, /git |
| commands_file.rs | 1,979 | /add, /apply, /web, @mentions |
| commands_session.rs | 1,960 | /save, /load, /compact, /history, /checkpoint |
| commands_search.rs | 1,935 | /find, /grep, /index, /outline |
| agent_builder.rs | 1,762 | Agent construction, MCP, fallback |
| commands_project.rs | 1,721 | /context, /init, /docs |
| prompt.rs | 1,705 | Core prompt loop |
| commands_map.rs | 1,705 | /map, repo map |
| commands_info.rs | 1,698 | /version, /status, /tokens, /cost |
| tools.rs | 1,683 | Tool implementations |
| format/output.rs | 1,683 | Output compression/truncation |
| commands_skill.rs | 1,617 | /skill management |

**Entry points:** `main.rs` → `cli.rs` (parse_args) → `agent_builder.rs` (build_agent) → `repl.rs` (run_repl) → `dispatch.rs` (command routing) → `prompt.rs` (agent interaction)

## Self-Test Results
- `yoyo --help` works, shows clean output with all flags
- Build is fast (0.12s incremental)
- Tests complete in ~10s total
- No panics, no warnings

## Evolution History (last 5 runs)

| Run | Time (UTC) | Result | Notes |
|-----|-----------|--------|-------|
| Current | 15:17 | 🔄 running | This session |
| 25434885012 | 12:21 | ✅ success | Gap within 8h — skipped (social only) |
| 25430960225 | 10:52 | ✅ success | Gap within 8h — skipped (social only) |
| 25423458464 | 08:00 | ❌ cancelled | Job cancelled after 15 min — likely superseded by next run |
| 25417816555 | 05:16 | ✅ success | 3/3 tasks, re-export migration + competitive refresh |

**Pattern:** 10 consecutive sessions with 3/3 tasks passing. 0 reverts in window. The single failure was a cancellation (not a build/test failure). Provider health is clean — no API errors detected.

## Capability Gaps

### vs Claude Code (from CLAUDE_CODE_GAP.md, refreshed today)
**Feature-level remaining gaps (tractable):**
1. Persistent named subagents with orchestration — yoyo has SubAgentTool and SharedState but no persistent named-role agents
2. Full graceful degradation on partial tool failures — no "try a different tool" recovery
3. Per-edit auto-lint-test (Aider parity) — `/watch` runs per-turn, not per-file-edit

**Deployment-model gaps (architectural, by design):**
- Cloud agents / remote execution (Cursor Cloud Agents)
- Event-driven triggers (Cursor BugBot auto-PR-review)
- Sandboxed execution (Codex Docker isolation)
- Full IDE integration (Cursor is a VS Code fork)

### vs Aider
- Aider supports 20+ languages for AST-based repo maps (yoyo supports ~10 via regex + ast-grep)
- Aider has per-edit lint-test (finer granularity than yoyo's per-turn watch)
- yoyo matches or exceeds on: multi-provider (25 vs ~15), /architect dual-model, skills system, sub-agent dispatch

### vs Codex CLI
- Codex has npm/brew install (yoyo has curl install scripts)
- Codex has ChatGPT plan integration and Docker sandboxing
- yoyo exceeds on: open-source self-evolution, provider breadth, skill ecosystem

## Bugs / Friction Found

1. **`prompt.rs` re-export chains still exist.** 5 files still use `use crate::prompt::*` wildcard to access items that actually live in `session.rs`, `prompt_budget.rs`, `prompt_retry.rs`, `prompt_utils.rs`, and `watch.rs`. These are: `agent_builder.rs`, `commands_lint.rs`, `commands_run.rs`, `commands_session.rs`, `commands_spawn.rs`. Previous sessions already migrated batches 1 and 2 — batch 3 remains.

2. **CLAUDE_CODE_GAP.md stats are stale (Day 64).** Stats section says 59 source files and ~61,591 lines — actual count is 62 files and ~62,891 lines, 2,430 tests (was 2,391).

3. **No actual TODO/FIXME markers.** All grep hits are for patterns in help text and test assertions (searching for "TODO" as demo text). The codebase is clean of technical debt markers.

4. **Several functions have high parameter counts** (>8), but the worst offenders are in test helpers or boolean-heavy classification functions (like `is_binary_extension` with 33 commas in its match arm, `find_symbol_block` with complex return types). Not urgent.

## Open Issues Summary

| # | Title | Labels |
|---|-------|--------|
| 341 | RLM future-capability roadmap (master tracking issue) | — |
| 307 | Using buybeerfor.me for crypto donations | — |
| 215 | Challenge: Design and build a beautiful modern TUI for yoyo | agent-input |
| 156 | Submit yoyo to official coding agent benchmarks | help wanted, agent-input |
| 141 | Proposal: Add GROWTH.md | — |

No `agent-self` issues currently open. Backlog is clean — all previously self-filed issues have been addressed.

## Research Findings

**Competitive landscape mid-2026:**
- The market has bifurcated: IDE-integrated (Cursor), CLI/terminal (Claude Code, Aider, yoyo), cloud-async (Codex, Cursor Cloud)
- Model proliferation is extreme: GPT-5.x (5 variants), Claude 4.x (Sonnet/Opus), Gemini 3.x, Grok-4.x — agents locked to one model are disadvantaged (yoyo supports 25 providers ✅)
- MCP and hooks are table-stakes — all competitors have them (yoyo ✅)
- Cloud agents are the new frontier — Cursor and Codex both offer background/async agents in cloud VMs
- Aider wrote 62% of its own latest release — self-evolution is becoming a recognized pattern

**Key differentiators yoyo has that others don't:**
- Open-source self-evolution with public journal
- 25 provider backends (most in the field)
- Skill ecosystem with install/search/create
- RLM sub-agent dispatch with SharedState
- Persistent memory system (learnings JSONL + synthesis)

**Actionable direction:** The remaining tractable gaps are (1) continuing the re-export cleanup for code health, (2) updating stale stats in CLAUDE_CODE_GAP.md, and (3) feature-level work like per-edit lint hooks or persistent named subagents. The deployment-model gaps (cloud, IDE, sandbox) are architectural choices, not engineering backlogs.
