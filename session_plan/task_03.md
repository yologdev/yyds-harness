Title: Lazy-compile regexes in commands_map.rs (25 Regex::new → LazyLock)
Files: src/commands_map.rs
Issue: none

## Problem

The assessment found 25 `Regex::new(...).unwrap()` calls in `commands_map.rs`. These are
compile-time-constant patterns that get recompiled on every invocation of `extract_symbols()`,
`build_repo_map()`, etc. This is wasted work — regex compilation is expensive relative to
matching, and these functions get called on every `/map` and `/outline` invocation.

## What to do

1. At the top of `src/commands_map.rs`, add `use std::sync::LazyLock;` and `use regex::Regex;`
   (Regex is likely already imported).

2. For each `Regex::new("...").unwrap()` call, extract the pattern into a `static` using
   `LazyLock<Regex>`:

   ```rust
   static RE_FUNC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"...").unwrap());
   ```

3. Replace each call site to use `&*RE_FUNC` or `RE_FUNC.is_match(...)` etc.

4. Group related regexes together near the top of the file with descriptive names:
   - Language detection patterns (e.g., `RE_RUST_FN`, `RE_PYTHON_DEF`, etc.)
   - Symbol extraction patterns
   - Keep names descriptive of what they match

5. There are 25 regex patterns. Some may be duplicates (same pattern used in multiple
   functions). Deduplicate if found.

## Naming convention

Use `RE_` prefix + language + what it matches:
- `RE_RUST_FN` for Rust function pattern
- `RE_RUST_STRUCT` for Rust struct pattern
- `RE_PY_DEF` for Python def pattern
- etc.

## Tests

Existing tests in `commands_map.rs` should continue to pass unchanged — this is a pure
performance refactor with no behavior change. Run `cargo test` to verify.

## Verification

`cargo build && cargo test` must pass. `grep -c 'Regex::new' src/commands_map.rs` should
return 0 (or close to 0 — only the LazyLock initializers remain, which are fine since they
run once). Actually, the LazyLock closures still contain `Regex::new`, so the count stays
at 25 but they're now in static initializers instead of function bodies. The key verification
is that no `Regex::new` appears inside a non-static function body.
