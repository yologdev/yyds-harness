Title: Show cache hit rate in /cost and /tokens displays
Files: src/commands_info.rs, src/format/cost.rs
Issue: none

## What

With prompt caching being enabled (Task 1), users should be able to see how much caching
is saving them. Currently, `/cost` and `/tokens` show input/output token counts and cost,
and the cost breakdown includes cache_read/cache_write costs, but there's no visible
"cache hit rate" metric or savings summary that tells the user "caching saved you X%."

yoagent's `Usage` type already has a `cache_hit_rate()` method that returns 0.0–1.0.
Wire it into the user-facing displays.

## Implementation

### In `src/format/cost.rs`

1. Add a `fn format_cache_stats(usage: &yoagent::Usage) -> Option<String>` function that:
   - Returns `None` if `usage.cache_read == 0 && usage.cache_write == 0` (no caching activity)
   - Otherwise returns a string like `"Cache: 85% hit rate (150.2k read, 12.0k written)"`
   - Uses `usage.cache_hit_rate()` for the percentage
   - Uses `format_token_count()` for the counts

2. Add tests for `format_cache_stats`:
   - Returns None when no cache activity
   - Returns correct string with cache data
   - Handles edge cases (all cache_read, all cache_write, mixed)

### In `src/commands_info.rs`

1. In `handle_cost()` — after printing the cost breakdown, if cache stats are available,
   print them on a new line. Look for where the cost output is assembled and add:
   ```rust
   if let Some(cache_line) = format_cache_stats(&usage) {
       eprintln!("  {}", cache_line);
   }
   ```

2. In `handle_tokens()` — similar addition showing cache hit rate alongside token counts.

## Why

- Completes the prompt caching story (Task 1 enables it, this task makes it visible)
- Users need feedback that caching is working and saving money
- Uses existing yoagent API (`cache_hit_rate()`) — no reinvention
- Small, focused change that makes cost awareness more actionable
- The data is already being tracked (cache_read/cache_write in Usage) — just not surfaced

## Verification

- `cargo build && cargo test`
- After Task 1, multi-turn conversations should show non-zero cache hit rates in `/cost`
