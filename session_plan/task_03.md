Title: Prepare CHANGELOG.md for v0.1.14 release
Files: CHANGELOG.md
Issue: none

## What

Add the v0.1.14 section to CHANGELOG.md documenting all features, improvements, and fixes from Days 82-85 (since the v0.1.13 release on 2026-05-20).

## Why

5 days and 8 sessions of features have accumulated since v0.1.13. This is well past the threshold for a new release. The changelog needs to be written before tagging — this is the first step in the release pipeline.

## Implementation

Add a new `## [0.1.14] — 2026-05-25` section at the top of the changelog (after the header), documenting:

### Added
- **SmartEditTool fuzzy matching** — when `edit_file` fails, shows the nearest match with line numbers and highlights whitespace differences (Day 83)
- **SmartEditTool auto-fix** — whitespace-only mismatches are silently retried with correct indentation, eliminating wasted turns (Day 85)
- **`/retry --with "..."` modifier** — steer retries with additional context without retyping the full prompt (Day 83)
- **Contextual command hints** — dim one-line suggestions after prompt turns based on what just happened (Day 84)
- **`/help search`** — keyword search across all commands, scored by relevance (Day 84)
- **`/add` suggests related files** — after adding a file, suggests test files, imports, and module companions (Day 84)
- **`/add` token cost estimates** — shows approximate token cost of adding each file (Day 83)
- **`LiteDescriptionTool`** — adds JSON examples to tool descriptions for small language models in `--lite` mode (Day 84)
- **Per-tool usage summary** — `/cost` and `/tokens` now show per-tool call counts and failure rates (Day 85)
- **Estimated remaining turns** — `/tokens` and `/profile` estimate how many more turns fit in the context window (Day 85)
- **`/review` effort levels** — `--quick` for bugs/security only, `--thorough` for exhaustive deep dive (Day 85)
- **Exit summary with colored diffs** — on session end, shows a compact diff of what changed (Day 83)
- **`/goal` system prompt injection** — goals set with `/goal set` are injected into the system prompt for persistent awareness (Day 83)
- **`/blindspot` skill** — structured critique mode with 7 analysis dimensions and adjustable intensity (Day 83)

### Improved
- **Richer `/status`** — now shows active goal, watch command, active modes (architect/teach/read), and session file changes (Day 84)
- **Relative timestamps in `/memories`** — shows "3d ago" instead of raw ISO timestamps (Day 85)
- **SmartEditTool extracted to `src/smart_edit.rs`** — 758-line module with clear separation from tool_wrappers (Day 85)
- **Turn change summary** — dim line after each AI turn showing which files were just modified (Day 82)

### Fixed
- (Check git log for any specific bug fixes in this window — likely serial test fixes or minor corrections)

## Format
Follow the existing changelog format exactly (Keep a Changelog style). Include the session/Day reference in parentheses after each item. The date should be `2026-05-25`. Write the intro paragraph summarizing the release theme (8 sessions spanning Days 82-85).

## Constraints
- Only modify CHANGELOG.md
- Don't remove or modify any existing entries
- The version number is `0.1.14`
- Don't actually create the git tag — just prepare the changelog entry
