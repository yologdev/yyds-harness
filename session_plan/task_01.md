Title: Add /copy command for clipboard integration
Files: src/commands_file.rs, src/dispatch.rs, src/help.rs
Issue: none

Add a `/copy` slash command that copies text to the system clipboard using platform-appropriate tools:
- macOS: pipe to `pbcopy`
- Linux: try `wl-copy` (Wayland), then `xclip -selection clipboard`, then `xsel --clipboard --input`
- Windows: pipe to `clip.exe`

Subcommands:
- `/copy` (no args) — copy the last assistant message text (strip markdown formatting, just plain text)
- `/copy last` — same as no args
- `/copy code` — copy the last code block from the last assistant message
- `/copy <text>` — copy the literal text argument

Implementation:
1. In `src/commands_file.rs`, add a `pub fn handle_copy(input: &str, agent: &Agent)` function that:
   - Parses the subcommand
   - Extracts text from agent messages as needed (use `agent.messages()` to find last assistant message)
   - Detects platform via `cfg!(target_os = ...)` and picks the right clipboard command
   - Pipes the text to the clipboard command via `std::process::Command` with stdin
   - Prints a confirmation like "  ✓ Copied N chars to clipboard" or an error if clipboard tool not found

2. In `src/dispatch.rs`, add the `/copy` command routing to call `handle_copy`

3. In `src/help.rs`, add `/copy` to the command help and completions

Add a unit test that verifies:
- `extract_last_code_block` correctly finds the last ```...``` fenced code block in markdown
- `extract_last_assistant_text` finds the right message
- The clipboard command selection logic picks the right tool per platform (don't actually run clipboard commands in tests)

The function should gracefully handle: no messages yet, no code blocks found, clipboard tool not available (print helpful error suggesting install of xclip/xsel).
