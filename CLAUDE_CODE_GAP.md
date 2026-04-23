# Gap Analysis: yoyo vs Claude Code

Last verified: Day 54 (2026-04-23)
Last updated: Day 24 (2026-03-24) — major refresh on Day 38, stats refresh on Day 50, Day 54

This document tracks the feature gap between yoyo and Claude Code, used to inform
development priorities when there are no community issues to address. It is a
**snapshot**, not a TODO list — the priority queue at the bottom names the real
remaining gaps, but task selection still happens through the normal planning loop.

## Legend
- ✅ **Implemented** — yoyo has this
- 🟡 **Partial** — yoyo has a basic version, Claude Code's is better
- ❌ **Missing** — yoyo doesn't have this yet

---

## Core Agent Loop

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Streaming text output | ✅ | ✅ | True token-by-token streaming — mid-line tokens render immediately, line-start briefly buffers for fence/header detection (Day 17, fixed line-buffering bug); streaming flush improvements (Day 23) |
| Tool execution | ✅ | ✅ | bash (with per-command timeout), read_file, write_file, edit_file, search, list_files, rename_symbol, ask_user, todo |
| Multi-turn conversation | ✅ | ✅ | Both maintain conversation history |
| Thinking/reasoning display | ✅ | ✅ | yoyo shows thinking dimmed; --thinking flag controls budget |
| Error recovery / auto-retry | ✅ | ✅ | yoagent retries 3x with exponential backoff by default |
| Subagent / task spawning | 🟡 | ✅ | `/spawn` runs tasks in separate context; yoagent's `SubAgentTool` exposes subagents as tools; no named-role persistent orchestration yet |
| Tool output streaming | 🟡 | ✅ | `ToolExecutionUpdate` events handled and rendered live (line counts, partial tail); full real-time subprocess streaming inside a single tool call still buffered |
| Background processes | ✅ | ✅ | `/bg` command (Day 45): launch, list, view output, kill background jobs with persistent tracker; Claude Code has similar with `/bashes` |

## CLI & UX

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Interactive REPL | ✅ | ✅ | |
| Piped/stdin mode | ✅ | ✅ | Improved piped mode handling (Day 23) |
| Single-shot prompt (-p) | ✅ | ✅ | |
| Output to file (-o) | ✅ | ✅ | |
| Model selection | ✅ | ✅ | --model flag and /model command |
| Session save/load | ✅ | ✅ | /save, /load, --continue, /history |
| Git integration | ✅ | ✅ | Branch in prompt, /diff, /undo, /commit (with co-authored-by trailer), /pr; git-aware system prompt gives agent branch/dirty state automatically |
| Readline / line editing | ✅ | ✅ | rustyline: arrow keys, history (~/.local/share/yoyo/history), Ctrl-A/E/K/W |
| Tab completion | ✅ | ✅ | Slash commands, file paths, and argument-aware completion (--model values, git subcommands, /pr subcommands) (Day 14) |
| Fuzzy file search | ✅ | ✅ | `/find` with scoring, git-aware file listing, top-10 ranked results (Day 12) |
| Syntax highlighting | ✅ | ✅ | Language-aware ANSI highlighting for Rust, Python, JS/TS, Go, Shell, C/C++, JSON, YAML, TOML |
| Markdown rendering | ✅ | ✅ | Incremental ANSI: headers, bold, code blocks, inline code, syntax-highlighted code blocks |
| Progress indicators | ✅ | ✅ | Braille spinner animation during AI responses (Day 8); per-tool live progress timer |
| Multi-line input | ✅ | ✅ | Backslash continuation and code fences |
| Image input support | ✅ | ✅ | `/add` reads images as base64; `--image` flag for CLI; auto-detects png/jpg/gif/webp/bmp (v0.1.1) |
| Custom system prompts | ✅ | ✅ | --system, --system-file, plus config file `system_prompt`/`system_file` keys (Day 23) |
| Extended thinking control | ✅ | ✅ | --thinking flag |
| Color control | ✅ | ✅ | --no-color, NO_COLOR env |
| Edit diff display | ✅ | ✅ | Colored inline diffs for `edit_file` tool output — red/green removed/added lines (Day 14) |
| Inline @file mentions | ✅ | ✅ | `@path` in prompts expands to file contents; supports line ranges `@file:10-20` and images (Day 21) |
| Conversation bookmarks | ✅ | ❌ | `/mark`, `/jump`, `/marks` — name points in conversation and jump back (Day 14) |
| First-run onboarding | ✅ | ✅ | Detects first run, shows welcome message, guides API key and model configuration (Day 22) |
| Terminal bell notifications | ✅ | ✅ | Bell on long completions; --no-bell flag and YOYO_NO_BELL env to disable (Day 23) |
| Conversation stash | ✅ | ❌ | `/stash` saves/restores conversation context without files (Day 22) |
| File patch application | ✅ | ❌ | `/apply` applies unified diff patches to files (Day 23) |
| AST structural search | ✅ | ❌ | `/ast` searches code by structure using tree-sitter patterns (Day 23) |
| Auto-test watcher | ✅ | ❌ | `/watch` auto-runs tests on file changes (Day 23) |
| Refactoring umbrella | ✅ | ❌ | `/refactor` with subcommands: rename, extract, move (Day 23) |

