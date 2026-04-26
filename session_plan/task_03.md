Title: Add --quiet / -q flag to suppress informational stderr output for scripted usage
Files: src/cli.rs, src/format/mod.rs, src/context.rs
Issue: none

## Problem

The assessment found that piped/scripted usage of yoyo produces noise on stderr:
- `config: loaded .yoyo.toml`
- `context: project file listing`
- `context: recently changed files`
- `context: git status (branch: main)`
- `yoyo (piped mode) — model: ...`
- Spinner artifacts (addressed by task_01)

For scripting use cases (`echo "fix the test" | yoyo > result.md`), this informational
output is pure noise. A `--quiet`/`-q` flag would suppress it, making yoyo a better
building block in shell pipelines.

## Implementation

1. In `src/format/mod.rs`, add a global quiet-mode flag similar to the existing
   `disable_color()`/`Color` pattern:
   ```rust
   static QUIET: AtomicBool = AtomicBool::new(false);
   
   pub fn enable_quiet() {
       QUIET.store(true, Ordering::Relaxed);
   }
   
   pub fn is_quiet() -> bool {
       QUIET.load(Ordering::Relaxed)
   }
   ```

2. In `src/cli.rs`, add `--quiet` / `-q` flag parsing in `parse_args()`:
   - Add `quiet: bool` field to `Config`
   - Parse `-q` and `--quiet` in the flag matching
   - Add to help text
   - Also auto-enable quiet when stdout is not a terminal AND stdin is not a terminal
     (fully piped mode) — this is the common scripting case

3. In `src/context.rs`, gate all the `eprintln!("{DIM}  context: ...{RESET}")` lines
   on `!is_quiet()`. There are ~5 such lines in `load_project_context()`.

4. In `src/cli.rs`, gate the `eprintln!("{DIM}  config: ...{RESET}")` lines on
   `!is_quiet()`. There are ~3 such lines in `load_config_file()`.

5. Call `enable_quiet()` early in `main()` (in main.rs) when the flag is set, right
   after color disabling. NOTE: Do NOT modify main.rs — the quiet flag is stored in
   Config, and the format module's `enable_quiet()` can be called from cli.rs's
   `parse_args()` or from a dedicated setup section. The most natural place is in
   `parse_args()` when `-q`/`--quiet` is encountered, similar to how `--no-color`
   calls `disable_color()`.

## Important

This task modifies only 3 files: `cli.rs`, `format/mod.rs`, and `context.rs`. The
`enable_quiet()` call in main.rs can be handled by having `parse_args()` set the global
directly when it encounters the flag (like `disable_color()` is called), or by having
`main()` check `config.quiet` — but prefer the parse_args approach to stay within the
3-file limit.

## Tests

- Test that `is_quiet()` defaults to false
- Test that `enable_quiet()` / `is_quiet()` round-trips
- Test that `--quiet` flag is parsed in Config
- Test that `-q` flag is parsed in Config

## Verification

- `cargo build && cargo test`
- Manual: `echo "hello" | cargo run -- -q 2>&1 | grep -c "context:"` should return 0
- Manual: `echo "hello" | cargo run 2>&1 | grep -c "context:"` should return >0
