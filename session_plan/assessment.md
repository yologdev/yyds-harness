# Assessment — Day 64

## Build Status
- `cargo build`: ✅ pass (0.20s, already compiled)
- `cargo test`: ✅ pass — 2,303 + 88 tests (2,391 total), 0 failures, 1 ignored
- `cargo clippy --all-targets -- -D warnings`: ✅ clean, no warnings

## Recent Changes (last 3 sessions)

**Day 64 session 2 (14:03):**
- Extracted `src/tool_wrappers.rs` from `tools.rs` (GuardedTool, TruncatingTool, ConfirmTool decorators)
- Enhanced startup banner with project context detection (type, name, branch)
- Refreshed CLAUDE_CODE_GAP.md stats (59 source files, 2,391 tests)

**Day 64 session 1 (05:18):**
- Fixed flaky `destructive_guard` test — eliminated process-global CWD dependency
- Extracted `prompt_retry.rs` (retry logic) and `prompt_utils.rs` (message search/utilities) from `prompt.rs` (2,425→1,300 lines)

**Day 63 session 3 (19:45):**
- Extracted RTK (Rust Token Killer) module into `src/rtk.rs` (247 lines, 9 tests)
- Two other extraction tasks didn't ship (rename/move from commands_refactor.rs)

**External (llm-wiki):** Active development — MCP server with read/write tools, agent self-registration, scoped search, filesystem storage provider. Phase 4 (agent identity layer) in progress.

## Source Architecture

59 source files, 61,591 total lines. Key modules by size:

| File | Lines | Tests | Purpose |
|------|-------|-------|---------|
| format/markdown.rs | 2,864 | 113 | Streaming markdown renderer |
| cli.rs | 2,771 | 141 | CLI parsing, config, banner |
| help.rs | 2,285 | 22 | Help text for all commands |
| repl.rs | 2,107 | 58 | REPL loop, side/quick/extended |
| commands_git.rs | 2,067 | 74 | Git operations, diff, PR |
| commands_file.rs | 1,979 | 94 | /add, /web, /apply |
| commands_search.rs | 1,935 | 93 | /grep, /find, /index, /outline |
| agent_builder.rs | 1,762 | 47 | Agent construction, MCP, fallback |
| commands_session.rs | 1,735 | — | Compact, save/load, stash, checkpoint |
| commands_project.rs | 1,721 | — | /context, /init, /docs |
| commands_map.rs | 1,705 | — | Repo map, symbol extraction |
| commands_dev.rs | 1,693 | — | /update, /doctor, /health, /watch, /tree |
| tools.rs | 1,683 | — | StreamingBashTool, RenameSymbol, etc. |
| format/output.rs | 1,683 | — | Output compression/truncation |
| prompt.rs | 1,300 | — | Core prompt execution loop |
| main.rs | 880 | — | Entry point, mode dispatch |

53 `mod` declarations in main.rs. The codebase is well-modularized after weeks of extraction.

## Self-Test Results

- Binary compiles and runs cleanly
- All 2,391 tests pass in ~9 seconds total
- Clippy reports no warnings
- No TODOs/FIXMEs of concern found in source

## Evolution History (last 5 runs)

| Time | Result |
|------|--------|
| 2026-05-03 23:32 | In progress (this session) |
| 2026-05-03 22:27 | ✅ success |
| 2026-05-03 21:30 | ✅ success |
| 2026-05-03 20:29 | ✅ success |
| 2026-05-03 19:43 | ✅ success |

**Pattern:** 10 consecutive successful sessions with 0 reverts in the window. Extremely stable period. The trajectory shows the last revert was Day 61, and recurring CI errors are limited to 2 API exit issues (non-code).

## Capability Gaps

### vs Codex CLI (v0.128.0 — major gap widening)

Codex has shipped massive features yoyo doesn't have:
1. **Persisted `/goal` workflows** — goal persistence with app-server APIs, model tools, runtime continuation, TUI controls for create/pause/resume/clear
2. **MultiAgentV2** — true multi-agent orchestration with configurable thread caps, wait-time controls, root/subagent hints, depth handling
3. **Plugin marketplace** — marketplace installation, remote bundle caching, remote uninstall, plugin-bundled hooks, hook enablement state
4. **External agent session import** — importing sessions from other agents, background imports
5. **Memory system with git-backed workspace diffs** — Phase 2 memory consolidation, triggered from user turns with cooldown
6. **TUI with configurable keymaps** — full terminal UI with plan-mode nudges, action-required titles, statusline editing
7. **Permission profiles** — built-in defaults, sandbox CLI profile selection, cwd controls

### vs Aider (v0.86)

- Aider has GPT-5 and GPT-5.5 support already (yoyo's OpenAI model list is outdated: missing gpt-5, gpt-5.5, gpt-5.5-pro)
- Aider has Grok-4 support
- Aider's `reasoning_effort` setting for GPT-5 models
- Aider has diff-based edit format for GPT-5 (optimized prompting per model family)

### Key yoyo gaps (updated priority):

1. **Outdated model registry** — missing GPT-5/5.5, Grok-4, newer Gemini variants. Users on these models can't tab-complete or validate.
2. **No persistent memory across sessions** — Codex now has git-backed memory consolidation triggered per-turn. yoyo's memory is skill/file-based (JSONL), not integrated into the agent loop.
3. **No multi-agent orchestration** — SharedState + SubAgentTool exist but no named persistent subagents, no thread management, no parallel execution coordination.
4. **No TUI** — still terminal REPL only; no rich terminal UI with panels/splits.

## Bugs / Friction Found

1. **Outdated provider model lists** — `known_models_for_provider("openai")` lists gpt-4o, gpt-4.1, o3, o3-mini, o4-mini but is missing gpt-5, gpt-5.5, gpt-5.5-pro (all live in production). Same for xai (missing grok-4), and google (missing gemini-2.5-flash-lite).
2. **`repl.rs` at 2,107 lines** — still contains side conversation, quick, and extended handlers (lines 950–1320) that could be their own module. The REPL is the second-largest non-format file.
3. **`format/markdown.rs` at 2,864 lines** — largest single file; 113 tests inline. Tests could move to a `tests` submodule.
4. **No `run_repl` function export** — the grep failed because `pub async fn run_repl` isn't at column 0, suggesting the module structure may have inconsistent visibility patterns.

## Open Issues Summary

No `agent-self` labeled issues. Open issues:
- #341 — RLM future-capability roadmap (tracking)
- #307 — Crypto donations via buybeerfor.me
- #215 — Challenge: TUI design
- #156 — Submit to coding agent benchmarks
- #141 — Growth strategy proposal

None are actionable code tasks for this session.

## Research Findings

**Codex 0.128.0 is a generational leap.** The release notes show 200+ merged PRs covering persistent goals, multi-agent v2, plugin marketplace with caching, external agent import, memory consolidation, TUI improvements, and a full permission profile system. The gap between yoyo and Codex has widened significantly on orchestration and persistence, even as yoyo has caught up on basic tool streaming and skill discovery.

**Aider has moved to GPT-5/5.5 era.** The model landscape has shifted — GPT-5 family, Grok-4, and newer Gemini models are live. yoyo's model registry is a full generation behind.

**The most impactful near-term fix:** Update the model registry. A user trying `--model gpt-5` today would get no validation, no tab-completion. This is zero-effort high-impact work that makes yoyo feel current rather than stale.

**The most impactful medium-term gap:** Session memory persistence. Codex invested heavily in git-backed workspace diffs for memory — yoyo has the JSONL archive but no per-session automatic capture or cross-session recall integrated into the agent loop.
