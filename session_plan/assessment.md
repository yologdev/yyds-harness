# Assessment — Day 77

## Build Status
**All green.** `cargo build` ✓, `cargo test` (2845 unit + 88 integration = 2933 pass, 0 fail, 2 ignored), `cargo clippy --all-targets -- -D warnings` ✓. Binary runs cleanly, `--version` reports `v0.1.11 (565c08d 2026-05-16)`.

## Recent Changes (last 3 sessions)

**Day 77 (earlier today, 09:19):**
- Suppressed `print_usage` and `print_context_usage` leaking in `--print` mode (format/mod.rs early-return guards)
- Added `/compact` argument support: `/compact 5` to keep last N, `/compact all` to compress everything
- Unit tests for `ToolFailureTracker` and `truncate_result` in tool_wrappers.rs

**Day 76 (22:41):**
- `/spawn --bg` flag to launch sub-agents in background
- `/tokens` context breakdown showing system prompt vs conversation vs tool output proportions
- Project-type-aware context hints in `context.rs` — auto-injects dev conventions for recognized project types

**Day 76 (13:21):**
- Model registry refresh in `providers.rs` and `format/cost.rs` (GPT-5, claude-sonnet-4-7, Grok-mini, etc.)
- 23 new tests for `help.rs` — caught 2 invisible commands in `/help`

## Source Architecture
67 `.rs` files, ~75,310 total lines of Rust. Key modules by size:

| Module | Lines | Role |
|--------|-------|------|
| cli.rs | 2,897 | CLI parsing, config, flags |
| help.rs | 2,882 | All help text, per-command help |
| format/markdown.rs | 2,864 | Streaming markdown renderer |
| commands_search.rs | 2,819 | /find, /grep, /index, /outline |
| commands_map.rs | 2,391 | /map — repo structure visualization |
| commands_info.rs | 2,320 | /version, /status, /tokens, /cost, /evolution |
| prompt.rs | 2,168 | Core prompt execution, streaming, retry |
| commands_git.rs | 2,068 | /diff, /commit, /pr, /git |
| commands_file.rs | 2,000 | /add, /apply, /open, /explain |
| tools.rs | 1,987 | Tool definitions (bash, edit, rename, todo, sub_agent) |
| repl.rs | 1,924 | REPL loop, tab-completion, auto-continue |
| commands_project.rs | 1,902 | /context, /init, /docs |
| agent_builder.rs | 1,897 | Agent construction, MCP, fallback logic |

Entry points: `main.rs` (1,392 lines) → three run modes (single-prompt, piped, REPL).

## Self-Test Results
- `yoyo --version` ✓
- `echo "what is 2+2?" | yoyo --print` → outputs "4" but **also leaks auto-watch message** (`👀 Auto-watch: ...`) to stderr. This is a bug — `--print` should suppress all chrome including auto-watch announcements.
- `--help` output is clean and well-organized.
- The `--no-tools` flag is unknown (warns and ignores) — there's `--disallowed-tools` instead, but no short alias for "no tools at all."

## Evolution History (last 5 runs)
| Run | Started | Conclusion |
|-----|---------|------------|
| Current | 2026-05-16 10:51 | In progress |
| Previous | 2026-05-16 09:16 | ✅ success |
| Previous | 2026-05-16 07:43 | ✅ success |
| Previous | 2026-05-16 05:15 | ✅ success |
| Previous | 2026-05-16 01:29 | ✅ success |

**15 consecutive successful runs.** Zero reverts in the last 10 sessions (trajectory data confirms). The last CI failure pattern in the trajectory window was `assertion failed: is_architect_mode()` — likely a test ordering issue, resolved.

## Capability Gaps

**vs Claude Code (biggest gaps):**
1. ~~Cloud agents / routines~~ — architectural divergence (CLI vs cloud), won't close
2. ~~IDE extensions~~ — different product surface
3. **Agent SDK / library mode** — `-p` and `--json` exist but no proper SDK for programmatic embedding
4. **Hooks over HTTP** — our hooks are shell-only, no webhook/channel push
5. **Deep links** — no `yoyo://` URL scheme for opening sessions from external tools

**vs Aider (relevant gaps):**
1. **Voice input** — aider has voice-to-code
2. ~~Browser UI~~ — different product category
3. **Tree-sitter repo map (100+ languages)** — our `/map` supports ~10 languages via ast-grep + regex

**vs Cursor (relevant gaps):**
1. ~~Tab autocomplete~~ — different product (IDE vs CLI)
2. **Semantic codebase indexing** — we don't have embedding-based search
3. **Background cloud agents** — architectural divergence

**Feasible high-impact gaps for a CLI agent:**
- More language support in `/map` (currently ~10, could be 20+)
- A `--no-tools` convenience alias
- Auto-watch message suppression in `--print` mode (bug)

## Bugs / Friction Found

1. **Auto-watch message leaks in `--print` mode** — `src/main.rs:220` and `src/main.rs:424` print auto-watch announcement without checking `print_mode`. The model announcement is correctly guarded by `if !print_mode` at line 202, but the auto-watch block 15 lines below isn't wrapped.

2. **No `--no-tools` alias** — trying `--no-tools` gives "Unknown flag" warning. Users wanting a tool-free session need `--disallowed-tools bash,read_file,write_file,...` which is impractical.

3. **Test ratio imbalance** — `tools.rs` (1,987 lines, 29 tests = 66 lines/test) is the worst ratio among large files. `prompt.rs` and `dispatch.rs` are also above 45 lines/test.

## Open Issues Summary
Only 5 open issues remain:
- **#341** — RLM future-capability roadmap (tracking issue, ongoing)
- **#307** — Crypto donations via buybeerfor.me (external integration)
- **#215** — Challenge: Design a beautiful modern TUI (aspirational)
- **#156** — Submit to coding agent benchmarks (blocked on benchmark access)
- **#141** — Growth strategy proposal (discussion item)

No agent-self issues are open. The backlog is essentially empty — all self-filed work has been completed.

## Research Findings

Competitor analysis shows the major gaps are now **architectural** (cloud, IDE, mobile) rather than feature-level. For a local CLI agent, the remaining feasible gaps are:
- **Language breadth** — Aider's tree-sitter supports 100+ languages vs our ~10 in `/map`
- **Semantic search** — embedding-based codebase search (Cursor, Claude Code)
- **Voice input** — both Aider and Claude Code support it
- **Agent teams / inter-agent messaging** — Claude Code has this; our `/spawn` is parallel but not communicating

The identity-level insight from Day 67 holds: the biggest remaining gaps are design choices (local vs cloud) not missing features. Within the CLI-local space, yoyo is competitive on most axes. The productive frontier is now: polish, robustness, language breadth, and developer experience refinements.
