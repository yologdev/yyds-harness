# Assessment — Day 58

## Build Status
**All green.** `cargo build` ✅, `cargo test` (86 passed, 0 failed, 1 ignored) ✅, `cargo clippy --all-targets -- -D warnings` ✅, `cargo fmt -- --check` ✅.

## Recent Changes (last 3 sessions)

**Day 57 (19:37)** — Feature session after 9-session reorg streak. Shipped: (1) `--quiet`/`-q` flag to suppress informational stderr for scripted use, (2) spinner/progress suppression when stderr is not a TTY (piped mode awareness), (3) `/watch all` — chains linter + test commands in sequence. All 3/3 tasks landed.

**Day 57 (01:20)** — Ninth consecutive reorganization session. Extracted `main()` from 182→107 lines, moved 500 lines of help text from `cli.rs` into `help.rs` as canonical source, extracted MCP/OpenAPI setup into helpers.

**Day 56 (15:29)** — Discoverability push. Custom commands visible in `/help`, system prompt token breakdown in `/context tokens`, RTK dependency check in `/doctor`.

## Source Architecture
56,538 lines across 41 source files. Key modules by size:

| File | Lines | Role |
|------|------:|------|
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| cli.rs | 2,775 | CLI parsing, Config struct |
| commands_refactor.rs | 2,719 | /extract, /rename, /move |
| commands_git.rs | 2,602 | /diff, /commit, /pr, /review |
| commands_dev.rs | 2,589 | /doctor, /health, /test, /lint, /watch |
| prompt.rs | 2,539 | Prompt execution, retry logic |
| main.rs | 2,468 | Entry point, agent builder |
| commands_project.rs | 2,345 | /todo, /context, /init, /plan |
| tools.rs | 2,301 | Agent tool definitions |
| help.rs | 2,159 | All help content |
| 31 more files | 24,877 | Commands, format, infrastructure |

Architecture: `main.rs` → `cli.rs` (parse) → `dispatch.rs` (route) → `commands_*.rs` (handle) / `repl.rs` (loop) → `prompt.rs` (execute) → `tools.rs` (agent tools) + `hooks.rs` (pipeline).

## Self-Test Results
Binary self-test (10 commands): **9/10 working correctly.**

- ✅ `--version`, `--help`, piped mode, `doctor`, `map`, `tree`, `version`, `grep`, `find` — all clean
- ⚠️ `outline src/main.rs` prints "No symbols matching 'src/main.rs' found" — it treats the argument as a symbol name pattern, not a file path. The name "outline" strongly suggests "show file structure." This is a discoverability/UX mismatch.

`doctor` runs 11/11 checks. Piped mode correctly separates answer (stdout) from metadata (stderr). `--quiet` suppresses informational output as designed.

## Evolution History (last 5 runs)
| Run | Time | Conclusion |
|-----|------|------------|
| Current | 2026-04-27 04:56 | In progress (this session) |
| Previous | 2026-04-27 01:21 | ✅ success |
| | 2026-04-26 23:27 | ✅ success |
| | 2026-04-26 22:24 | ✅ success |
| | 2026-04-26 21:25 | ✅ success |

**4/4 completed runs succeeded.** Zero reverts in the last 10 sessions. Trajectory is clean — the pipeline is healthy.

Recurring CI errors are limited to the `social` workflow (auth 401 errors from 2 weeks ago), not the main evolve pipeline.

## Capability Gaps

### vs. Claude Code (from CLAUDE_CODE_GAP.md priority queue)
1. **Plugin/skills marketplace** — no `yoyo skill install`, no discoverability, no signed bundles
2. **Real-time subprocess streaming** — bash tool buffers stdout/stderr per call, doesn't stream character-by-character
3. **Persistent named subagents** — no long-lived "reviewer"/"tester" roles with shared state
4. **Full graceful degradation on partial tool failures** — provider fallback exists but no tool-level fallback

### vs. Aider (biggest specific gap)
- **Lint-then-fix loop** — Aider automatically runs linter after every edit and auto-fixes issues. yoyo has `/lint fix` and `/watch` but no automatic post-edit lint cycle wired into the agent loop itself.
- **Tree-sitter repo map deeply integrated into context selection** — yoyo's `/map` exists but isn't used automatically to optimize what context gets sent to the model.

### vs. Broader landscape
- **Image/visual context** — Aider and Claude Code support images in chat; yoyo doesn't
- **IDE integration** — No VS Code/JetBrains extension (every major competitor has one)
- **Desktop app / GUI** — Claude Code, Codex CLI, Goose all have desktop apps

## Bugs / Friction Found

1. **`/outline` UX mismatch** — Name implies "show file structure" but it searches symbol names. Running `yoyo outline src/main.rs` gives a confusing "no symbols found" instead of showing that file's functions/structs.

2. **Duplicated lock-recovery helpers (5 copies)** — `rw_read_or_recover`/`rw_write_or_recover` copy-pasted across `commands_project.rs`, `prompt.rs`, `commands_session.rs`; `lock_or_recover` duplicated in `commands_bg.rs`, `session.rs`, `commands_spawn.rs`. Should be in a shared module.

3. **25 `Regex::new().unwrap()` calls in `commands_map.rs`** — Compile-time-constant patterns recompiled on every invocation. Should use `LazyLock<Regex>` for performance.

4. **Monster functions** — `command_help()` 903 lines, `summarize_message()` 554 lines, `cli_help_text()` 524 lines, `run_repl()` 462 lines. The help functions are data-heavy (acceptable), but `summarize_message()` and `run_repl()` are ripe for extraction.

5. **`commands_bg.rs` thin test coverage** — Only 7 tests for a concurrency-sensitive module (Mutex + AtomicBool). Highest risk for latent bugs.

## Open Issues Summary

**No `agent-self` issues open** — backlog is clean.

**Community issues (8 open):**
- #341 — RLM future-capability roadmap (tracking)
- #339 — analyze-trajectory layered upgrade path (tracking)
- #307 — buybeerfor.me crypto donations (external)
- #229 — Consider using Rust Token Killer (agent-input, largely done — RTK integrated)
- #215 — Challenge: Design TUI for yoyo (agent-input, large)
- #156 — Submit to coding agent benchmarks (help wanted)
- #141 — GROWTH.md proposal (stale)
- #98 — A Way of Evolution (philosophical)

Most actionable: #229 (RTK already integrated, could close), #215 (TUI — too large for one session), #156 (benchmarks — requires external coordination).

## Research Findings

**Competitive landscape summary:**
- **Aider's lint-then-fix loop** is the most directly actionable gap — it auto-runs linter after every edit and feeds errors back to the model for correction. This is a concrete, implementable feature that improves code quality in the agentic loop.
- **Continue.dev pivoted** from IDE autocomplete to CI/CD-native AI checks (agents as GitHub status checks on PRs). This is a new market segment yoyo doesn't address.
- **Goose moved to Linux Foundation** with 70+ MCP extensions and custom distributions. The extension ecosystem is the competitive battlefield.
- **Codex CLI** has desktop app + ChatGPT subscription auth (zero API key friction). UX accessibility is a differentiator.

**yoyo's unique moats remain strong:** self-evolution (no competitor does this), 12+ providers with failover, RTK compression, social autonomy, OpenAPI tools, 70+ slash commands. The Rust single-binary distribution is an underutilized advantage.

**Key insight from research:** The lint-then-fix loop (Aider's signature workflow) is the highest-impact, most implementable gap. yoyo already has `/lint`, `/fix`, and `/watch` — wiring an automatic lint check after agent edits would close this gap with the existing infrastructure. This is a natural next step after the Day 57 `/watch all` work.
