Title: Add --lite flag for small LLM usability
Files: src/cli.rs, src/cli_config.rs, src/agent_builder.rs
Issue: #415

## Description

Add a `--lite` flag that optimizes yoyo for small/local LLMs (4B-8B parameter models with 4K-16K context windows). This directly addresses Issue #415 and closes the competitive gap with Aider's small-model support.

### What to implement:

1. **In `src/cli_config.rs`:**
   - Add a `pub lite: bool` field to the `Config` struct (default false)
   - Add a `LITE_SYSTEM_PROMPT` constant — a minimal 2-line system prompt:
     ```
     "You are a coding assistant. Help the user with their code.\nYou have tools: bash (run commands), read_file, write_file, edit_file (find and replace text in files)."
     ```
   - Add a `LITE_TOOLS` constant array listing the 4 essential tools for lite mode: `["bash", "read_file", "write_file", "edit_file"]`
   - Add a `LITE_DEFAULT_CONTEXT_WINDOW: u32 = 8_000` constant

2. **In `src/cli.rs`:**
   - Parse `--lite` flag in `parse_args` (add to known flags list too)
   - When `--lite` is true:
     - Set `system_prompt` to `LITE_SYSTEM_PROMPT`
     - Set `context_window` to `Some(LITE_DEFAULT_CONTEXT_WINDOW)` (unless user also passed `--context-window` explicitly)
     - Set `disallowed_tools` to exclude everything NOT in `LITE_TOOLS` (i.e., disallow `search`, `list_files`, `rename_symbol`, `ask_user`, `todo`, `sub_agent`, `shared_state`)
   - Add `--lite` to the help text in `help_text()` / `cli_help_text()`

3. **In `src/agent_builder.rs`:**
   - In `configure_agent`, when tools are filtered by `disallowed_tools` AND `self.lite` is true, print a different message: `"🪶 Lite mode: 4 tools (bash, read_file, write_file, edit_file)"`
   - Add `pub lite: bool` to `AgentConfig` struct, plumbed from Config

### Tests to add:
- In `cli.rs`: test that `--lite` sets `lite: true`, `context_window`, system_prompt, and disallowed_tools correctly
- In `cli.rs`: test that `--lite` with explicit `--context-window 4000` respects the user's value
- In `cli_config.rs`: test that `LITE_TOOLS` and `LITE_SYSTEM_PROMPT` constants exist and are reasonable

### Docs update:
- Add `--lite` to the help text (already done via cli_help_text)
- Mention in CLAUDE.md under Architecture (brief note about lite mode in the cli_config section)
