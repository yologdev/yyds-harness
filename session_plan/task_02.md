Title: Add /open command to launch $EDITOR on files
Files: src/commands_file.rs, src/dispatch.rs, src/help.rs
Issue: none

Add a `/open` slash command that opens a file in the user's preferred editor, optionally at a specific line number.

Syntax:
- `/open <file>` — open file in $VISUAL or $EDITOR or fallback (vi/nano/code)
- `/open <file>:<line>` — open file at specific line (uses `+N` syntax for vi/emacs/nano/code)
- `/open <file> <line>` — alternative syntax for line number

Implementation:
1. In `src/commands_file.rs`, add `pub fn handle_open(input: &str)` that:
   - Parses the file path and optional line number from input
   - Resolves editor: check `$VISUAL`, then `$EDITOR`, then try `code`, `vim`, `vi`, `nano` in order (whichever is found in PATH via `which`)
   - If a line number is provided, format the editor invocation with `+N` (works for vim, nano, code, emacs) — put `+N` before the filename
   - Spawn the editor process (NOT in background for terminal editors; use `.status()` not `.spawn()` so yoyo waits)
   - Print a brief "Opening <file> in <editor>..." message before launching
   - Handle errors: file doesn't exist (warn but still try — editor might create it), no editor found (print helpful message)

2. In `src/dispatch.rs`, add `/open` command routing

3. In `src/help.rs`, add `/open` to command help text and completions

Add tests:
- Test parsing of `file:line` syntax (both `path:42` and `path 42` forms)
- Test editor resolution logic (with mocked env vars)
- Test that the `+N` argument is formatted correctly

Note: `/open` should also support tab-completion of file paths (already handled by the general file-path completion in repl.rs for any word that looks like a path).
