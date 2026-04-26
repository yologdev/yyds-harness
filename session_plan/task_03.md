Title: /outline — lightweight symbol search across the codebase
Files: src/commands_search.rs, src/commands.rs, src/dispatch.rs
Issue: none

## What to do

Add a `/outline` command that searches for symbols (functions, structs, enums, traits, types) across the project, showing matches with their file locations. This is the CLI equivalent of VS Code's "Go to Symbol in Workspace" (Ctrl+T). It reuses the existing `extract_symbols()` infrastructure from `commands_map.rs` but provides a search-first interface rather than a full dump.

### Implementation

1. **In `src/commands_search.rs`**: Add a new `handle_outline(input: &str)` function:
   - Parse the argument: `/outline <query>` where query is a fuzzy or substring match
   - If no query, show usage hint
   - Use `crate::commands_map::build_repo_map(MapBackend::Regex)` (or the faster regex backend) to get all symbols
   - Filter symbols by fuzzy/substring match against the query
   - Display results in a compact, colored format:
     ```
     fn parse_args        src/cli.rs:142
     fn parse_config      src/config.rs:89
     struct Config        src/cli.rs:56
     ```
   - Limit to 30 results by default, with `/outline <query> --all` to show all
   - Sort by relevance (exact prefix match > contains > fuzzy)

2. **In `src/commands.rs`**: Add `"outline"` to `KNOWN_COMMANDS`. Add completion hints.

3. **In `src/dispatch.rs`**: Add the dispatch arm for `/outline`:
   ```rust
   s if s == "/outline" || s.starts_with("/outline ") => {
       commands::handle_outline(input);
       CommandResult::Continue
   }
   ```

### Interface examples

```
/outline parse
  fn parse_args              src/cli.rs:142
  fn parse_config_file       src/cli.rs:658
  fn parse_thinking_level    src/cli.rs:412
  fn parse_blame_args        src/commands_git.rs:891
  ... (26 more — use /outline parse --all)

/outline Config
  struct Config              src/cli.rs:56
  struct PermissionConfig    src/config.rs:12
  struct McpServerConfig     src/config.rs:204

/outline
  Usage: /outline <query> [--all]
  Search for functions, structs, enums, and traits across the project.
```

### Testing

- Test that `handle_outline` with a known symbol name (e.g., in a temp directory with a test file) returns matching results
- Test that empty query shows usage
- Test that `--all` flag is parsed correctly
- Test result limiting (>30 results triggers truncation message)

### Why this matters

This gives yoyo a fast "go to symbol" capability that's one of the most-used features in IDEs. Real developers spend significant time finding where things are defined. `/map` shows everything; `/outline` answers "where is this specific thing?" It's the navigation primitive that makes yoyo usable on large codebases without leaving the terminal.

### Docs update

Add `/outline` to the commands reference in `docs/src/usage/commands.md` and ensure `/help outline` returns useful text (add to `help.rs` command_help function).
