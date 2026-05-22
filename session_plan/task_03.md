Title: Show token estimate when adding files to context via /add
Files: src/commands_file.rs
Issue: none

When users add files to the conversation with `/add`, they see the file name and line count but not how much of their context window it consumes. This makes context management opaque — you don't know you've blown your budget until compaction kicks in. Claude Code's UI shows token usage clearly.

**What to build:**

After each successful `/add`, show an approximate token count for the added content alongside the existing line count. The output should look like:

```
  ✓ src/main.rs (145 lines, ~580 tokens)
```

instead of the current:

```
  ✓ src/main.rs (145 lines)
```

**Implementation approach:**

1. Add a simple token estimation function `estimate_tokens_simple(text: &str) -> usize` in `src/commands_file.rs`:
   - Use the standard approximation: `text.len() / 4` (chars ÷ 4 is the widely-used rough estimate for English/code)
   - This is intentionally simple — no tokenizer dependency needed
   - Return value is approximate, hence the `~` prefix in display

2. In `handle_add()`, where the success message is printed (the `println!` with "✓" and line count):
   - Calculate `estimate_tokens_simple(&content)` 
   - Append `, ~{tokens} tokens` to the existing output format
   - For images, skip the token estimate (or show the image token formula if known)

3. For URL fetches (web content), also show the token estimate after stripping HTML.

4. For truncated files, show the token estimate of the *truncated* content (what's actually being injected), not the full file.

**Tests:**
- Test `estimate_tokens_simple` with empty string → 0
- Test `estimate_tokens_simple` with known text → approximately correct (within 20%)
- Test `estimate_tokens_simple` with code (more tokens per char) → reasonable estimate
- Test that the function doesn't panic on very large inputs

**Constraints:**
- The estimation must be fast (no external tokenizer, no API calls)
- Use `~` prefix to signal it's approximate
- Don't change the format when `is_quiet()` is true
- Keep the function simple — this is UX polish, not precision engineering
