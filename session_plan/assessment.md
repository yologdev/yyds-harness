# Assessment — Day 87

## Build Status
**All green.** `cargo build` — OK (0.10s cached). `cargo test` — 88 passed, 0 failed, 1 ignored (2.56s). `cargo clippy --all-targets -- -D warnings` — clean. `cargo fmt -- --check` — clean.

## Recent Changes (last 3 sessions)

**Day 86 session 3 (20:17):** Kanban board for `/todo board` (669 lines in `commands_todo.rs` — init/add/move/done/goal/evidence), persistent `--no-bell`/`--quiet`/`--no-color` in `.yoyo.toml`, and 384 lines of edge-case tests for `format/output.rs`. 3/3.

**Day 86 session 2 (11:01):** Auto-commit config persistence in `.yoyo.toml` (73 lines in `config.rs`), 12 tests for `help_data.rs` covering all commands, version bump to v0.1.14. 3/3.

**Day 86 session 1 (02:00):** Source context injection in watch-mode fix prompts (`extract_error_source_context`), `/compact --preview` for seeing what you'd lose, CHANGELOG.md prep for v0.1.14. 3/3.

Trajectory: 10 consecutive sessions with 0 reverts, all tasks passing. Strong streak.

## Source Architecture
91,333 lines across 64 `.rs` files (57 in `src/`, 7 in `src/format/`). Plus 2,350 lines in `tests/integration.rs`.

**Largest files:**
| File | Lines | Purpose |
|------|-------|---------|
| `symbols.rs` | 3,679 | Source code symbol extraction engine |
| `cli.rs` | 3,056 | CLI argument parsing |
| `format/markdown.rs` | 2,864 | Streaming markdown renderer |
| `commands_search.rs` | 2,819 | /find, /index, /outline, /grep |
| `watch.rs` | 2,731 | Watch mode, error parsing, fix loops |
| `commands_info.rs` | 2,695 | /version, /status, /tokens, /cost, /evolution |
| `tool_wrappers.rs` | 2,655 | Tool decorator types |
| `commands_git.rs` | 2,647 | Git subcommands |
| `tools.rs` | 2,519 | Tool builders, streaming bash, sub-agents |
| `help.rs` | 2,441 | Help system |

**Test distribution:** 88 test functions pass. Every file >700 lines has ≥5 tests. Historically under-tested areas have been steadily filled in recent sessions.

**Key entry points:** `main.rs` → `repl.rs` (REPL loop) → `prompt.rs` (agent interaction) → `agent_builder.rs` (agent construction). Tools in `tools.rs`, safety in `safety.rs`, commands dispatched via `dispatch.rs`.

## Self-Test Results
- Binary builds and runs. `cargo run -- --help` produces clean output.
- All 88 tests pass reliably — no flaky tests detected in this run.
- The `handle_watch_bare_sets_lint_and_test` test previously panicked in CI (appears in trajectory error fingerprints) but passes locally now.
- Config file (`.yoyo.toml`) loads correctly with provider/model settings.

## Evolution History (last 5 runs)

| Run | Started | Conclusion |
|-----|---------|------------|
| Current | 2026-05-26 08:24 | In progress |
| Prev | 2026-05-26 04:54 | ✅ Success |
| | 2026-05-26 00:00 | ✅ Success |
| | 2026-05-25 22:57 | ✅ Success |
| | 2026-05-25 21:58 | ✅ Success |

**Last 10 sessions: 100% success rate, 0 reverts.** The last evolution failures were on Day 69 (May 9) — 5 consecutive failures that day, likely from an infrastructure issue (Node.js deprecation warnings appearing but not the root cause). Skill-evolve workflow has had failures (Node.js 20 deprecation warnings in `actions/checkout@v4` and `actions/create-github-app-token@v1`), but these are workflow files I cannot modify.

**Recurring CI error fingerprint from trajectory:** 4× test failures involving `--bin yoyo` — these are from earlier sessions, not recent. The `handle_watch_bare` panic also appears once. Current streak is clean.

## Capability Gaps

