# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A self-evolving coding agent CLI built on [yoagent](https://github.com/yologdev/yoagent). The agent spans multiple Rust source files under `src/`. A GitHub Actions cron job (`scripts/evolve.sh`) runs the agent hourly using a 3-phase pipeline (plan → implement → respond), which reads its own source, picks improvements, implements them, and commits — if tests pass. All runs use a flat 8h gap (~3/day). Sponsors get benefit tiers (issue priority, shoutout issues, listing eligibility) but no run-frequency speedup. One-time sponsors ($2+) get 1 accelerated run that bypasses the gap (only consumed when they have open issues; tracked in `sponsors/credits.json`).

**Sponsor benefit tiers:**

Monthly recurring (benefits only):
- $5/mo: Issue priority (💖)
- $10/mo: Priority + shoutout issue
- $25/mo: Above + SPONSORS.md eligible
- $50/mo: Above + README eligible

One-time (cumulative — each tier includes all benefits below it):
- $2: 1 accelerated run (bypasses 8h gap)
- $5: Accelerated run + issue priority (14 days)
- $10: Above + shoutout issue (30 days)
- $20: Above + SPONSORS.md eligible (30 days)
- $50: Above + priority for 60 days + SPONSORS.md + README eligible
- $1,000 💎 Genesis: All above + permanent priority + SPONSORS.md + README + journal acknowledgment (never expires)

## Build & Test Commands

```bash
cargo build              # Build
cargo test               # Run tests
cargo clippy --all-targets -- -D warnings   # Lint (CI treats warnings as errors)
cargo fmt -- --check     # Format check
cargo fmt                # Auto-format
```

CI runs all four checks (build, test, clippy with -D warnings, fmt check) on PR to main. A separate Pages workflow builds and deploys the website on push to main.

To run the agent interactively:
```bash
ANTHROPIC_API_KEY=sk-... cargo run
ANTHROPIC_API_KEY=sk-... cargo run -- --model claude-opus-4-6 --skills ./skills
```

To trigger a full evolution cycle:
```bash
ANTHROPIC_API_KEY=sk-... ./scripts/evolve.sh
```

## Architecture

**Build** (`build.rs`): Sets compile-time env vars `GIT_HASH`, `BUILD_DATE`, `DAY_COUNT`, and `YOAGENT_VERSION` from git/Cargo.lock/DAY_COUNT file. All overridable by env var at build time (CI/release builds).

**Multi-file agent** (`src/`):
- `main.rs` — agent core, REPL, streaming event handling, rendering with ANSI colors, sub-agent tool integration, AskUserTool (interactive question-asking)
- `hooks.rs` — Hook trait, HookRegistry, AuditHook, HookedTool wrapper, maybe_hook helper
- `tools.rs` — StreamingBashTool, RenameSymbolTool, AskUserTool, TodoTool, tool builders, RTK proxy integration
- `update.rs` — version comparison (`version_is_newer`) and update checking (`check_for_update`) against GitHub releases
- `safety.rs` — bash command safety analysis, destructive pattern detection
- `cli.rs` — CLI argument parsing, subcommands, configuration (delegates `--help` text to `help.rs`)
- `commands.rs` — slash command dispatch, grouped /help, custom command discovery (loads user-defined `.md` files from `.yoyo/commands/` and `~/.yoyo/commands/`)
- `help.rs` — canonical source for all help content: `cli_help_text()` (`--help` output), `/help` REPL help, per-command detailed help
- `config.rs` — permission config, directory restrictions, MCP server config, TOML parsing helpers
- `context.rs` — project context loading, file listing, git status, recently changed files
- `providers.rs` — provider constants (KNOWN_PROVIDERS), API key env vars, default/known models per provider
- `format/mod.rs` — Color, constants, utility functions, re-exports
- `format/diff.rs` — LCS-based line diff algorithm, colored unified diff rendering
- `format/output.rs` — tool output compression, filtering, truncation, batch summary, indentation
- `format/highlight.rs` — syntax highlighting for code, JSON, YAML, TOML
- `format/cost.rs` — pricing, cost display, token formatting
- `format/markdown.rs` — MarkdownRenderer for streaming markdown output
- `format/tools.rs` — Spinner, ToolProgressTimer, ActiveToolState, ThinkBlockFilter
- `prompt.rs` — prompt execution, agent interaction, streaming event handling, auto-retry logic
- `prompt_budget.rs` — session wall-clock budget + audit log helpers (extracted from `prompt.rs`)
- `session.rs` — session tracking types: SessionChanges, TurnSnapshot, TurnHistory, format_changes (extracted from `prompt.rs`)

Uses `yoagent::Agent` with `AnthropicProvider`, `default_tools()`, and an optional `SkillSet`.

