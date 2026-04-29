Title: Add /skill install command for local skill installation
Files: src/commands_project.rs
Issue: none

The #1 competitive gap is the plugin/skills marketplace. Claude Code has 12+ bundled plugins and a marketplace. yoyo has `--skills <dir>` and `/skill list|show|path` but no way to install new skills. This task adds the first step: `/skill install <source>` to copy a skill into the user's skill directory.

**What to do:**

1. In `src/commands_project.rs`, extend `handle_skill` to support an `install` subcommand.

2. Implement `skill_install(source: &str)` that:
   - Accepts a local path to a skill directory (e.g., `/skill install ./my-skill/` or `/skill install /path/to/skills/my-skill`)
   - Validates the source directory exists and contains a `SKILL.md` file
   - Parses the SKILL.md frontmatter to extract the skill name
   - Copies the entire skill directory to `~/.config/yoyo/skills/<name>/`
   - Creates the destination directory if it doesn't exist
   - Prints success message with the installed skill name and path
   - If the skill already exists at the destination, warns and asks for confirmation (or skips with a message)

3. Add the `install` subcommand to `SKILL_SUBCOMMANDS` for tab completion.

4. Update the help text in the `/skill` handler to mention the install option.

5. Write tests:
   - Test that `skill_install` validates SKILL.md presence
   - Test that it rejects a nonexistent path
   - Test that it extracts the skill name from frontmatter
   - Use temp directories for all tests (no real filesystem side effects)

**Design notes:**
- For now, only support local paths. Future: support `gh:user/repo` URLs.
- The skill directory is copied, not symlinked, so it's self-contained.
- This touches only `commands_project.rs` (max 3 files rule).
- The shell subcommand `yoyo skill install <path>` should also work since `dispatch.rs` already routes `skill` subcommands.

**Verification:** `cargo build && cargo test` must pass. Manual test: create a temp skill dir with a SKILL.md, run the install function, verify it copies correctly.
