Title: Structured Rust compiler error parsing for smarter watch fix prompts
Files: src/watch.rs
Issue: none

## Goal

Make the auto-fix loop (watch mode) significantly smarter by parsing Rust compiler errors
into structured data and providing targeted fix hints. Currently, `build_watch_fix_prompt`
passes raw compiler output with a generic "this is a lint/test failure" hint. We can do much
better by extracting specific error patterns.

## What to build

1. Add a `struct CompilerError` with fields: `code` (e.g. "E0382"), `message`, `file`, `line`,
   `category` (borrow, type, lifetime, import, unused, syntax, test_assertion, other).

2. Add `fn parse_rust_errors(output: &str) -> Vec<CompilerError>` that extracts structured
   errors from `cargo build`/`cargo test`/`cargo clippy` output. Key patterns to parse:
   - `error[E0382]: borrow of moved value` → category: borrow
   - `error[E0308]: mismatched types` → category: type  
   - `error[E0106]: missing lifetime` → category: lifetime
   - `warning: unused import` / `warning: unused variable` → category: unused
   - `error: cannot find value` / `error[E0433]` → category: import
   - `thread '...' panicked at` → category: test_assertion
   - Any `error[EXXXX]` → extract the code
   - File path + line number from `  --> src/foo.rs:42:5`

3. Add `fn error_category_hint(category: &str) -> &'static str` that returns targeted advice:
   - borrow: "This is a borrow checker error. Consider cloning the value, restructuring ownership, or using references."
   - type: "This is a type mismatch. Check the expected vs actual types, consider conversions or generics."
   - lifetime: "This is a lifetime error. Consider adding explicit lifetime annotations or restructuring borrows."
   - import: "Missing import or unresolved name. Add the missing `use` statement or check the module path."
   - unused: "Unused code warnings. Remove the unused items or prefix with underscore if intentionally unused."
   - test_assertion: "Test assertion failed. Read the expected vs actual values, fix the implementation or update the test."

4. Enhance `build_watch_fix_prompt` to:
   - Call `parse_rust_errors` on the output
   - If errors are found, prepend a structured summary: "Found N errors: M borrow, K type, etc."
   - Include the most specific hint for the dominant error category
   - Still include the raw (truncated) output for context

5. Write thorough tests:
   - Test `parse_rust_errors` with real-looking Rust compiler output
   - Test each error category detection
   - Test `error_category_hint` returns non-empty strings
   - Test `build_watch_fix_prompt` with structured error output includes the summary
   - Test edge cases: empty output, non-Rust output, mixed errors

## Constraints
- Only modify `src/watch.rs`
- Keep the existing `classify_watch_command` logic — the new parser adds detail on top of it
- The enhanced prompt should only activate for Rust projects (when errors match Rust patterns)
- Non-Rust output falls through to the existing generic prompt unchanged
