Title: Add tests for main.rs pure functions — build_json_output and apply_config_flags
Files: src/main.rs
Issue: none

## What

main.rs is at 1.6% test density (16 tests for 962 lines). Several pure/near-pure functions have zero tests. Add tests to harden the entry point logic.

## Specific functions to test

1. **`build_json_output`** (line ~123) — constructs JSON for `--output-format json` piped mode. Test:
   - Basic text output produces valid JSON with `type`, `content` fields
   - Empty input produces valid JSON
   - Special characters (quotes, newlines, unicode) in content are properly escaped
   - Verify the JSON structure matches what downstream consumers expect

2. **`apply_config_flags`** (line ~474) — maps Config fields to global state. Test:
   - Returns `true` when config has verbose flag
   - Sets verbose mode when config.verbose is true
   - Sets quiet mode when config.quiet is true
   - Handles default config (all false) correctly
   - Note: this function calls global setters, so tests should verify side effects

3. **`apply_cli_flags`** (line ~447) — processes CLI flag strings. Test:
   - `--verbose` sets verbose mode
   - `--quiet` sets quiet mode  
   - `--no-color` disables color
   - `--no-bell` disables bell
   - `--no-notify` disables notifications
   - Unknown flags are silently ignored

4. **`looks_like_slash_command`** already has 4 tests — add edge cases:
   - Input with only whitespace before slash
   - Input with `/` followed by numbers (e.g., `/123`)
   - Empty string

## Rules
- Don't delete existing tests
- Each test should be independent
- Use `#[test]` attribute, keep tests in the existing `mod tests` block
- Run `cargo test` to verify all pass
