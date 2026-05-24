Title: Estimated remaining turns in /tokens and /profile
Files: src/commands_info.rs, src/format/cost.rs
Issue: none

## What

Add a "remaining turns" estimate to `/tokens` and `/profile` output. When you've been in a session for a few turns, compute the average context growth per turn and project how many more turns fit before hitting the context limit. This helps users manage long sessions without surprises — you know to `/compact` or `/clear` proactively instead of being caught off guard by an overflow.

Claude Code shows context pressure warnings; we go further by predicting when you'll run out.

## Implementation

### In `src/format/cost.rs`:

1. Add function `estimate_remaining_turns(messages: &[AgentMessage], max_context: u64) -> Option<(usize, f64)>`:
   - Count assistant turns (like `extract_turn_costs` does)
   - If fewer than 2 turns, return `None` (not enough data to estimate)
   - Compute current context size via `total_tokens(messages)`
   - Compute average context growth per turn: `context_used / turn_count`
   - Compute remaining capacity: `max_context - context_used`
   - Estimate remaining turns: `remaining / avg_per_turn` (floor to usize)
   - Return `Some((remaining_turns, avg_tokens_per_turn))`
   - Note: this is a rough estimate — early turns are cheaper (less context to repeat), later turns are more expensive. But even a rough estimate is useful.

2. Add function `format_remaining_turns(remaining: usize, avg_per_turn: f64) -> String`:
   - Format like: `~12 turns remaining (~4.2k tokens/turn avg)`
   - If remaining <= 3, use YELLOW color for warning
   - If remaining == 0, use RED with "context nearly full"

### In `src/commands_info.rs`:

3. In `handle_tokens()`, after the context bar, add:
   ```rust
   if let Some((remaining, avg)) = estimate_remaining_turns(&messages, max_context) {
       println!("    {}", format_remaining_turns(remaining, avg));
   }
   ```

4. In `handle_profile()`, add a "Remaining" line to the profile box when data is available:
   - After the "Context" line, add estimated remaining turns
   - Keep the box formatting consistent

### Tests

Add tests in `src/format/cost.rs`:
- `test_estimate_remaining_turns_empty` — no messages → None
- `test_estimate_remaining_turns_one_turn` — 1 turn → None (not enough data)
- `test_estimate_remaining_turns_basic` — 3 turns with known usage → correct estimate
- `test_format_remaining_turns_normal` — normal case
- `test_format_remaining_turns_low` — ≤3 remaining → contains warning color

No docs changes needed.