## Context Management

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Proactive context compaction | ✅ | ✅ | Proactive at 70% + auto-compact at 80% context (Day 23, upgraded from auto-only) |
| Manual compaction | ✅ | ✅ | /compact command |
| Token usage display | ✅ | ✅ | /tokens with visual bar; live context-window percentage in prompt |
| Cost estimation | ✅ | ✅ | Per-request and session totals |
| Context window awareness | ✅ | ✅ | Per-model context limit tracked (no longer hardcoded to 200k — #195 fix) |

## Permission System

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Tool approval prompts | ✅ | ✅ | `--yes`/`-y` to auto-approve; interactive confirm for bash, write_file, and edit_file; "always" persists per-session (Day 15) |
| Allowlist/blocklist | ✅ | ✅ | `--allow`/`--deny` flags with glob matching; `[permissions]` config section; deny overrides allow (`PermissionConfig` in `src/config.rs`) |
| Directory restrictions | ✅ | ✅ | `--allow-dir`/`--deny-dir` flags + `[directories]` config; canonicalized path checks prevent traversal; sub-agents inherit restrictions (Day 35) (`DirectoryRestrictions` in `src/config.rs`) |
| Auto-approve patterns | ✅ | ✅ | `--allow` glob patterns + config file `allow` array; "always" option during confirm |
| User-configurable hooks | ✅ | ✅ | `[[hooks]]` config blocks for shell hooks on tool calls; `Hook` trait + `HookRegistry` in `src/hooks.rs` (Issue #21, Day 34) |

## Project Understanding

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Project context files | ✅ | ✅ | yoyo reads YOYO.md, CLAUDE.md, and .yoyo/instructions.md (`src/context.rs`) |
| Auto-detect project type | ✅ | ✅ | `detect_project_type` used by `/test`, `/lint`, `/health`, `/fix` (Rust, Node, Python, Go, Make) |
| Project scaffolding | ✅ | ✅ | `/init` scans project and generates a YOYO.md context file (Day 13) |
| Git-aware file selection | ✅ | ✅ | `get_recently_changed_files` appended to project context (Day 12) |
| Git-aware system prompt | ✅ | ✅ | Agent always sees current branch and dirty state in system prompt (Day 23) |
| Codebase indexing | ✅ | ✅ | `/index` builds lightweight project index: file count, language breakdown, key files (Day 14) |
| Repo map for prompt context | ✅ | ✅ | `/map` builds tree-sitter or ast-grep symbol map for the agent |

## Developer Workflow

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Run tests | ✅ | ✅ | `/test` auto-detects project type and runs tests (Day 12) |
| Auto-fix lint errors | ✅ | ✅ | `/lint` auto-detects and runs linter; `/fix` sends failures to AI (Day 9+12) |
| PR description generation | ✅ | ✅ | `/pr create [--draft]` generates AI-powered PR descriptions |
| Commit message generation | ✅ | ✅ | `/commit` with heuristic-based message generation from staged diff (Day 8) |
| Code review | ✅ | ✅ | `/review` provides AI-powered code review of staged/unstaged changes (Day 13) |
| Multi-file refactoring | ✅ | ✅ | `/refactor` umbrella command (rename, extract, move); `rename_symbol` agent tool for cross-project renames (Day 23) |

## Configuration

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| Config file | ✅ | ✅ | yoyo reads .yoyo.toml and ~/.config/yoyo/config.toml |
| Per-project settings | ✅ | ✅ | .yoyo.toml in project directory |
| MCP server support | ✅ | ✅ | `--mcp` flag + `[[mcp.servers]]` config blocks; `McpServerConfig` + `parse_mcp_servers_from_config` in `src/config.rs`; stdio transport, used in production |
| Multi-provider support | ✅ | ❌ | yoyo supports 12 providers via `--provider` (anthropic, openai, google, ollama, bedrock, z.ai, cerebras, etc.) — `KNOWN_PROVIDERS` in `src/providers.rs` |
| Skills system | ✅ | 🟡 | yoyo loads skills via `--skills <dir>` (yoagent's `SkillSet`); Claude Code has formal skill packs and a plugin marketplace (see gap below) |
| OpenAPI tool support | ✅ | ❌ | `--openapi <spec>` loads OpenAPI specs and registers API tools (Day 9) |
| Config system_prompt/system_file | ✅ | ✅ | `system_prompt` and `system_file` keys in .yoyo.toml for persistent custom prompts (Day 23) |
| Plugin / skills marketplace | ❌ | ✅ | Claude Code has a plugin marketplace and bundled skill packs; yoyo has the loader (`--skills`) but no discoverability, no signed bundles, no install command |

## Error Handling

| Feature | yoyo | Claude Code | Notes |
|---------|------|-------------|-------|
| API error display | ✅ | ✅ | Shows error messages |
| Network retry | ✅ | ✅ | yoagent handles 3 retries with exponential backoff by default |
| Rate limit handling | ✅ | ✅ | yoagent respects retry-after headers on 429s |
| Context overflow recovery | ✅ | ✅ | Auto-compacts conversation and retries on context overflow errors (Day 20) |
| Provider fallback | ✅ | ❌ | `--fallback` chains providers; auto-switches on hard errors (#205, Day 31) |
| Graceful degradation | 🟡 | ✅ | Retry logic, error handling, context overflow recovery, provider fallback; not yet full fallback on partial tool failures |
| Ctrl+C handling | ✅ | ✅ | Both handle interrupts |

---

## Priority Queue (real remaining gaps)

After the Day 38 refresh, the gaps that are actually still gaps. Re-evaluated
on Day 54 — these four remain the real delta, though the competitive landscape
has shifted (see below).

1. **Plugin / skills marketplace** (since Day ≤38) — Claude Code has formal skill packs and a
   plugin marketplace with discoverability and install commands. yoyo has
   `--skills <dir>` (yoagent's `SkillSet`) but no marketplace, no signed
   bundles, and no `yoyo skill install` flow. Claude Code's API now also
   exposes advisor, memory, and web tools as first-class capabilities, widening
   the plugin surface area.
2. **Real-time subprocess streaming inside tool calls** (since Day ≤38) — Claude Code shows
   compile/test output as it streams from the child process. yoyo's
   `ToolExecutionUpdate` events render line counts and partial tails, and
   Day 51 improved live output for long-running bash commands. But the
   underlying bash tool still buffers stdout/stderr per call rather than
   pumping it to the renderer character-by-character. Per-command timeout
   helps with runaway processes but doesn't change the streaming model.
3. **Persistent named subagents with orchestration** (since Day ≤38) — yoyo has `/spawn` and
   yoagent's `SubAgentTool`, but no named-role persistent subagent system
   (e.g., a long-lived "reviewer" or "tester" subagent the orchestrator can
   delegate to repeatedly with shared state).
4. **Full graceful degradation on partial tool failures** (since Day ≤38) — provider fallback
   covers hard API errors, but there's no story for "this tool call failed,
   try a different tool that achieves the same effect."

### Competitive landscape shift (Day 54)

The gap is no longer just yoyo vs Claude Code. The field has widened:

- **Claude Code API** now exposes web search, web fetch, code execution,
  advisor, and memory tools as first-class API capabilities — things that
  were previously CLI-only are now programmable.
- **Codex CLI** (OpenAI) has npm/brew install, ChatGPT plan integration,
  and a desktop app — lowering the barrier to entry for non-terminal users.
- **Aider** has expanded tree-sitter language support and continues to
  iterate on its edit format and model compatibility.

yoyo's differentiators remain: open-source self-evolution, multi-provider
support (14 backends), and the skills/hooks extensibility model. The
marketplace gap (#1 above) is increasingly important as competitors
formalize their extension stories.

### What was on the old priority queue and is now done

These were listed as gaps on Day 24 but have shipped since:

- ✅ **MCP server support** — `--mcp` flag, `[[mcp.servers]]` config blocks,
  `McpServerConfig` and `parse_mcp_servers_from_config` in `src/config.rs`,
  used in production for weeks.
- ✅ **User-configurable hooks** — `[[hooks]]` config blocks, `Hook` trait and
  `HookRegistry` in `src/hooks.rs`, closing Issue #21 (Day 34).
- ✅ **Sub-agent tool** — `build_sub_agent_tool` in `src/tools.rs` exposes
  yoagent's `SubAgentTool` to the model.
- ✅ **Per-model context window** — Issue #195 fix removed the hardcoded
  200k limit; `effective_context_tokens` in `src/cli.rs` reads per-model
  defaults.
- ✅ **Provider fallback** — `--fallback` chains providers and auto-switches
  on hard errors (Issue #205, Day 31, `try_switch_to_fallback` in `src/main.rs`).
- ✅ **Bedrock provider wiring** — both the wizard and the actual provider
  construction landed (Day 30 trap closed).
- ✅ **Background process management** — `/bg` command in `src/commands_bg.rs`
  (Day 45): launch, list, view output, kill background jobs. Persistent
  `BackgroundJobTracker` with async completion detection.
- ✅ Recently completed (Day 23–37): `/refactor` umbrella + `rename_symbol`,
  `/watch` auto-test watcher, `/ast` structural search, `/apply` patch
  application, `/stash` conversation stash, terminal bell notifications,
  config `system_prompt`/`system_file` keys, git-aware system prompt,
  proactive context compaction (70% + 80%), streaming flush improvements,
  piped mode improvements, sub-agent directory restriction inheritance,
  audit-log wiring, autocompact thrash detection, live context-window
  percentage, byte-indexing safety pass on tool output pipeline (#250).
- ✅ Recently completed (Day 38–44): per-command bash timeout (`"timeout": N`
  parameter, 1–600s, Day 44), co-authored-by trailer on `/commit` (Day 43),
  `/status` shows session elapsed time and turn count (Day 43), `/changelog`
  command for recent git evolution history (Day 44), CWD race condition fix
  in repo map tests (Day 44), multi-provider fork guide (Day 43).
- ✅ Recently completed (Day 45–46): `/bg` background process management
  (Day 45), multi-provider fork guide (Day 45), destructive-git-command
  guard in `run_git()` (Day 45), streaming output for `/run` and `/watch`
  (Day 45), `/lint fix`, `/lint pedantic`, `/lint strict`, `/lint unsafe`
  (Day 46).
- ✅ Recently completed (Day 47–49): piped mode graceful slash-command
  handling (Day 47), `/blame` with colorized output (Day 48), proper
  unified diffs (LCS-based) for edit_file operations (Day 48), dead code
  cleanup (Day 48), 23 shell subcommands wired for direct CLI invocation
  (Days 48–49), comprehensive categorized help with 68+ commands (Day 49).
- ✅ Recently completed (Day 50–51): context budget warnings at 60/80/90/95%
  (Day 50), `/status` enriched with token counts (Day 50), `/explain`
  file explanation command (Day 50), fuzzy command suggestions via
  Levenshtein distance (Day 50), tool output compression for noisy build
  logs (Day 50), v0.1.8 release (Day 50), integration test speedup —
  removed 2.5 min of unnecessary network waits (Day 51), live output
  improvements for long-running bash commands (Day 51), `/profile`
  session statistics command (Day 51), CWD race fix in repo map tests
  (Day 51).
- ✅ Recently completed (Day 52–53): poison-proof mutex/rwlock handling
  across all production code (Days 52), v0.1.9 release prep (Day 52),
  safety sweep — `.unwrap()` hardening in non-test code including
  `commands_refactor.rs` UTF-8 safety (Day 53), `--stat` flag for `/diff`
  with compact diffstat view (Day 53), exit summary enriched with tokens,
  cost, and duration (Day 53), format module extraction —
  `format/output.rs` (1,543 lines) and `format/diff.rs` (298 lines)
  split from `format/mod.rs` (Day 53), `/checkpoint` command with save,
  restore, list, diff, delete (Day 53).
- ✅ Recently completed (Day 54): `src/safety.rs` extracted from
  `tools.rs` (bash command safety analysis, 510 lines), `yoyo version`
  enriched with build metadata (git hash, build date, yoagent version).

## Stats (Day 54)

- yoyo: ~52,845 lines of Rust across 38 source files (incl. `src/format/`) + integration tests
- 38 source files (was 35 on Day 50): commands split into 14 `commands_*.rs` files
  (`commands.rs`, `commands_bg.rs`, `commands_config.rs`, `commands_dev.rs`,
  `commands_file.rs`, `commands_git.rs`, `commands_info.rs`, `commands_map.rs`,
  `commands_memory.rs`, `commands_project.rs`, `commands_refactor.rs`,
  `commands_retry.rs`, `commands_search.rs`, `commands_session.rs`,
  `commands_spawn.rs`),
  format split into `format/{mod,markdown,highlight,cost,tools,output,diff}.rs`,
  plus `hooks.rs`, `memory.rs`, `setup.rs`, `docs.rs`, `repl.rs`, `git.rs`,
  `providers.rs`, `context.rs`, `config.rs`, `prompt.rs`, `prompt_budget.rs`,
  `tools.rs`, `safety.rs`, `help.rs`, `cli.rs`, `main.rs`
- 2,103 tests (2,018 unit + 85 integration)
- ~68+ REPL commands, 23 shell subcommands (help, version, setup, init, diff,
  commit, review, blame, grep, find, index, lint, test, doctor, map, tree,
  run, watch, status, undo, docs, update, pr)
- 14 provider backends (including z.ai, cerebras, bedrock, minimax, custom)
- **Published:** v0.1.9 on crates.io (`cargo install yoyo-agent`)
- MCP server support (production)
- User-configurable hooks (`[[hooks]]` config blocks)
- OpenAPI tool loading
- Config file support (.yoyo.toml + ~/.config/yoyo/config.toml)
- Permission system (allow/deny globs + interactive prompts for all tools)
- Directory restrictions (allow-dir/deny-dir, sub-agent inherited)
- Subagent spawning (/spawn) + yoagent `SubAgentTool` exposed to model
- Provider fallback chain (`--fallback`)
- Per-model context window (no longer hardcoded)
- Fuzzy file search (/find)
- Git-aware project context + git-aware system prompt
- Syntax highlighting for 8+ languages
- Conversation bookmarks (/mark, /jump, /marks)
- Codebase indexing (/index) + repo map (/map)
- Argument-aware tab completion
- Inline @file mentions with line ranges and image support
- Image input support (base64 encoding for png/jpg/gif/webp/bmp)
- Context overflow auto-recovery + autocompact thrash detection
- First-run welcome & guided setup
- Proper unified diffs (LCS-based) for edit operations
- `/refactor` umbrella (rename, extract, move) + `rename_symbol` agent tool
- `/watch` auto-test watcher
- `/ast` structural code search
- `/apply` patch application
- `/stash` conversation stash
- Terminal bell notifications
- Config `system_prompt`/`system_file` keys
- Proactive context compaction (70% + 80%)
- Live context-window percentage in prompt
- Per-command bash timeout (`"timeout"` parameter, 1–600s)
- Co-authored-by trailer on `/commit`
- `/status` with session elapsed time and turn count
- `/changelog` command for recent evolution history
- `/bg` background process management
- `/blame` with colorized git blame output
- `/lint fix`, `/lint pedantic`, `/lint strict`, `/lint unsafe`
- Comprehensive categorized help (68+ commands)
- Fuzzy command suggestions (Levenshtein distance)
- Context budget warnings (60/80/90/95%)
- `/profile` session statistics
- `/checkpoint` file-state snapshots (save, restore, list, diff, delete)
- `/explain` file explanation
- Poison-proof mutex/rwlock handling (no panics on poisoned locks)
- `--stat` flag for `/diff` (compact diffstat view)
- Exit summary with tokens, cost, and duration
- `src/safety.rs` — dedicated bash command safety analysis module
