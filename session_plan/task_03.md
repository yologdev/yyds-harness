Title: /web search — user-facing web search slash command + help text
Files: src/commands_web.rs, src/commands.rs, src/help_data.rs
Issue: none

## What

Add `/web search <query>` as a user-facing slash command so users can search the web interactively from the REPL. Update command completions and help text.

## Why

Task 1 builds the search engine, Task 2 makes it available to the agent. This task completes the surface area by making it available to the human user too. The `/web` command currently only fetches URLs — adding search makes it a full web access tool.

## Implementation

### 1. Update `handle_web` in `commands_web.rs`

Currently `handle_web` only handles `/web <url>`. Modify it to detect when the input is a search query vs a URL:

```rust
pub fn handle_web(input: &str) {
    let arg = input.trim_start_matches("/web").trim();
    
    if arg.is_empty() {
        // Show updated usage
        println!("  usage: /web <url>        — fetch a web page");
        println!("         /web search <query> — search the web");
        return;
    }
    
    if arg.starts_with("search ") || arg.starts_with("search\t") {
        let query = arg.strip_prefix("search").unwrap().trim();
        handle_web_search(query);
        return;
    }
    
    // Existing URL fetch logic...
}
```

### 2. Add `handle_web_search` function in `commands_web.rs`

```rust
fn handle_web_search(query: &str) {
    if query.is_empty() {
        println!("  usage: /web search <query>");
        return;
    }
    
    println!("  Searching for: {query}...");
    
    let results = web_search(query, 8);  // from Task 1
    match results {
        Ok(results) if results.is_empty() => {
            println!("  (no results found)");
        }
        Ok(results) => {
            // Display results with color formatting
            for (i, r) in results.iter().enumerate() {
                println!("  {}. {BOLD}{}{RESET}", i + 1, r.title);
                println!("     {CYAN}{}{RESET}", r.url);
                if !r.snippet.is_empty() {
                    println!("     {DIM}{}{RESET}", r.snippet);
                }
                println!();
            }
            println!("  {DIM}Tip: use /web <url> to read any result{RESET}");
        }
        Err(e) => {
            println!("  {RED}Search failed: {e}{RESET}");
        }
    }
}
```

### 3. Update command completions in `commands.rs`

Add `"search"` as a subcommand/completion for `/web`:
- In `command_arg_hint`, add a case for `"web"` returning `"<url> | search <query>"`
- In `command_arg_completions`, add a case for `"web"` returning `&["search"]`

### 4. Update help text in `help_data.rs`

Update the `/web` help entry to document both URL fetching and search:
- Short description: `"Fetch a web page or search the web"`
- Detailed help should show both usages with examples

### 5. Tests

- Test `handle_web` routing: `/web search rust async` routes to search, `/web https://example.com` routes to fetch
- Test `handle_web_search` with empty query shows usage
- Test that command completions for "web" include "search"

### Notes
- This task depends on Task 1 for the `web_search` function
- Keep the URL detection simple: if the arg starts with "search " (followed by space), it's a search. Everything else is treated as a URL (existing behavior preserved).
- The existing URL fetch path must not be broken — this is additive only
