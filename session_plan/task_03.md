Title: Release v0.1.14 — version bump
Files: Cargo.toml
Issue: none

## What

Bump the version in `Cargo.toml` from `0.1.13` to `0.1.14`. The CHANGELOG entry for v0.1.14 is already written and covers 11 sessions spanning Days 82–86.

## Why

11 sessions of accumulated improvements need to ship. The CHANGELOG is drafted (committed in the previous session as "Prepare CHANGELOG.md for v0.1.14 release"). Key features in this release:

- SmartEditTool fuzzy matching and whitespace auto-fix
- Watch-mode source context injection
- Contextual command hints after prompt turns
- `/help search` for searching help text
- Per-tool cost breakdown in `/cost`
- Estimated remaining turns in `/tokens`
- `/review` effort levels (--quick/--thorough)
- `/compact --preview`
- Exit summaries with colored diffs
- LiteDescriptionTool for small models

This is a meaningful release with user-visible improvements worth shipping.

## Implementation

1. In `Cargo.toml`, change:
   ```
   version = "0.1.13"
   ```
   to:
   ```
   version = "0.1.14"
   ```

2. Verify `cargo build` succeeds with the new version.

3. Verify `cargo test` passes (the version string may appear in some test assertions — check for any hardcoded "0.1.13" references that need updating).

## Verification

- `cargo build` — clean
- `cargo test` — all pass
- `grep 'version = "0.1.14"' Cargo.toml` returns the line
- No hardcoded "0.1.13" remaining in test assertions
