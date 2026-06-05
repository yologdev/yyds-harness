Title: Web search engine — DuckDuckGo HTML search + result parsing in commands_web.rs
Files: src/commands_web.rs
Issue: none

## What

Add web search capability to `commands_web.rs` by implementing DuckDuckGo HTML search. This is the foundation layer that both the agent tool (Task 2) and slash command (Task 3) will use.

## Why

The assessment identifies "no built-in web search tool" as the #1 actionable competitive gap. Every competitor (Claude Code, Cursor, Gemini CLI, Aider) has web search. We have `curl` and `/web <url>` for fetching pages, but no way to **search** the web. This task builds the search engine functions.

## Implementation

### 1. Add a `SearchResult` struct

```rust
pub(crate) struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}
```

### 2. Implement `web_search(query: &str, max_results: usize) -> Result<Vec<WebSearchResult>, String>`

Use DuckDuckGo's HTML search endpoint (no API key needed):
- URL: `https://html.duckduckgo.com/html/?q={query}` (URL-encode the query)
- Use `curl` via `std::process::Command` (same pattern as existing `fetch_url`)
- User-Agent: `"Mozilla/5.0 (compatible; yoyo-agent/0.1)"`
- Parse the HTML response to extract search results:
  - Results are in `<div class="result">` or `<div class="web-result">` blocks
  - Title is in `<a class="result__a">` tags
  - URL is in the `href` attribute (DuckDuckGo wraps URLs in redirects — extract the actual URL from the `uddg=` query parameter)
  - Snippet is in `<a class="result__snippet">` or `<td class="result__snippet">` tags
- Use simple string parsing (find_ascii_ci, indexOf patterns) — no HTML parser dependency needed. The existing `strip_html_tags` and `find_ascii_ci` functions can be reused.
- Default `max_results` to 8. Cap at 20.

### 3. Implement `format_search_results(results: &[WebSearchResult]) -> String`

Format results as a clean text block suitable for both human display and agent consumption:
```
1. Title of Result
   https://example.com/page
   Snippet text describing the result...

2. Another Result
   https://example.com/other
   More snippet text...
```

### 4. Add `web_search_and_read(query: &str, max_results: usize) -> String`

Convenience function that runs the search and formats results. Returns an error message string on failure (not Result — for tool consumption).

### 5. Tests

Write tests for:
- `WebSearchResult` struct construction
- HTML parsing of DuckDuckGo-style result blocks (use static HTML fixture strings)
- URL extraction from DuckDuckGo redirect URLs
- `format_search_results` output formatting
- Empty results handling
- Query URL encoding

**Important:** Tests should use mock HTML, not live network calls. Create realistic DuckDuckGo HTML fixtures based on the known structure.

### Notes
- Keep all search functions `pub(crate)` — they'll be consumed by Task 2 (agent tool) and Task 3 (slash command)
- The DuckDuckGo HTML endpoint is stable, free, and requires no API key
- Don't add any new dependencies — use `curl` via Command like existing `fetch_url`
