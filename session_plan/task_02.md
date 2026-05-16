Title: Suppress chrome in --print mode — print_usage and print_context_usage leak through quiet
Files: src/format/mod.rs
Issue: none

## What to do

Fix the self-discovered bug from the assessment: `--print` mode still shows cost/context usage lines. The `--print` flag enables quiet mode via `enable_quiet()`, but `print_usage()` and `print_context_usage()` don't check `is_quiet()` before emitting output.

### Current behavior
Running `echo "hello" | cargo run -- --print` outputs the assistant response BUT ALSO shows:
- `↳ 1.2s · 500→42 tokens` (from `print_usage`)
- `⬤ 2% of context window used` (from `print_context_usage`)

These lines break programmatic use where `--print` is supposed to give raw output only.

### Fix

In `src/format/mod.rs`:

1. **`print_usage`** — add `if is_quiet() { return; }` at the top of the function.
2. **`print_context_usage`** — add `if is_quiet() { return; }` at the top of the function.

That's it — two one-line guards.

### Tests
Add tests:
- `test_print_usage_quiet_suppressed` — enable_quiet(), call print_usage with non-zero usage, verify no output (capture stdout or just verify the function returns early without panicking).
- `test_print_context_usage_quiet_suppressed` — enable_quiet(), call print_context_usage, verify no output.

Actually, since these functions print to stdout and tests run in parallel, the simplest test approach is to verify the guard logic exists. We can check that `is_quiet()` is consulted. Or better: add a `should_print_usage()` helper that returns `!is_quiet() && usage has data` and test that.

Alternatively, the cleanest approach: make `print_usage` and `print_context_usage` early-return when `is_quiet()`, and add a simple unit test that `is_quiet()` after `enable_quiet()` returns true (this test already exists per the grep — `test_is_quiet_returns_bool`). The behavioral correctness follows from the guard + existing test.

### Verify
`cargo build && cargo test`
After: `echo "test" | cargo run -- --print` should show only the model's response text, no cost/context lines.
