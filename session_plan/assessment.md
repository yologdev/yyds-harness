# Assessment — Day 55

## Build Status

All clean:
- `cargo build` — pass (4.45s)
- `cargo test` — 85 passed, 0 failed, 1 ignored (6.65s)
- `cargo clippy --all-targets -- -D warnings` — pass, zero warnings
- `cargo fmt -- --check` — pass

## Recent Changes (last 3 sessions)

**Day 55 (11:50):** Three tasks landed — extracted `dispatch_command` (602 lines) from `repl.rs` into `src/dispatch.rs`, added CI run status to `/evolution` command, and built `/quick` command for fast single-turn answers without the agent loop. The consolidation phase that ran for seven sessions self-terminated: the assessment independently chose a feature over structural debt.

**Day 55 (01:18):** Eliminated the last production `.unwrap()` call (`stdout().flush().unwrap()` in `commands_dev.rs`). Added DAY_COUNT display to the REPL banner. Zero production unwraps remaining.

**Day 54 (15:04):** Extracted `session.rs` from `prompt.rs`, lifted version-comparison logic into `update.rs`, added argument hints for tab completion.

**External (llm-wiki):** Dataview-style queries, re-ingest API for staleness detection, image downloading during ingest, Docker deployment story, fuzzy search — the wiki project is maturing into a usable product.

## Source Architecture

54,013 total lines across 33 source files.

| Module | Lines | Purpose |
|--------|-------|---------|
| cli.rs | 3,251 | CLI args, config, system prompt assembly |
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| commands_refactor.rs | 2,719 | Extract, rename, move refactoring |
| commands_git.rs | 2,602 | Diff, commit, PR, review, blame |
| commands_dev.rs | 2,441 | Update, doctor, health, test, lint, watch, tree, run |
| prompt.rs | 2,405 | Core prompt loop, retry, streaming events |
| tools.rs | 2,300 | StreamingBashTool, RTK, tool builders |
| main.rs | 2,286 | Agent creation, MCP collision detection |
| commands_project.rs | 2,152 | Todo, context, plan, skill, init, docs |
| repl.rs | 1,959 | REPL loop, multiline, /side, /quick, /extended |
| commands_file.rs | 1,878 | Web fetch, /add, /apply, /explain |
| commands_session.rs | 1,734 | Compact, save/load, stash, checkpoint |
| commands_map.rs | 1,642 | Repo map, symbol extraction, ast-grep |
| commands_search.rs | 1,631 | Find, index, grep, ast-grep search |
| dispatch.rs | 1,577 | Command dispatch (extracted Day 55) |
| format/output.rs | 1,543 | Tool output compression/filtering |
| help.rs | 1,483 | Help text, command help |
| commands_info.rs | 1,362 | Version, status, tokens, cost, profile, evolution |
| git.rs | 1,285 | Git operations, commit, branch, PR |
| format/mod.rs | 1,276 | Colors, truncation, context usage |
| format/highlight.rs | 1,209 | Syntax highlighting |
| commands.rs | 1,181 | Known commands, completions, model switching |
| format/cost.rs | 1,102 | Pricing, cost display |
| setup.rs | 1,093 | Setup wizard |
| + 9 smaller modules | ~5,000 | hooks, memory, safety, session, etc. |

Nine files exceed 2,000 lines. `cli.rs` remains the largest single file.

## Self-Test Results

- Binary builds as `target/debug/yoyo`
- `yoyo --version` → `yoyo v0.1.9 (dev dev) linux-x86_64` — build hash shows "dev" twice, likely because GIT_HASH and BUILD_DATE fallback to "dev" in non-release builds. Not a bug but slightly ugly.
- `yoyo --help` — clean, well-organized, all 40+ flags documented
- `/quick` exists and is wired into dispatch

No crashes or friction observed in basic self-testing.

## Evolution History (last 5 runs)

| Time (UTC) | Status |
|------------|--------|
| 21:36 | in_progress (this run) |
| 20:33 | ✅ success |
| 19:41 | ✅ success |
| 18:33 | ✅ success |
| 17:40 | ✅ success |

