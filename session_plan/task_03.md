Title: Auto-detect related files when /add is used, suggest them
Files: src/commands_file.rs
Issue: none

## Problem
When a developer adds a file to context with `/add src/foo.rs`, they often need related files too — the test file, files that import it, the module declaration. Currently they have to know and add each one manually. Aider's repo map integration is cited as more mature; this is a step toward closing that gap.

## What to Build
After processing `/add <file>`, if the added file is a source file, suggest related files the user might also want to add. Show suggestions as a dim hint line, not as an automatic action.

### Related File Detection Rules (simple, no AST needed):
1. **Test file:** If adding `src/foo.rs`, check if `tests/foo.rs`, `src/foo_test.rs`, or a `#[cfg(test)]` module exists in the same file (just note "contains inline tests")
2. **Module parent:** If adding `src/foo/bar.rs`, suggest `src/foo/mod.rs`
3. **Corresponding impl/header:** If adding `src/foo.rs` and `src/foo/mod.rs` exists, suggest it
4. **For Rust specifically:** Parse `use crate::` and `mod ` declarations in the added file to find sibling modules. E.g., if `src/commands.rs` has `use crate::dispatch`, suggest `src/dispatch.rs`

### Output Format
After the normal `/add` output (file content + token estimate), show:
```
  💡 Related: src/foo_test.rs, src/foo/mod.rs (use /add to include)
```
- Only show if there are 1-3 related files found (don't overwhelm)
- Only for source files (`.rs`, `.py`, `.ts`, `.js`, `.go`, `.java`, `.rb`, `.cpp`, `.c`, `.h`)
- DIM color, single line
- Skip files already in the conversation (check if they were previously added — can use a simple heuristic: skip if the related file path appears in any prior AddResult)

### Implementation
1. Add a `fn suggest_related_files(path: &str) -> Vec<String>` function in `commands_file.rs`
2. It checks the rules above using `std::path::Path::exists()` for test/module files
3. For `use crate::` scanning: read the first 50 lines of the file, extract `use crate::X` where X maps to `src/X.rs` or `src/X/mod.rs`
4. Cap at 3 suggestions
5. Call it from `handle_add` after processing each file, and if suggestions exist, print the hint line to stderr

### Testing
- Test that `suggest_related_files("src/foo.rs")` finds `src/foo_test.rs` when it exists (use temp dir)
- Test that module parent detection works
- Test that `use crate::` parsing finds sibling modules (mock file content)
- Test that non-source files return empty suggestions
- Test that cap of 3 works when many related files exist