**Documentation** (`docs/`): mdbook source in `docs/src/`, config in `docs/book.toml`. Output goes to `site/book/` (gitignored). The journal homepage (`site/index.html`) is built by `scripts/build_site.py`. Both are built and deployed by the Pages workflow (`.github/workflows/pages.yml`), not during evolution.

**Evolution loop** (`scripts/evolve.sh`): pipeline:
1. Verifies build → fetches GitHub issues (community, self, help-wanted) via `gh` CLI + `scripts/format_issues.py` → scans for pending replies on previously touched issues
2. **Phase A** (Planning): Agent reads everything, writes task files to `session_plan/`
3. **Phase B** (Implementation): Agents execute each task (20 min each), with two fix loops: build/test failures get up to 10 fix attempts (10 min each), then the evaluator runs and rejections get up to 9 more fix attempts (10 min each). Reverts only after all fix attempts are exhausted. Max 3 tasks per session.
4. Verifies build, fixes or reverts → agent-driven issue responses (agent directly calls `gh issue comment`/`close`) → pushes

**Wall-clock budget** (opt-in): The hourly cron can fire while a previous session is still running, causing GH Actions to cancel the in-flight run (#262). Set `YOYO_SESSION_BUDGET_SECS=2700` (45 min default if set but unparseable) to enable a soft, agent-side wall-clock budget. The helper `prompt::session_budget_remaining()` returns `Some(remaining)` when the env var is set and `None` otherwise (sessions are unbounded by default for interactive use). The timer starts on the first call, not at process startup, so cold-start time doesn't eat into agent work. `session_budget_remaining()` is now consulted at the top of each retry attempt in `run_prompt_auto_retry`, `run_prompt_auto_retry_with_content`, and the watch-mode fix loop via `session_budget_exhausted(30)`; when ≤30s remain, retries stop early and the current outcome is returned. The shell-side export in `scripts/evolve.sh` is a separate (human-approved) follow-up — until then the env var stays unset and behavior is unchanged.

**Skills** (`skills/`): Markdown files with YAML frontmatter loaded via `--skills ./skills`. Six core skills (immutable, `core: true` + `origin: creator`) define the agent's foundational capabilities:
- `self-assess` — read own code, try tasks, find bugs/gaps
- `evolve` — safely modify source, test, revert on failure
- `communicate` — write journal entries and issue responses
- `research` — internet lookups and knowledge caching
- `skill-evolve` — autonomous meta-skill: refines/creates/retires non-core skills based on past-session evidence (cron-driven, gated)
- `skill-creator` — on-demand meta-skill: scaffolds a new skill when the human creator or a community issue explicitly asks for one (interview-driven, no autonomous gating)

Additional skills (`origin: yoyo`, eligible for skill-evolve to refine/retire):
- `social` — community interaction via GitHub Discussions
- `family` — fork registration, introduction, and cross-fork discussion via the yoyobook discussion category
- `release` — binary release pipeline

**skill-evolve vs skill-creator** — both can produce new skills, but they're complementary, not redundant:
- skill-evolve runs autonomously on cron, mines past sessions for recurring patterns, gated by ≥3-session recurrence + 24h cooldown + diff-scope guard. Strong safety properties.
- skill-creator runs on demand inside a normal evolve session when explicitly invoked, no recurrence gate, human-in-the-loop. Use only when a person asks for a skill — never as autonomous self-creation (that belongs in skill-evolve).

**Discussion categories**: General, Journal Club, The Show, Ideas, and `yoyobook` (family discussions for yoyo forks — registration address book, introductions, cross-fork conversation). The `yoyobook` category is created manually in repo settings; `format_discussions.py` fetches all categories automatically.

**Memory system** (`memory/`): Two-layer architecture — append-only JSONL archives (source of truth, never compressed) and active context markdown (regenerated daily by `.github/workflows/synthesize.yml` with time-weighted compression tiers):
- `memory/learnings.jsonl` — self-reflection archive. Each line: `{"type":"lesson","day":N,"ts":"ISO8601","source":"...","title":"...","context":"...","takeaway":"..."}`
- `memory/social_learnings.jsonl` — social insight archive. Each line: `{"type":"social","day":N,"ts":"ISO8601","source":"...","who":"@user","insight":"..."}`
- `memory/active_learnings.md` — synthesized prompt context (recent=full, medium=condensed, old=themed groups)
- `memory/active_social_learnings.md` — synthesized social prompt context
- Archives are appended via `python3` with `json.dumps()` (never `echo` — prevents quote-breaking). Admission gate: only write if genuinely novel AND would change future behavior.
- Context loaded centrally by `scripts/yoyo_context.sh` → `$YOYO_CONTEXT` (WHO YOU ARE, YOUR VOICE, SELF-WISDOM, SOCIAL WISDOM, YOUR ECONOMICS, YOUR SPONSORS sections)

**Release pipeline** (`.github/workflows/release.yml`): Triggered by `v*` tags. Builds binaries for 4 targets (Linux x86_64, macOS Intel, macOS ARM, Windows x86_64) and publishes a GitHub Release with tarballs/zips + SHA256 checksums. Install scripts:
- `install.sh` — `curl -fsSL ... | bash` for macOS/Linux
- `install.ps1` — `irm ... | iex` for Windows PowerShell

**State files** (read/written by the agent during evolution):
- `IDENTITY.md` — the agent's constitution and rules (DO NOT MODIFY)
- `PERSONALITY.md` — voice and values (DO NOT MODIFY)
- `journals/JOURNAL.md` — chronological log of evolution sessions (append at top, never delete). External project journals (e.g., `journals/llm-wiki.md`) also live here.
- `DAY_COUNT` — integer tracking current evolution day
- `session_plan/` — ephemeral directory with per-task files (task_01.md, task_02.md, etc.), written by Phase A planning agent (gitignored)
- `.yoyo/commands/` — project-local custom slash command definitions (`.md` files); `~/.yoyo/commands/` for global commands
- `ISSUES_TODAY.md` — ephemeral, generated during evolution from GitHub issues (gitignored)
- `ECONOMICS.md` — what money and sponsorship mean to yoyo (DO NOT MODIFY)
- `SPONSORS.md` — auto-maintained sponsor recognition (only additions, never removals; amounts shown so yoyo understands the investment)
- `sponsors/sponsor_info.json` — single source of truth for sponsor state (recurring + one-time, with run_used, shouted_out, benefit_expires). Rebuilt by `scripts/refresh_sponsors.py`; only the `run_used` flag is mutated by `evolve.sh` when consuming an accelerated run.

**Skill evolution loop** (decoupled from main evolve pipeline):
- `skills/skill-evolve/SKILL.md` — meta-skill that refines/creates/retires *other* skills based on past-session evidence. Three hard rules: (1) only edit skills declaring `origin: yoyo` (allow-list); (2) never edit itself; (3) one mutation per cycle.
- `scripts/skill_evolve.sh` — one cycle entry point. Gates: dirty-tree refusal, session-counter ≥ 5, 24h cooldown, `cargo build && cargo test` green. Post-agent: diff-scope guard (`origin: yoyo` + not `core: true` + within allow-list), build/test re-verify, revert on any violation.
- `.github/workflows/skill-evolve.yml` — hourly cron at `:30` (off-phase from evolve which runs at `:00`); runs `scripts/skill_evolve.sh` which exits silently if gates aren't met.
- `audit-log` branch — long-lived data-only branch, never merges to main. `evolve.sh` pushes per-session evidence (`audit.jsonl` from `--audit`, `outcome.json`, `transcripts/*.log`) into `sessions/day-N-<ts>/`. skill-evolve clones it into a worktree to mine recurrence/scoring signals.
- `skills/_journal.md` — append-only ledger of every skill-evolution event (init, refine, create, retire, meta-suggestion, refused, NO-OP).
- `skills_attic/` — soft-delete destination for retired skills (sibling of `skills/`, NOT scanned by `--skills`).
- `.skill_evolve_counter` (tracked) — bumped at end of every evolve session; reset to 0 by skill-evolve cycles.
- `.skill_evolve_last_run` (gitignored) — epoch timestamp for cooldown.
- `scripts/skill_evolve_report.py` — Layer-3 observability report (per-skill score/eligibility, event log, recurrence trend).

**Skill provenance via `origin:` frontmatter field** — every skill declares one of:
- `origin: creator` — written by the human creator (Yuanhao or fork creator). Immutable. Backed up by `core: true` on the four core skills.
- `origin: yoyo` — written by yoyo (via skill-evolve, or in past evolutions like `social`/`family`/`release`). Eligible for skill-evolve to refine/retire.
- `origin: marketplace` (or `gh:user/repo`, etc.) — installed third-party skills. Off-limits — upstream owns them.
- (missing) — unknown provenance. Off-limits (default-safe).

This is enforced both by HARD RULE #1 in the meta-skill (LLM-side) and by the diff-scope guard in `scripts/skill_evolve.sh` (harness-side).

**Skill scoring inputs** — `origin: yoyo` skills carry an additional `keywords:` list in their frontmatter (e.g., `keywords: ["gh api graphql", "discussion"]` for `social`). skill-evolve uses these to detect "this skill was used in session N" by grepping each session's `audit.jsonl` for any keyword. `last_used`, `uses`, and `wins` are computed from this signal.


## MCP gotchas

**Tool-name collisions (Day 39):** If an MCP server exposes a tool whose name matches one of yoyo's builtins (`bash`, `read_file`, `write_file`, `edit_file`, `list_files`, `search`, `rename_symbol`, `ask_user`, `todo`, `sub_agent`), the Anthropic API will reject the first turn with `"Tool names must be unique"` and the session dies. The flagship reference server `@modelcontextprotocol/server-filesystem` collides on `read_file` AND `write_file`, so the common case was broken until the guard landed.

yoyo now runs a pre-flight tool listing (via a short-lived `yoagent::mcp::McpClient`) before every `with_mcp_server_stdio` call. If any MCP tool name appears in `BUILTIN_TOOL_NAMES` (defined in `src/main.rs`), the whole server is skipped with a clear stderr warning naming the colliding tool(s). Non-colliding servers connect normally. If the pre-flight itself fails (e.g. server can't spawn), we fall through to yoagent's connect so the user sees the real diagnostic.

Keep `BUILTIN_TOOL_NAMES` in sync with `tools::build_tools` whenever a new builtin is added — the pure helper `detect_mcp_collisions` is unit-tested in `src/main.rs` against the filesystem server's known tool set as a regression guard.

## yoagent: Don't Reinvent the Wheel

yoyo is built on [yoagent](https://github.com/yologdev/yoagent). Before implementing any agent-related or low-level agent feature, **check if yoagent already provides it**. Past examples of reinvented wheels:
- Manual context compaction (`compact_agent`, `auto_compact_if_needed`) — yoagent has `ContextConfig`, `CompactionStrategy`, and built-in 3-level compaction
- Hardcoded token limits — yoagent has `ExecutionLimits` (max_turns, max_total_tokens, max_duration)
- Ignoring `MessageStart`/`MessageEnd` events — yoagent streams these for agent stop messages

**Before building agent infrastructure in src/:**
1. Search yoagent's source (`~/.cargo/registry/src/*/yoagent-*/src/`) for existing features
2. Check yoagent's `Agent` builder methods, tool traits, callbacks (`on_before_turn`, `on_after_turn`, `on_error`), and examples
3. If yoagent has it → use it. If yoagent almost has it → file an issue on yoagent. If yoagent doesn't have it → build it in yoyo.

Key yoagent features available: `SubAgentTool`, `ContextConfig`, `ExecutionLimits`, `CompactionStrategy`, `AgentEvent` stream, `default_tools()`, `SkillSet`, `with_sub_agent()`.

**yoagent 0.7.x prompt lifecycle gotcha (Issue #258):** `agent.prompt()` / `agent.prompt_messages()` spawns the agent loop into a tokio task and returns the event receiver immediately. The agent's internal `self.messages` is NOT updated until `agent.finish().await` is called. If you read `agent.messages()` (or `total_tokens(agent.messages())`) right after draining the event stream WITHOUT calling `finish()` first, you will see the stale pre-prompt state — which silently breaks anything that depends on message count (e.g., the context-window usage bar). Always call `agent.finish().await` between event drain and message read.

## Safety Rules

These are enforced by the `evolve` skill and `evolve.sh`:
- Never modify `IDENTITY.md`, `PERSONALITY.md`, `ECONOMICS.md`, `scripts/evolve.sh`, `scripts/format_issues.py`, `scripts/build_site.py`, or `.github/workflows/`
- Every code change must pass `cargo build && cargo test`
- If build fails after changes, revert with `git checkout -- src/ Cargo.toml Cargo.lock`
- Never delete existing tests
- Multiple tasks per evolution session, each verified independently
- Write tests before adding features
- **Never use byte indexing on strings.** `s[..n]`, `s.truncate(n)`, and `s.split_at(n)` panic if `n` falls inside a multi-byte UTF-8 character. Use `is_char_boundary()` to find a safe boundary first:
  ```rust
  // BAD: panics on multi-byte chars like ✓ (3 bytes)
  acc.truncate(max_bytes);
  // GOOD: find nearest char boundary
  let mut b = max_bytes;
  while b > 0 && !acc.is_char_boundary(b) { b -= 1; }
  acc.truncate(b);
  ```
  This caused planning agent crashes in production (#250).
- **`run_git()` has a `#[cfg(test)]` destructive-command guard.** During `cargo test`, calling `run_git()` with a destructive subcommand (commit, revert, reset, push, checkout, etc.) from the project root panics. Tests that need destructive git operations must use a temp directory. This prevents tests from accidentally mutating the real repo (which caused a 6-session deadlock across Days 42-44).
