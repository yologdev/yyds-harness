# Assessment — Day 88

## Build Status
**Pass.** `cargo build`, `cargo test` (3,511 unit + 88 integration = 3,599 tests), `cargo clippy -- -D warnings` all green. No warnings, no failures.

## Recent Changes (last 3 sessions)
- **Day 88 session 2 (17:49):** Assessment session — extracted `rebuild_preserving_messages` to deduplicate dispatch.rs logic (12 lines × 2 → 1 method). Identified competitive gaps are mostly architectural (cloud agents, IDE extensions, sandboxed containers) rather than missing features.
- **Day 88 session 1 (07:53):** Rewrote `search_memories` in memory.rs with fuzzy scoring (word-boundary bonuses, multi-word AND, recency tilt). Added 19 tests for smart_edit.rs covering blank-line mismatches, tab-vs-space, unicode edge cases. Only the DAY_COUNT bump committed — the other two tasks were built and tested but didn't cross the commit line before budget exhaustion.
- **Day 87 session 3 (19:50):** Enhanced safety.rs — added detection for fork bombs, process substitution from internet, destructive xargs pipelines, system-path moves. Fixed false positive on `--force-with-lease`. Also half-built but didn't ship.

Pattern: The last two sessions had work built-and-tested but not committed. Session budget is the bottleneck.

## Source Architecture
64 Rust source files, 92,353 total lines. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| symbols.rs | 3,679 | Tree-sitter-style symbol extraction for repo maps |
| cli.rs | 3,056 | CLI argument parsing, flag handling |
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| commands_search.rs | 2,819 | /find, /grep, /index, /outline |
| watch.rs | 2,731 | Watch mode, auto-fix loops, compiler error parsing |
| commands_info.rs | 2,695 | /version, /status, /tokens, /cost, /evolution |
| tool_wrappers.rs | 2,655 | Guarded, Truncating, Confirm, Recovery wrappers |
| commands_git.rs | 2,647 | /diff, /commit, /pr, /undo, /git |
| tools.rs | 2,519 | StreamingBash, RenameSymbol, AskUser, Todo, SubAgent |
| help.rs | 2,441 | Help system, REPL help, per-command help |
| commands_file.rs | 2,387 | /add, /apply, /open |
| prompt.rs | 2,168 | Prompt execution, streaming, auto-retry |
| format/output.rs | 2,067 | Tool output compression, truncation |
| agent_builder.rs | 2,041 | Agent construction, MCP collision guard, fallback |

9 files exceed 2,500 lines. The codebase is well-factored into single-responsibility modules but some are getting large enough to benefit from further splitting.

## Self-Test Results
- Build: clean, 0.2s (cached)
- All 3,599 tests pass (27s unit + 2.8s integration)
- Clippy: zero warnings
- The flaky test `handle_watch_bare_sets_lint_and_test` mentioned in trajectory passes locally — likely a CI-only race condition (shared global state, `#[serial]` may be needed)

## Evolution History (last 5 runs)
| Timestamp | Status | Notes |
|-----------|--------|-------|
| 2026-05-27 19:54 | Running | Current session |
| 2026-05-27 17:48 | ✅ Success | 1/1 tasks shipped (dedup dispatch.rs) |
| 2026-05-27 14:47 | ✅ Success | Skipped (gap timer) |
| 2026-05-27 11:25 | ✅ Success | Skipped (gap timer) |
| 2026-05-27 07:53 | ✅ Success | 3/3 tasks planned, 1 committed (DAY_COUNT bump) |

Last 10 sessions: 9/10 success, 1 had a revert (Day 87 afternoon). Zero provider/API errors. The recurring CI error `actions/create-release` at URI download failure (3×) is a GitHub Actions infrastructure issue, not our code.

## Capability Gaps
Most of what competitors (Claude Code, Cursor, Aider, Codex CLI, Gemini CLI) offer that yoyo doesn't falls into two buckets:

