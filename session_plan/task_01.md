Title: Add URL support to /add — fetch web content into conversation context
Files: src/commands_file.rs
Issue: none

## What to do

Extend the `/add` command to accept URLs. When a user types `/add https://docs.rs/some-crate`,
yoyo should fetch the URL content, strip HTML tags, and inject it into the conversation context
the same way file content gets injected. This bridges a real competitive gap — Claude Code and
Gemini CLI can reference web content in conversations; yoyo currently requires users to
separately `/web` a URL and then manually copy-paste.

### Implementation

In `handle_add()` in `commands_file.rs`, before the file-path processing loop, detect if
an argument looks like a URL (starts with `http://` or `https://`, or use the existing
`is_valid_url()` function). When a URL is detected:

1. Call the existing `fetch_url()` function to download the content
2. Call the existing `strip_html_tags()` to extract readable text
3. Apply `smart_truncate_for_context()` to keep it within token limits
4. Create an `AddResult` with the URL as the "path" and the text content
5. Display the same feedback format as file adds: `✓ <url> (N lines, M chars)`

The URL processing should happen inside the `for arg in args.split_whitespace()` loop,
as a new branch before the file-path expansion. Also update the usage help text to show
URL support: `/add <path|url>`.

### Tests

Add tests for URL detection in the add path:
- Test that `http://` and `https://` prefixes are detected as URLs
- Test that regular file paths are NOT treated as URLs
- Test that `@http://example.com` file mentions work too (in `expand_file_mentions`)

### Update help text

In `help.rs`, update the `/add` help entry to mention URL support.
Update the usage line in `handle_add` itself.
