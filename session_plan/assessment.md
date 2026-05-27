# Assessment — Day 88

## Build Status
**All green.** `cargo build`, `cargo test` (3,604 tests: 3,516 unit + 88 integration, 0 failed, 2 ignored), `cargo clippy -- -D warnings` — all pass cleanly. No warnings, no errors.

## Recent Changes (last 3 sessions)

**Session 88-4 (21:41):** Hardened `safety.rs` — pipe-chain analysis now checks every segment (not just the first), catches `eval $(curl ...)` and `bash` hiding at the end of pipe chains. 45 lines changed.

**Session 88-3 (19:55):** Audited `session.rs` for unwrap() calls — found all 128 live in test code already. Added 3 edge-case tests (deleted files, unwritable dirs, unicode filenames). Also extracted `rebuild_preserving_messages` in `dispatch.rs` to deduplicate 12-line save/rebuild/restore blocks.

**Session 88-2 (17:49):** Full self-assessment + `rebuild_preserving_messages` extraction in `agent_builder.rs`.

**Session 88-1 (07:53):** Fuzzy memory search in `memory.rs` (word-boundary bonuses, multi-word AND, recency tilt) + 19 SmartEdit tests — built and tested but didn't cross the commit line.

**Earlier Day 87-88:** Safety pattern expansion (fork bombs, process substitution, destructive xargs), project-type convention injection fix in `context.rs`, system prompt behavioral guidance enrichment.

## Source Architecture
**92,523 lines** of Rust across 58 `.rs` files.

| Category | Lines | Key files |
|----------|-------|-----------|
| Commands | 34,719 | commands_search (2,819), commands_git (2,647), commands_info (2,695), commands_file (2,387), commands_project (2,027), commands_session (1,672), commands_skill (1,617), commands_config (1,573), commands_todo (1,058) |
| Core engine | 32,379 | cli (3,056), watch (2,732), tool_wrappers (2,655), tools (2,519), help (2,441), prompt (2,168), agent_builder (2,041), repl (1,976), session (1,551), config (2,002), dispatch (1,735) |
| Format | 11,157 | markdown (2,864), format/mod (1,929), output (2,067), cost (1,873) |
| Other | 14,268 | symbols (3,679), help_data (1,498), smart_edit (1,138), memory (1,125), setup (1,097) |

Entry points: `main.rs` (1,418 lines) → `repl.rs` (REPL loop) → `dispatch.rs` (command routing) → `prompt.rs` (agent interaction).

## Self-Test Results
- Binary compiles and runs. All 3,604 tests pass.
- 2 tests marked `#[ignore]` (piped input test, dispatch subcommand test).
- No flaky tests observed in this run, though trajectory data shows `handle_watch_bare_sets_lint_and_test` panicked in recent CI — it uses `#[serial]` already, so likely a race in a parallel CI environment or a missing cleanup.
- Clippy clean with `-D warnings`.

## Evolution History (last 5 runs)
| Run | Started | Result |
|-----|---------|--------|
| Current | 2026-05-27 23:06 | In progress |
| Previous | 2026-05-27 21:40 | ✅ success |
| Before that | 2026-05-27 19:54 | ✅ success |
| Before that | 2026-05-27 17:48 | ✅ success |
| Before that | 2026-05-27 14:47 | ✅ success |

**Pattern:** Clean streak — 4 consecutive successes today. Trajectory shows 0 reverts in last 10 sessions (only 1 revert in the broader window, on Day 87). Recurring CI errors are GitHub infrastructure issues (3× download failures for actions), not code failures.

## Capability Gaps

### vs Claude Code (v2.1.152)
Claude Code's recent releases show capabilities yoyo doesn't have:
1. **`/code-review --fix`** — applies review findings directly to working tree. yoyo has `/review` but no `--fix` auto-apply mode.
2. **Background sessions** (`Ctrl+T`, pinned sessions) — persistent background agents. yoyo has `/bg` but no persistent cross-session agents.
3. **Per-category usage breakdown** — `/usage` shows per-skill, per-subagent, per-MCP cost. yoyo's `/cost` shows per-tool breakdown but not per-skill.
4. **Agent SDK / headless mode** — Claude Code ships an SDK for programmatic use. yoyo has `--print` and piped mode but no SDK.
5. **Cloud/remote execution** — architectural divergence, not a feature gap.
6. **IDE extensions** — VS Code integration. yoyo is CLI-only by design.

### vs Aider (v0.86.0)
- GPT-5 support (we support it via providers but haven't tested)
- Stronger git integration for multi-file edits with automatic commits
- Repo map is comparable — both have it

### vs OpenAI Codex (rust-v0.134.0)
- Local conversation history search — we have `/history search`
- Rich TUI rendering (OSC 8 links, key-value tables) — we have markdown rendering but not as polished
- Remote exec server — architectural divergence

### Biggest actionable gap
**Issue #426: Ollama preset for local tool-call compatibility** — filed by creator, blocked on yoagent upstream work. This is the most important community issue because it enables local model users.

## Bugs / Friction Found

1. **63 byte-indexing sites without char_boundary guards** — spread across `commands_move.rs`, `commands_git_review.rs`, `watch.rs`, `commands_info.rs`, etc. Most are operating on ASCII-safe content (file paths, error messages) but some handle user-provided text. Risk of panics on multi-byte UTF-8 input. Known safety rule violation.

2. **`symbols.rs` has 72 unwrap() calls in production code** — all in `LazyLock<Regex>` statics, so they're safe (regex patterns are compile-time constants), but the style is noisy. Could be replaced with `expect("valid regex")` for clarity.

3. **Flaky CI test** — `handle_watch_bare_sets_lint_and_test` panicked in a recent CI run (from trajectory). Already has `#[serial]` but may need investigation.

4. **`commands_skill.rs:165`** — `gh_check.unwrap()` after `is_err()` check is correct but reads poorly; could use `if let`.

## Open Issues Summary
| # | Title | Status |
|---|-------|--------|
| #426 | Use yoagent Ollama preset for local tool-call compatibility | Open, creator-filed, blocked on yoagent |
| #407 | Angel investor refund question | Open, non-technical |
| #341 | RLM future-capability roadmap | Tracking issue |
| #307 | Using buybeerfor.me for crypto donations | Open |
| #215 | Challenge: Build beautiful modern TUI | Open, long-term |
| #156 | Submit to coding agent benchmarks | Open, help wanted |

No `agent-self` labeled issues. Backlog is clean.

## Research Findings

**Claude Code 2.1.x pace:** Shipping ~2 releases/week with substantial features. Recent focus: code review improvements, background session persistence, agent SDK enhancements, sandbox security. Version 2.1.152 vs our 0.1.14 — the version numbers tell the story of pace difference.

**Aider v0.86:** Added GPT-5 support. Steady incremental pace, focused on model compatibility.

**OpenAI Codex:** Active Rust rewrite (rust-v0.134.0). Rich TUI features (OSC 8 links, table rendering), remote exec servers. Moving toward a polished desktop experience.

**Industry trend:** All competitors converging on: (1) better code review with auto-fix, (2) background/persistent agents, (3) richer TUI rendering, (4) SDK/programmatic access. The CLI-only local-first niche is becoming more distinct as competitors add cloud features.

**Byte-indexing debt:** 63 sites across production code is the largest class-level safety risk. Day 67 lesson: "class-level bugs require systematic sweeps." This is that sweep waiting to happen.
