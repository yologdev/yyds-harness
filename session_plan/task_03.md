Title: Extract watch module from prompt.rs
Files: src/prompt.rs, src/watch.rs (new)
Issue: none

## Context

The assessment identifies `prompt.rs` (2,539 lines) as having mixed concerns: watch mode,
retry logic, message search, output writing, and streaming. The watch-mode functions are
a cohesive group that can be their own module. This is the most natural extraction —
the functions have clear boundaries and minimal coupling to the rest of prompt.rs.

## What to do

### 1. Create `src/watch.rs`

Extract these functions and their associated items from `prompt.rs`:

- `set_watch_command` (line ~30)
- `get_watch_command` (line ~36)
- `clear_watch_command` (line ~42)
- `MAX_WATCH_FIX_ATTEMPTS` constant (line ~44 area)
- `build_watch_fix_prompt` (line ~54)
- `run_watch_command` (line ~74)
- `run_watch_after_prompt` (line ~160) — this is the big async function

Also extract the corresponding tests:
- `test_build_watch_fix_prompt`
- `test_max_watch_fix_attempts_constant`
- `test_build_watch_fix_prompt_truncates_long_output`
- `test_run_watch_command_success`

### 2. Handle dependencies

The watch functions use:
- `std::sync::Mutex` / `OnceLock` for the global watch command state
- `std::process::Command` for running shell commands
- Agent types from yoagent for `run_watch_after_prompt`
- Functions from other yoyo modules (check imports carefully)

`run_watch_after_prompt` is the most complex — it takes agent references, calls
prompt functions, etc. If the coupling is too tight, it's acceptable to leave
`run_watch_after_prompt` in `prompt.rs` and only extract the simpler functions
(set/get/clear/build/run). The goal is a clean extraction, not a forced one.

### 3. Update `prompt.rs`

- Remove the extracted functions and tests
- Add `pub(crate) use watch::*;` or specific imports as needed
- Ensure all existing callers still compile

### 4. Add `mod watch;` to `main.rs`

Register the new module in the crate root.

### 5. Verify

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

## Important notes

- This is a pure refactor — zero behavior changes
- Do NOT modify any function signatures (callers shouldn't need updates beyond import paths)
- If `run_watch_after_prompt` is too entangled, leave it in `prompt.rs` — partial extraction
  is fine. The simpler watch functions alone are ~130 lines of clean extraction.
- Check that all functions use `pub(crate)` visibility (they should already)
- Don't forget to move the tests too — keep test coverage identical
