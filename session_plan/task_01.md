Title: Extract RTK integration from tools.rs into src/rtk.rs
Files: src/tools.rs, src/rtk.rs
Issue: none

## What to do

The RTK (Rust Token Killer) integration in `tools.rs` is a self-contained concern mixed in with tool definitions. Extract it into its own module `src/rtk.rs`.

### Code to extract (from tools.rs)

Move the following to `src/rtk.rs`:
1. The three statics: `RTK_DISABLED`, `RTK_AVAILABLE`, `RTK_ANNOUNCED` (lines ~42-49)
2. `pub fn disable_rtk()` 
3. `pub fn is_rtk_disabled() -> bool`
4. `pub fn detect_rtk() -> bool`
5. `const RTK_SUPPORTED_COMMANDS` array
6. `fn is_simple_command(command: &str) -> bool` (helper used by `maybe_prefix_rtk`)
7. `pub fn maybe_prefix_rtk(command: &str) -> String`
8. All RTK-related tests (search for `test_rtk`, `test_maybe_prefix_rtk`, `test_detect_rtk`, `test_is_simple_command`, the section marked `// --- RTK integration tests ---`)

### New file structure

`src/rtk.rs`:
- Add appropriate module doc comment (`//! RTK (Rust Token Killer) integration`)
- Bring over needed imports (`std::sync::atomic`, `std::sync::OnceLock`, `std::process::Command`)
- Keep all functions `pub` as they are
- Move tests into `#[cfg(test)] mod tests { ... }` within the new file

### In tools.rs

- Remove all RTK code and tests
- Add `use crate::rtk::{disable_rtk, is_rtk_disabled, detect_rtk, maybe_prefix_rtk};` (or whatever is needed by the remaining code — `StreamingBashTool` calls `maybe_prefix_rtk`)
- Keep the `use crate::format::*;` and other imports that tools.rs still needs

### In main.rs (or wherever disable_rtk/is_rtk_disabled/detect_rtk are called)

- Update `use` statements from `crate::tools::` to `crate::rtk::`
- Search for all call sites: `grep -rn "disable_rtk\|is_rtk_disabled\|detect_rtk\|maybe_prefix_rtk" src/`

### Don't forget

- Add `mod rtk;` to `main.rs`
- Run `cargo build && cargo test` to verify
- Run `cargo clippy --all-targets -- -D warnings` to verify

### Update CLAUDE.md

Add `rtk.rs` to the module list:
```
- `rtk.rs` — RTK (Rust Token Killer) detection, proxy integration, output compression
```
