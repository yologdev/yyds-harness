Title: Add unit tests for commands_map.rs — symbol extraction and repo map formatting
Files: src/commands_map.rs
Issue: none

## Context

`commands_map.rs` is the largest file in the project at 3,605 lines with 71 tests, giving a
50 lines/test ratio — the worst coverage ratio of any file. It contains critical functionality:
repo map building, multi-language symbol extraction, relevance ranking, and formatting.

The recent Day 78 session added relevance-ranked repo map, but the ranking logic itself has
limited test coverage. Symbol extraction for the 5 newly-added languages (C#, PHP, Kotlin,
Swift, Scala) was added in Day 77 but tests for those extractors are sparse.

## What to do

Add focused unit tests for under-tested areas:

1. **Relevance ranking tests** — Test `recently_modified_files()` and the ranking logic in
   `build_repo_map_with_backend()`:
   - Test that recently modified files appear before unmodified files
   - Test that files with more symbols get higher priority
   - Test edge cases: empty file list, all files equally recent

2. **New language symbol extraction** — Add tests for the 5 languages added on Day 77:
   - C# class, method, property extraction
   - PHP class, function extraction
   - Kotlin class, fun extraction
   - Swift class, func, struct extraction
   - Scala class, def, object extraction
   - Each test should verify that `extract_symbols()` correctly identifies symbol names,
     kinds, and line numbers from a small representative snippet

3. **Format tests** — Test `format_repo_map()` and `format_repo_map_colored()`:
   - Test that output contains expected file paths and symbol names
   - Test truncation behavior when output exceeds limits
   - Test that colored output contains ANSI escape codes

4. **Edge cases for detect_language**:
   - Test that `.cs` → `csharp`, `.php` → `php`, `.kt` → `kotlin`, etc.
   - Test unknown extensions return `None` or `"unknown"`

## Important notes

- Only add tests, do not change any existing code
- Use small inline string snippets for language extraction tests (don't read real files)
- Target adding 15-20 new tests to bring the ratio closer to 40 lines/test
- Each test should be focused on one assertion or a closely related group
