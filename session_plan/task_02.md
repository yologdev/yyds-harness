Title: Add /help search <keyword> for command discovery by keyword
Files: src/help.rs, src/help_data.rs
Issue: none

## Problem
Currently `/help` lists all commands grouped by category, and `/help <command>` shows help for a specific command. But if a user wants to find commands related to "git" or "review" or "testing", they have to read through the entire help output. This is the second layer of the discoverability gap: even when users know to ask for help, they can't search efficiently.

## What to Build
Add a `/help search <keyword>` sub-command that searches all commands by name, short description, and detailed help text, returning matching commands with relevance ranking.

### Behavior
```
/help search git
  Commands matching "git":
    /commit      Commit staged or all changes (git commit)
    /diff        Show file diffs with color (git diff)  
    /pr          Pull request management (create, list, view, review)
    /git         Run arbitrary git commands
    /undo        Undo last file change or git operation
    /blame       Show git blame with syntax highlighting

/help search test
  Commands matching "test":
    /test        Run project tests (auto-detects test runner)
    /watch       Set/run watch command (auto-test after prompts)
    /health      Run project health checks
```

### Implementation
1. In `help.rs`, add a `handle_help_search(keyword: &str)` function
2. It should search across:
   - Command names (from `command_short_description` in `help_data.rs`)
   - Short descriptions (from `command_short_description`)
   - Detailed help text (from `command_help`)
3. Scoring: exact name match > name contains > description contains > detail text contains
4. Display results sorted by relevance, showing command name + short description
5. Case-insensitive matching
6. If no results: "No commands matching '<keyword>'. Try /help for the full list."

### Integration
- In `handle_help` in `help.rs`, detect when the argument starts with "search " and route to `handle_help_search`
- Add "search" to `help_command_completions` for tab completion
- Add a line in the `/help` output: "  /help search <keyword>  Search commands by keyword"

### Testing
- Test that searching "git" returns git-related commands
- Test that searching "test" returns test-related commands
- Test case-insensitive matching
- Test no-results case
- Test that scoring works (name matches rank higher than description matches)
- Test empty/whitespace keyword returns appropriate message
