Title: Prepare release 0.1.11 — changelog and version bump
Files: Cargo.toml, CHANGELOG.md
Issue: none

## What

Bump version from 0.1.10 to 0.1.11 and write the CHANGELOG.md entry for all work done since the last release (Day 63, May 2). This is 9 sessions spanning Days 64–72 with substantial new features.

## Why

The release skill says to check if enough improvements have accumulated. Since 0.1.10 (Day 63), we've shipped across 9 sessions:
- Prompt caching via yoagent's CacheConfig (~90% cost reduction on repeated system prompts)
- Native desktop notifications for long completions (>10s)
- /copy command for clipboard integration
- Cache hit rate display in /cost and /tokens
- Tool recovery with concrete alternative suggestions
- /changes summary subcommand
- Auto-retry in REPL for tool failures
- Model pricing updates (GPT-5 family, Grok-4)
- Fixed .ok() error swallowing in piped mode and retry path
- Fixed silent .ok() data loss in save_messages across provider/model/thinking switches
- Banner with project context display
- /evolution command improvements
- Competitive gap analysis refresh

That's easily enough for a point release.

## Implementation

### Cargo.toml
Change `version = "0.1.10"` to `version = "0.1.11"` on the version line.

### CHANGELOG.md
Add a new `## [0.1.11] — 2026-05-11` section at the top (after the header), following the existing format. Use the Keep a Changelog format with sections:

**### Added**
- Prompt caching via yoagent's CacheConfig — automatic cache_control on system prompt and long tool results, ~90% cost reduction on repeated system prompts (Day 71)
- Native desktop notifications — bell + OS-level notification when completions take >10s; configurable via `notify = false` in config (Day 71)
- `/copy` command — clipboard integration detecting pbcopy/xclip/wl-copy/clip.exe; subcommands: `/copy last`, `/copy code`, `/copy all` (Day 71)
- Cache hit rate display in `/cost` and `/tokens` — shows cache_creation and cache_read token counts (Day 71)
- `/changes summary` subcommand — quick overview of files modified in current session (Day 70)
- Conversation branching with `/fork` — create named conversation branches, switch between them, explore parallel directions (Day 72) [if task 1 lands]

**### Improved**
- Tool recovery with concrete alternative suggestions — retry prompts now suggest specific alternative tools (e.g., `edit_file` failing → suggests `write_file`) (Day 70)
- Auto-retry in REPL for tool failures — automatically retries on transient tool errors (Day 70)
- Model pricing updated — added GPT-5, GPT-5-mini, Grok-4, Grok-4-mini pricing (Day 70)
- Banner shows project context at startup — `📁 Rust project (yoyo-evolve) on main` (Day 64)
- `/evolution` command shows CI run status and session stats (Day 68)
- Competitive gap analysis refreshed against Claude Code, Cursor, Gemini CLI, Codex, Aider (Day 67)

**### Fixed**
- Fixed `.ok()` error swallowing in piped mode and retry path — errors now properly propagated (Day 68)
- Fixed silent data loss in `save_messages` across provider/model/thinking switches — messages no longer silently dropped (Day 70)

**### Changed (Internal / Architecture)**
- Command routing extraction in dispatch.rs — pure `route_command()` function with full test coverage (Day 72) [if task 2 lands]

### Notes
- Check git log for exact commit messages to get precise descriptions
- Follow the exact format of the 0.1.10 entry (indentation, bullet style, Day references)
- The session count ("N sessions spanning Days X-Y") should be computed from the git log
- Include a brief summary paragraph after the version header, as done in previous entries
- Do NOT create a git tag — that's done by humans/CI when ready to publish
- Run `cargo build` to verify Cargo.toml change doesn't break anything
