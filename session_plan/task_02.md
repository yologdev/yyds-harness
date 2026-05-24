Title: /git stage — interactive file staging from the REPL
Files: src/git.rs, src/commands_git.rs
Issue: none

## What

Add a `/git stage` subcommand that shows all modified/untracked files with numbered indices and lets the user pick which to stage by number, range, or glob — without leaving the REPL.

## Why

Claude Code and Cursor both handle staging smoothly. Right now yoyo has `/git add <path>` but no interactive picker. Real developers often want to selectively stage files without typing each path. This is a common workflow gap.

## How

1. **In `src/git.rs`:**
   - Add `GitSubcommand::Stage` variant to the enum
   - In `parse_git_args`, map `"stage"` → `GitSubcommand::Stage`
   - In `run_git_subcommand`, implement the `Stage` handler:
     a. Run `git status --porcelain` to get all changed files
     b. Parse each line into (status_code, filename)
     c. Display numbered list with colored status indicators:
        ```
        Modified files:
          1. [M] src/main.rs
          2. [M] src/tools.rs
          3. [?] new_file.rs
          4. [D] old_file.rs
        ```
     d. Prompt: `Stage which? (1,3 / 1-3 / *.rs / all / q): `
     e. Parse input:
        - Single numbers: `1`, `3`
        - Ranges: `1-3`
        - Comma-separated: `1,3,5`
        - Glob: `*.rs`, `src/*`
        - `all` or `a`: stage everything
        - `q` or empty: cancel
     f. Run `git add` for each selected file
     g. Show confirmation: `✓ staged 3 files`

2. **In `src/commands_git.rs`:**
   - Add `"stage"` to the `GIT_SUBCOMMANDS` list in `src/commands.rs` if it exists there, so tab completion works
   - Update the `/git` help text to include `stage`

3. **Tests in `src/git.rs`:**
   - Test `parse_git_args("stage")` returns `GitSubcommand::Stage`
   - Test the parsing functions for selection input (numbers, ranges, globs)
   - Extract the selection parsing into a pure testable function: `fn parse_stage_selection(input: &str, file_count: usize) -> Vec<usize>`

## Size estimate

~120 lines of new code. The parsing logic is the core — the git operations are just `run_git(&["add", path])` calls.
