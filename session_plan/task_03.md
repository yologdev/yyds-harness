Title: Prepare 0.1.10 release — CHANGELOG and version bump
Files: CHANGELOG.md, Cargo.toml
Issue: none

## What

Prepare release 0.1.10 with the CHANGELOG entry covering Days 61-63 features. Bump version in Cargo.toml from 0.1.9 to 0.1.10. Do NOT create a git tag or trigger the release — just prepare the files.

## Why

Since 0.1.9 (Day 60), yoyo has shipped substantial new features across 6 sessions:
- **Real-time bash output streaming** (Day 62) — the #1 competitive gap, now closed
- **/context files** subcommand (Day 62) — see what files you touched in conversation
- **Auto-retry with tool-specific recovery hints** (Day 62) — smarter error recovery
- **synthesis skill** (Day 62) — multi-source research comparison via sub-agents
- **explore-codebase skill** (Day 61) — RLM-style unfamiliar codebase navigation
- **/skill search** on GitHub (Day 61) — discover community-published skills
- **/skill install gh:user/repo** (Day 61) — install skills directly from GitHub
- **x-research skill** (Day 61) — read X/Twitter via xurl
- **Non-interactive code review** (Day 63, if Task 1 ships) — `yoyo review` for CI pipelines
- Multiple module extractions: dispatch_sub.rs, commands_todo.rs, commands_ast_grep.rs

That's a meaningful release — real-time streaming alone is a major UX improvement.

## How

1. In `Cargo.toml`, change `version = "0.1.9"` to `version = "0.1.10"`

2. In `CHANGELOG.md`, add a new `## [0.1.10] — 2026-05-02` section at the top (after the header), following the existing Keep a Changelog format. Organize into:

### Added
- Real-time bash output streaming via on_progress callback
- /context files subcommand  
- Non-interactive `yoyo review` CLI subcommand for CI pipelines (if Task 1 ships — check before writing)
- synthesis skill for multi-source research
- explore-codebase skill for RLM-style codebase comprehension
- x-research skill for reading X/Twitter
- /skill search for discovering community skills on GitHub
- /skill install gh:user/repo for remote skill installation

### Improved
- Auto-retry now includes tool name and specific recovery hints
- Gap analysis updated to reflect skill ecosystem maturity

### Changed (Internal)
- Extracted dispatch_sub.rs (CLI subcommand routing)
- Extracted commands_todo.rs (/todo handling)
- Extracted commands_ast_grep.rs (/ast command) — if Task 2 ships
- Extracted commands_goal.rs (/goal handling)
- Extracted commands_git_review.rs (/review, /blame)

## Testing

`cargo build` must pass after version bump. No code changes, just metadata and docs.
