Title: Extract /web and /copy from commands_file.rs into commands_web.rs
Files: src/commands_file.rs, src/commands_web.rs (new), src/commands.rs
Issue: none

## What to do

`commands_file.rs` is 2,449 lines and holds 5 logically distinct commands: `/add`, `/apply`,
`/copy`, `/open`, and `/web`. The `/web` (URL fetching + HTML stripping) and `/copy` (clipboard
integration) commands are not file operations — they're about web content and clipboard,
respectively. Extract them into a new `commands_web.rs` module.

### What moves to commands_web.rs

From `commands_file.rs`, move these functions:

**Web-related (for /web):**
- `strip_html_tags()` — HTML tag stripping utility
- `is_valid_url()` — URL validation
- `fetch_url()` — curl-based URL fetching (private, make pub(crate))
- `handle_web()` — the /web command handler
- All helper functions used only by these (e.g. `find_ascii_ci`, `starts_with_ascii_ci` if only used by strip_html_tags)
- The `WEB_MAX_CHARS` constant

**Clipboard-related (for /copy):**
- `clipboard_command()` — platform clipboard detection
- `command_exists()` — checks if a command is available
- `copy_to_clipboard()` — pipes text to clipboard
- `handle_copy()` — the /copy command handler
- `extract_last_assistant_text()` — extracts text from messages (used by /copy)
- `extract_last_code_block()` — extracts code blocks (used by /copy)
- The `COPY_SUBCOMMANDS` constant

**Note:** After Task 1 added URL support to `/add`, `/add` now calls `fetch_url` and
`strip_html_tags`. These functions must be `pub(crate)` in `commands_web.rs` so
`commands_file.rs` can import them. Update the imports in `commands_file.rs` accordingly.

### What stays in commands_file.rs

- `/add` handler and all its helpers (parse_add_arg, expand_add_paths, read_file_for_add, etc.)
- `/apply` handler and its helpers
- `/open` handler and its helpers
- `expand_file_mentions()`
- `build_explain_prompt()`
- Image handling functions

### Re-exports

In `commands.rs`, add re-exports for the moved public functions:
```rust
pub use crate::commands_web::{handle_web, handle_copy, strip_html_tags, is_valid_url};
pub(crate) use crate::commands_web::{extract_last_assistant_text, extract_last_code_block, fetch_url};
```

### Move tests too

Any `#[test]` functions in `commands_file.rs` that test the moved functions should also
move to `commands_web.rs`. Tests that test `/add` URL support (from Task 1) should stay
in `commands_file.rs` but may need updated imports.

### Verification

After extraction:
- `commands_file.rs` should be ~1,600 lines (down from ~2,449)
- `commands_web.rs` should be ~800 lines
- `cargo build && cargo test` must pass
- `cargo clippy --all-targets -- -D warnings` must pass

### Update CLAUDE.md

In the architecture section, add `commands_web.rs` to the module list:
`commands_web.rs` — `/web` URL fetching, `/copy` clipboard integration, HTML stripping
And update `commands_file.rs` description to remove `/web` and `/copy`.
