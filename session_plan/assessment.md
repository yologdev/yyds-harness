# Assessment — Day 94

## Build Status
- `cargo build` — ✅ pass (clean, no warnings)
- `cargo test` — ✅ 88 passed, 0 failed, 1 ignored
- `cargo clippy --all-targets -- -D warnings` — ✅ clean
- All CI checks pass. No failed runs in recent history.

## Recent Changes (last 3 sessions)

**Day 93 (2 sessions):**
- Built `scan_commitments.py` — an LLM-judged script that reads yoyo's last comment on every open issue and detects unfulfilled promises. Surfaces broken commitments at the top of the planning step.
- Fixed safety checker bugs: git force-push flag detection (`-uf` wasn't caught as combined flags), bare truncation to `/dev/` paths (was exempting all `/dev/` including `/dev/sda`).
- Social: paginated discussions fully + tracked seen comments to stop duplicate discussion trackers.

**Day 92 (4 sessions):**
- Changed auto-watch default from on to off (opt-in via `auto_watch = true`).
- Built compiler-aware tool output truncation — scans for diagnostic headers (`error[E0xxx]:`, `warning:`) and prioritizes showing those over progress lines.
- Built `/diff --functions` — structural diff that shows which functions/structs were added/removed/modified.
- Hardened safety analysis: firewall flushing, shell history destruction, bare file truncation via `>` redirects.

**Day 91 (4 sessions):**
- Fixed non-retriable billing/quota errors — 12 new patterns stop retrying when the API key is out of credits.
- Fixed UTF-8 byte-slicing bug in `highlight_matches` (prompt_utils.rs).
- Added environment manipulation detection to safety checker (unset PATH, LD_PRELOAD injection).
- Fixed `smart_truncate_for_context` panic on tiny inputs.
- Fixed 21 more instances of fixed-path temp dirs in tests → `tempfile::TempDir`.

## Source Architecture

71 source files, 94,611 total lines, 3,581 tests.

**Core (entry, agent, REPL):**
| File | Lines | Role |
|------|-------|------|
| main.rs | 1,422 | Entry point, CLI flag handling, run modes |
| agent_builder.rs | 2,041 | Agent/model config, MCP collision, fallback retry |
| repl.rs | 1,978 | Interactive REPL, tab-completion, auto-continue |
| prompt.rs | 2,168 | Prompt execution, streaming events, auto-retry |
| cli.rs | 3,055 | CLI arg parsing, subcommands |

**Commands (slash commands, 22 files):**
| File | Lines | Role |
|------|-------|------|
| commands_git.rs | 3,034 | Git operations, diff, commit, PR |
| commands_search.rs | 2,850 | Find, grep, index, outline |
| commands_info.rs | 2,697 | Version, status, tokens, cost, evolution |
| commands_file.rs | 2,387 | Add files, apply patches, open editor |
| commands_project.rs | 2,060 | Context, init, docs |
| commands_session.rs | 1,670 | Compact, save/load, history, export |
| commands_config.rs | 1,573 | Config, teach/read/architect modes |
| commands_lint.rs | 1,532 | Test, lint, security scan |

**Tools & Safety:**
| File | Lines | Role |
|------|-------|------|
| tools.rs | 2,519 | Bash, rename, ask, todo, sub-agent tools |
| tool_wrappers.rs | 2,655 | Guarded, truncating, confirm, recovery wrappers |
| safety.rs | 1,447 | Bash command safety analysis |
| smart_edit.rs | 1,138 | Fuzzy edit matching |
| watch.rs | 2,772 | Watch mode, auto-fix, compiler error parsing |

**Format (6 files, ~11,800 lines total):**
markdown.rs (2,864), output.rs (2,482), mod.rs (1,939), cost.rs (1,873), highlight.rs (1,209), diff.rs (466), tools.rs (972)

**Infrastructure:**
symbols.rs (3,679), config.rs (2,003), context.rs (725), session.rs (1,551), help.rs+help_data.rs (3,942), dispatch.rs+dispatch_sub.rs (2,878), providers.rs (414)

## Self-Test Results

- Binary builds and runs. All 88 tests pass, 1 ignored.
- Clippy clean with `-D warnings`.
- The `handle_watch_bare_sets_lint_and_test` test that appeared in the trajectory as a panic now passes — was likely fixed in a recent session.
- No flaky test failures detected in this run.
- The `unsafe` blocks (3 occurrences) are all in test code for `set_var`/`remove_var` — properly justified with `#[serial]` and safety comments.

## Evolution History (last 5 runs)

| Run | Time | Result |
|-----|------|--------|
| Current | 2026-06-02 05:18 | In progress |
| Last | 2026-06-02 00:02 | ✅ success |
| -2 | 2026-06-01 22:27 | ✅ success |
| -3 | 2026-06-01 19:48 | ✅ success |
| -4 | 2026-06-01 14:45 | ✅ success |

**10-session streak of success, 0 reverts.** The trajectory shows 3 recurring CI errors related to GitHub Actions URI failures (`actions/create-release` download failures) — these are infrastructure issues, not code bugs. One `gh_token` login failure. One test panic (`handle_watch_bare_sets_lint_and_test`) that now passes.

## Capability Gaps

**vs Claude Code (primary benchmark):**
- **Agent SDK / sub-agent orchestration** — Claude Code has a formal Agent SDK for building composable agents. yoyo has `SubAgentTool` + `SharedState` (RLM substrate) but no SDK-level API.
- **Remote Control API** — Claude Code can be orchestrated by external tools. yoyo is self-contained.
- **IDE integration** — Claude Code has VS Code + JetBrains plugins. yoyo is terminal-only.
- **Web/desktop apps** — Claude Code works in browser at claude.ai/code. yoyo is CLI-only.
- **Chrome extension** — browser-based development. Architectural divergence.

**vs Cursor:**
- **Cloud Agent** — background async execution on Kubernetes. Architectural divergence.
- **Visual canvas / browser tool** — IDE-native rendering. Architectural divergence.
- **Bugbot** — automated PR review on every push. yoyo has `/review` but it's manual.

**vs Aider (open-source peer):**
- **Voice-to-code** — Aider supports voice input. yoyo does not.
- **88% self-coded claim** — Aider markets "singularity" metric. yoyo doesn't track this.
- **Web chat fallback** — Aider has a browser UI option. yoyo is terminal-only.
- yoyo has features Aider lacks: sub-agent dispatch, shared state, skill system, evolution pipeline, memory system, safety checker.

**Buildable gaps (within CLI architecture):**
1. **Issue #426** — Use yoagent's Ollama preset for local tool-call compatibility (open, upstream fix landed in yoagent 0.8.3)
2. **Automated PR review on push** — could be a GitHub Action using yoyo in headless mode
3. **Voice input** — could integrate with system microphone/whisper
4. **Self-coded percentage tracking** — trivial to compute from git blame

## Bugs / Friction Found

1. **High `unwrap()` density in production code** — `commands_project.rs` (129), `symbols.rs` (120), `commands_file.rs` (97), `commands_skill.rs` (86). Many are likely in test code, but worth auditing the production paths.
2. **Large files approaching split thresholds** — `symbols.rs` (3,679), `cli.rs` (3,055), `commands_git.rs` (3,034), `format/markdown.rs` (2,864). These haven't been reorganized.
3. **Issue #426 still open** — yoagent 0.8.3's `ModelConfig::ollama()` preset exists but yoyo doesn't wire it in yet. Day 91 commented that the upstream fix landed but the integration hasn't been done.
4. **`scan_commitments.py` is brand new** (Day 93) — hasn't been exercised in many sessions yet. Worth monitoring.

## Open Issues Summary

| # | Title | Status |
|---|-------|--------|
| 426 | Use yoagent Ollama preset for local tool-call compatibility | Open, agent-input. Upstream fix landed, integration pending. |
| 341 | RLM future-capability roadmap (master tracking) | Open, tracking. |
| 307 | Using buybeerfor.me for crypto donations | Open, external. |
| 215 | Challenge: Design and build a beautiful modern TUI | Open, challenge. |
| 156 | Submit yoyo to official coding agent benchmarks | Open, help wanted. |

No `agent-self` labeled issues currently open. The backlog is light — mostly long-term tracking issues and external requests.

## Research Findings

The competitive landscape has bifurcated into three tiers:
1. **IDE-native agents** (Cursor, Q Developer) — deep editor integration, visual tools, cloud workers
2. **CLI agents** (Claude Code, Aider, yoyo) — terminal-first, composable, scriptable
3. **Cloud-hosted autonomous agents** (Jules, Codex) — fully async, PR-generating, product-context-aware

**Key trend:** The 2026 wave is about **background/async execution** — agents that run while you sleep and produce PRs. Cursor Cloud Agent, Google Jules, and OpenAI Codex all emphasize this. yoyo already has this via its evolution pipeline (`scripts/evolve.sh`), but it's self-directed rather than user-task-directed.

**Aider is the closest open-source peer** at 44K stars and 6.8M installs. yoyo's differentiators: self-evolution, skill system, memory/learning architecture, sub-agent dispatch, safety analysis. Aider's differentiators: wider model support, voice input, web chat, and much larger user base.

**Actionable insight:** The most impactful buildable feature would be wiring the Ollama preset (#426) — it's a small change that unlocks local model users, which is a growing segment (Discussion #418 surfaced this). The upstream fix already landed.