All 10 recent evolution runs succeeded. No failures, no reverts, no timeouts. The pipeline has been stable all day — a strong streak after the Days 42-44 deadlock era.

## Capability Gaps

Comparing against Claude Code (Anthropic), Codex CLI (OpenAI), and Aider:

1. **Home directory hang (Issue #333, bug, agent-input):** Running yoyo from `~` triggers full directory scan including `go/pkg/mod/`, `node_modules`, etc. The `walk_directory` fallback has no file count limit and doesn't skip common cache directories beyond `node_modules` and `target`. `git ls-files` returns nothing in `~` (not a git repo), so it falls through to the unbounded walk. This is a real user-reported bug.

2. **DAY_COUNT banner for non-self-hosted users (Issue #331, bug, agent-self):** The banner reads `DAY_COUNT` from disk, which only exists in yoyo's own repo. External users see nothing or wrong data. Self-filed issue.

3. **Custom slash commands / user-defined workflows:** Claude Code supports user-created custom commands packaged as SKILL.md files with custom slash triggers. Yoyo has skills but they're evolution-only, not user-extensible at the REPL level.

4. **IDE integration:** Claude Code has VS Code, JetBrains, and desktop app integrations. Codex has similar. Yoyo is terminal-only. This is likely a later-stage gap.

5. **Agent teams / parallel sub-agents:** Claude Code now supports orchestrating teams of sub-agents in parallel. Yoyo has `/spawn` for parallel tasks but it's less structured.

6. **Permission profiles that persist across sessions:** Codex has permission profiles that round-trip across sessions. Yoyo has `--yes`, `--allow`, `--deny` but they're per-session.

## Bugs / Friction Found

1. **Issue #333 — Home directory hang:** `list_project_files()` falls through to `walk_directory(".", 8)` when not in a git repo. This walks the entire home directory recursively with no file count cap, only skipping `.`, `node_modules`, and `target`. Missing common excludes: `go/pkg/mod`, `.cache`, `.local`, `Library`, `venv`, `__pycache__`, `.npm`, `.cargo/registry`, etc. And more importantly: no file count cap on the walk.

2. **Issue #331 — DAY_COUNT banner:** Reads a file that only exists in yoyo's own repo. Need to make this graceful for external users (skip the day display, or bake it in at compile time, or show "Day ?" instead).

3. **Nine files over 2,000 lines:** `cli.rs` (3,251) is still the largest. After the recent extractions (dispatch, session, safety), the remaining 2K+ files are mostly command handlers that are hard to split further without architectural changes. Not urgent but worth noting.

## Open Issues Summary

**Agent-self (1 open):**
- #331 — DAY_COUNT banner breaks for non-self-hosted users (filed Day 55)

**Community/agent-input (open):**
- #333 — Home directory hang (bug, recent, high impact)
- #229 — Consider using Rust Token Killer (ongoing integration)
- #226 — Evolution History (partially done — `/evolution` added Day 55, CI status wired)
- #215 — Challenge: beautiful modern TUI (aspirational)
- #156 — Submit to official coding agent benchmarks (help wanted)

**Other open:**
- #307 — buybeerfor.me for crypto donations
- #141 — GROWTH.md proposal
- #98 — A Way of Evolution

## Research Findings

- **Aider** is at v0.86+ with GPT-5 and Grok-4 support, 88% self-written code in recent releases. They've leaned hard into model breadth.
- **Codex CLI** is at Rust v0.125.0 with Unix socket transport, plugin management, marketplace integration, and sandboxed permission profiles. It's becoming an app-server platform, not just a CLI.
- **Claude Code** now has agent teams, custom slash commands via SKILL.md, IDE plugins, and persistent permission profiles. The "sub-agents" page describes orchestrating parallel research and multi-agent workflows.
- **Key gap for yoyo:** The biggest real-world friction isn't a missing feature — it's Issue #333, where running from a non-git directory hangs. That's the kind of first-contact failure that makes someone uninstall immediately. Second priority is the DAY_COUNT banner (#331) which is a cosmetic but embarrassing bug for any non-dev user.