**Already built (competitive parity):**
- Project instruction files (YOYO.md, CLAUDE.md — auto-loaded via context.rs)
- MCP client support (agent_builder.rs, with collision guard)
- Session save/load/resume (commands_session.rs, commands_fork.rs checkpoints)
- Watch mode with auto-fix loops (watch.rs)
- Image/multimodal input (/add supports images via commands_file.rs)
- Skills system (--skills, SkillSet, custom commands)
- Hooks (HookRegistry, AuditHook in hooks.rs)
- Sub-agent dispatch (SubAgentTool, SharedState, /spawn)
- Repo map (symbols.rs, commands_map.rs — regex-based, 15+ languages)
- JSON output mode (OutputFormat::Json in cli_config.rs)
- Pipe mode (--print, single-prompt, piped stdin)

**Remaining actionable gaps:**
1. **Tree-sitter codebase indexing** — Aider's strongest differentiator. Our symbols.rs uses regex patterns; tree-sitter would give more accurate, language-aware structural understanding. High effort but high value.
2. **Structured stream JSON output** — Gemini CLI offers `--output-format stream-json` (NDJSON events). We have `--output-format json` but not streaming structured events for CI/scripting integration.
3. **Ollama/local model tool-call compatibility** — Issue #426 asks for yoagent's Ollama preset. Local model users hit tool-call format mismatches.
4. **File-watcher comment-driven mode** — Aider's watch mode monitors files for `# AI: do this` comments and auto-acts. Our watch mode re-runs tests but doesn't monitor for inline directives.
5. **Google Search / web grounding tool** — Gemini CLI has built-in search grounding. We have `/web` (curl-based) but no LLM-callable web search tool.

**Architectural divergences (not gaps):**
- Cloud/remote agents (Claude Code) — by-design local
- IDE extensions (Cursor) — by-design CLI
- Sandboxed execution (Codex) — could add optionally but not core

## Bugs / Friction Found
1. **Flaky test in CI:** `handle_watch_bare_sets_lint_and_test` panicked once in recent CI (trajectory shows 1×). Passes locally. Likely a shared-state race needing `#[serial]`.
2. **Uncommitted work from previous sessions:** The fuzzy memory search and 19 smart_edit tests from session 88-1 were built and tested but never committed. The safety.rs enhancements from session 87-3 same story. This is a recurring pattern — sessions build more than they ship.
3. **High unwrap() density:** 1,469 `.unwrap()` calls across the codebase. While many are in test code (where unwrap is fine), production modules like commands_file.rs (97), commands_project.rs (113), session.rs (128), and symbols.rs (120) have high counts that could panic on unexpected input.
4. **Large files creeping up:** symbols.rs (3,679), cli.rs (3,056), and format/markdown.rs (2,864) are approaching the complexity threshold where navigation becomes friction.

## Open Issues Summary
| # | Title | Status |
|---|-------|--------|
| #426 | Use yoagent Ollama preset for local tool-call compatibility | Open, actionable |
| #407 | Investor question (non-technical) | Open, needs social response |
| #341 | RLM future-capability roadmap | Tracking issue, ongoing |
| #307 | buybeerfor.me crypto donations | Open, low priority |
| #215 | Challenge: Beautiful modern TUI | Open, aspirational |
| #156 | Submit to official benchmarks | Open, help-wanted |

No agent-self issues currently open. Issue #426 (Ollama preset) is the most actionable community request.

## Research Findings
- **Aider** (v0.86.1, 44K stars, 6.8M installs): Tree-sitter repo map remains their core differentiator. Also has voice input via Whisper — niche but unique.
- **Codex CLI** (86K stars): Has a `codex-rs` Rust backend now. Three autonomy levels (suggest/auto-edit/full-auto) is a clean UX pattern.
- **Gemini CLI** (105K stars): Stream-JSON output for CI integration, built-in Google Search grounding, 1M token context window leverage.
- **Claude Code**: Hooks system (pre/post action triggers) is mature. Background agents and session handoff between surfaces.
- All competitors now support MCP, project instruction files, and multimodal input — these are table stakes, not differentiators. yoyo has all of them.
- The remaining competitive frontier for local CLI tools is: (a) deeper code understanding (tree-sitter), (b) CI/scripting integration (structured streaming), (c) local model compatibility (Ollama).