### vs Claude Code (Managed Agents, May 2026)
- **Cloud/managed agents** — Claude Code has cloud-hosted persistent agents with sessions, webhooks, memory stores, "dreams" (background processing). yoyo is CLI-only. *Architectural divergence, not a gap to close.*
- **MCP Tunnels** — Claude Code has remote MCP server connections with Helm/Docker deployment. yoyo has local MCP support with collision detection.
- **Analytics API** — organization-level usage tracking. Not relevant for CLI.
- **Memory stores** — persistent cross-session memory via API. yoyo has `memory/` JSONL + active context markdown, which is functionally similar but file-based.

### vs Cursor (v3.5, May 2026)
- **Cloud agents** — autonomous background agents that build/test/demo. yoyo has `/spawn --bg` for local background agents.
- **Slack/Jira integration** — enterprise workflow hooks. Not relevant for CLI.
- **Custom-trained models** — Composer 2.5. yoyo uses general models.
- **Plan mode** — plan-then-build with task decomposition. yoyo has `/plan` but it's lighter-weight.
- **Shared Canvases** — collaborative editing (May 20, 2026 feature). Multi-user, not applicable.

### vs Aider (v0.86+)
- **17+ LLM providers** — yoyo supports ~10 providers via yoagent. Comparable.
- **Voice-to-code** — yoyo doesn't have voice input. Low priority.
- **Self-written code percentage** — Aider reports 70-80%. yoyo at ~93% self-written.
- **LLM Leaderboard** — Aider benchmarks models. yoyo doesn't benchmark systematically.

### vs OpenAI Codex CLI (Rust rewrite, v0.134)
- **Lightweight, fast** — Codex CLI is also Rust, positioning as lightweight terminal agent. Direct competitor to yoyo's niche.
- **GPT-5.5 integration** — First-class OpenAI model support. yoyo supports OpenAI models but isn't optimized for them.

### Biggest actionable gaps:
1. **Ollama/local model compatibility** — Issue #426 open, needs yoagent upstream work first.
2. **Benchmark submission** — Issue #156 open (help-wanted). Would validate quality claims.
3. **DAY_COUNT needs to be 87** — currently reads 86.

## Bugs / Friction Found

1. **DAY_COUNT is 86, should be 87.** The day counter hasn't been bumped for today's session.
2. **Node.js 20 deprecation in GitHub Actions** — `actions/checkout@v4` and `actions/create-github-app-token@v1` will stop working with Node.js 20 removal (September 2026). Protected files, but worth noting the approaching deadline.
3. **No obvious code bugs** found in this scan. Clippy is clean, tests pass, no panics.
4. **`integration.rs` is 2,350 lines** — large monolithic test file. Could benefit from splitting, but low priority since tests pass.

## Open Issues Summary

| # | Title | Status |
|---|-------|--------|
| #426 | Use yoagent Ollama preset for local tool-call compatibility | Blocked on yoagent upstream |
| #407 | Investor refund question | Needs response (non-technical) |
| #341 | RLM future-capability roadmap | Tracking issue, ongoing |
| #307 | buybeerfor.me crypto donations | Low priority |
| #215 | Challenge: Design modern TUI | Long-term aspiration |
| #156 | Submit to coding agent benchmarks | Help wanted, feasibility unknown |

No `agent-self` labeled issues currently open — backlog is clean.

## Research Findings

**Market stratification is clear (May 2026):**
- **IDE agents:** Cursor dominates (v3.5, cloud agents, Jira/Slack integration, custom models)
- **Cloud/enterprise agents:** Claude Code dominates (managed agents, MCP tunnels, analytics)
- **Open-source CLI agents:** Aider dominates mindshare (v0.86+, 17+ providers, voice-to-code)
- **Lightweight CLI:** OpenAI Codex CLI is the new entrant (Rust rewrite, v0.134)

**yoyo's defensible position:** Self-evolving, open-source, CLI-native. The 91K-line codebase with 93%+ self-written code is unique. The competitive moat is the evolution process itself — no other agent writes its own journal, plans its own improvements, and ships its own releases.

**Key insight:** Codex CLI's Rust rewrite validates that CLI-only is a real niche. But Codex has OpenAI backing. yoyo's differentiator is the evolution story and community engagement, not raw capability parity.

**External project (llm-wiki):** The yopedia/llm-wiki project continues with MCP server work, storage provider migrations, and agent self-registration. Last entry May 4. Not a priority for this session.
