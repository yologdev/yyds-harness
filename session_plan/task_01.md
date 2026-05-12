Title: Strengthen auto-continue heuristic for incomplete responses
Files: src/repl.rs
Issue: #389

## Context

Issue #389 reports that yoyo stops mid-task requiring manual "continue" prompts during multi-step work. The current `looks_incomplete` heuristic in `repl.rs` catches some continuation signals but misses many common patterns. MAX_AUTO_CONTINUES is 3, which is too low for 12-step plans.

## What to do

1. **Raise MAX_AUTO_CONTINUES from 3 to 5.** The current limit of 3 isn't enough for plans with 8-12 steps where the model stops at semantic boundaries. 5 gives more room without being unbounded.

2. **Expand the `looks_incomplete` heuristic** with additional patterns:

   - **Pattern 5: Unclosed code blocks** — if the response ends with a code block that was opened (``` count is odd), the model was cut off mid-code.
   
   - **Pattern 6: "Let me update/fix/modify" phrases** — add more continuation phrases:
     - "let me update"
     - "let me fix"  
     - "let me modify"
     - "let me add"
     - "let me create"
     - "let me write"
     - "let me implement"
     - "let me handle"
     - "i'll update"
     - "i'll fix"
     - "i'll modify"
     - "i'll add"
     - "i'll create"
     - "i'll implement"
     - "i'll handle"
     
   - **Pattern 7: "First/Second/Third... next" progression** — when the tail contains ordinal language suggesting more steps ("first" without "finally", or "second" without "third").

   - **Pattern 8: Explicit "step X of Y" where X < Y** — e.g., "That's step 2 of 5 done" or "Step 3/7 complete"

3. **Add tests** for each new pattern. The existing test structure in `repl.rs` (search for `test_looks_incomplete_`) provides the template. Add:
   - `test_looks_incomplete_unclosed_code_block`
   - `test_looks_incomplete_action_phrases`
   - `test_looks_incomplete_step_x_of_y`
   - Negative tests for completed work that shouldn't trigger

4. **Update the auto-continue message** to show the new limit (already dynamic via the constant).

## Verification

- `cargo build && cargo test`
- All existing `looks_incomplete` tests still pass
- New tests cover new patterns
- The constant is 5, not 3
