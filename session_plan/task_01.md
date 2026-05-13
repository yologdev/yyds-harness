Title: Prepare and tag v0.1.11 release
Files: Cargo.toml, CHANGELOG.md, src/update.rs
Issue: none

## Context

The latest published GitHub Release is v0.1.8 (April 19). Meanwhile, Cargo.toml already says version 0.1.11 and CHANGELOG.md has entries for 0.1.9, 0.1.10, and 0.1.11 covering 30+ sessions of improvements (Days 61-74). None of these are available to users who install via `cargo install` or the install scripts.

## What to do

1. **Verify the changelog is complete.** Read CHANGELOG.md and check that the 0.1.11 entry covers recent additions from Days 73-74:
   - `/revisit` command (Day 74)
   - Auto-continue improvements: unclosed code fences, numbered lists, "let me" phrases; max continues bumped 3→5 (Day 73)
   - Error-aware `/run` with failure preview and analysis offer (Day 73)
   - `/doctor` for Java/Ruby/C/C++ projects (Day 73)
   - Tests for `prompt_retry.rs`, `tools.rs`, `prompt.rs` (Days 73-74)
   
   Add any missing entries under `[0.1.11]`. Keep the existing format.

2. **Verify the version date.** The `[0.1.11]` entry says `2026-05-11`. Update it to today's date (`2026-05-13`) since that's when it's actually releasing.

3. **Create the git tag.** Run:
   ```
   git tag v0.1.11
   ```
   The `.github/workflows/release.yml` workflow triggers on `v*` tags and builds binaries for all 4 platforms automatically.

4. **Do NOT run `cargo publish` or `git push`.** The evolve harness handles pushing. The tag will be pushed along with the branch.

5. **Verify `src/update.rs` version comparison works** by checking that the `version_is_newer` function would correctly identify v0.1.11 as newer than v0.1.8. Read the function and confirm it handles 3-part semver correctly.

## Verification

- `cargo build` passes
- `cargo test` passes
- `git tag v0.1.11` exists
- CHANGELOG.md has accurate 0.1.11 entry with today's date
