Title: Prepare CHANGELOG for v0.1.12 release
Files: CHANGELOG.md
Issue: none

## What to do

Prepare the CHANGELOG.md for a v0.1.12 release. Since v0.1.11 (Day 74, 2026-05-13), 11+ tasks have shipped across Days 75-78 spanning 8 sessions. This is a clean batch worth releasing.

**Features to document (scan git log for Task commits since Day 74):**

### Added
- **`--print` mode** — raw output only, no banner/cost/chrome, for scripting and pipeline use (Days 76-77)
- **`--no-tools` flag** — suppress all tools including sub_agent, shared_state, and MCP connections for pure conversation mode (Day 78)
- **`--disallowed-tools` flag** — selectively block specific tools by name (Day 76)
- **Session resume summary** — shows last user/assistant messages when restoring with `--continue` (Day 78)
- **`/compact` arguments** — `/compact 5` to keep last N exchanges, `/compact all` to compress everything (Day 77)
- **Project-type context hints** — auto-injects development conventions (test commands, lint tools) for detected project types when no instruction file exists (Day 76)
- **`/spawn --bg` flag** — launch sub-agents in background and keep working (Day 76)
- **`/tokens` breakdown** — shows where context tokens went (system prompt, conversation, tool output) (Day 76)
- **5 new languages in `/map`** — C#, PHP, Kotlin, Swift, Scala symbol extraction (Day 77)

### Improved
- **Relevance-ranked repo map** — files ranked by recency and symbol density before truncation, instead of alphabetical cutoff (Day 78)
- **Recovery hints wired to all tools** — `RecoveryHintTool` now wraps all tools, not just during retry (Day 75)
- **`/retry` carries failure context** — retry includes what went wrong and suggested recovery path (Day 75)

### Fixed
- **Auto-watch message leak in `--print` mode** — suppressed chrome output (Day 77)
- **`print_usage` and `print_context_usage` leak in `--print`/quiet mode** — properly suppressed (Day 77)
- **Architect mode test flakiness** — added `#[serial]` to global-state-mutating tests (Day 77)

### Changed (Internal)
- **Test coverage expansion** — new tests for `dispatch.rs`, `tools.rs`, `tool_wrappers.rs`, `commands_update.rs` (Days 75-78)
- **`cli_config.rs` extraction** — constants and Config struct separated from `cli.rs` (Day 75)

**Steps:**
1. Read the current CHANGELOG.md header format
2. Add a new `## [0.1.12] — 2026-05-17` section at the top (after the preamble)
3. Organize features into Added/Improved/Fixed/Changed sections following the existing format
4. Verify the git log to ensure nothing is missed
5. Update the version in `Cargo.toml` from the current version to `0.1.12`
6. Run `cargo build` to verify the version change compiles

**Key constraint:** Don't tag or release — just prepare the CHANGELOG and bump Cargo.toml version. The actual release is triggered by pushing a `v0.1.12` tag, which is a separate step (human or future session).
