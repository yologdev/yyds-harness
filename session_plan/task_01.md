Title: Auto-continue for incomplete model responses
Files: src/repl.rs, src/prompt.rs
Issue: none (addresses @danstis feedback in Discussion #378)

## Problem

Users report (Discussion #378) that the agent stops after 10-14 turns even though
the turn limit is 200. The model decides it's "done" via StopReason::Stop, but it
was clearly in the middle of multi-step work. The user has to manually type
"continue" to keep it going. This is a major UX friction point.

## Solution

Add auto-continue detection in `handle_post_prompt` in `src/repl.rs`. After the
model finishes a prompt (PromptOutcome is returned), check if the response text
suggests incomplete work using heuristic pattern matching. If so, automatically
send a "Continue with the remaining work" follow-up prompt.

### Implementation Details

1. **Add a function `looks_incomplete(text: &str) -> bool`** in `src/repl.rs` that
   checks for patterns indicating the model stopped mid-work:
   - Text ends with "Next, I'll..." / "I'll now..." / "Let me continue..." / "Moving on to..."
   - Text contains numbered steps where later steps haven't been addressed
     (e.g., mentions "Step 3:" but the text ends without mentioning "Step 4:" when 4+ were listed)
   - Text contains "remaining" + "steps/tasks/items" near the end
   - Text ends with a colon or "..." suggesting continuation
   - Text explicitly says "I'll continue" or "Let me proceed"

2. **Add auto-continue logic in `handle_post_prompt`** in `src/repl.rs`:
   - Track a counter `auto_continue_count` (add to `PostPromptContext` or track via
     a static/passed-in counter)
   - After normal post-prompt processing, if `looks_incomplete(&outcome.text)` is true
     AND `auto_continue_count < 3` (max 3 auto-continues per user prompt):
     - Print a dim message: "  ⚡ auto-continuing (response appears incomplete)..."
     - Send "Continue with the remaining work. Pick up where you left off." as a new prompt
     - Increment the counter
   - Reset the counter at the start of each user prompt (in the main REPL loop)
   - Do NOT auto-continue if the outcome had errors (last_tool_error or last_api_error)

3. **Add `auto_continue_count` field to PostPromptContext** or pass it as a mutable
   reference. The counter resets each time the user enters a new prompt.

4. **Tests**: Add unit tests for `looks_incomplete()` with various patterns:
   - Positive: "Next, I'll fix the remaining tests", "...moving on to step 3"
   - Negative: "All done! The tests pass.", "I've completed all the changes."
   - Edge cases: empty string, very short responses

### Opt-out

The auto-continue should be on by default (it's what users want). If needed,
a config key `auto_continue = false` can be added later, but for now keep it
simple — just implement the detection and auto-continue logic.

### Safety

- Max 3 auto-continues prevents infinite loops
- Don't auto-continue on errors
- Don't auto-continue if session budget is exhausted
- The heuristic should be conservative — only trigger on clear signals of incompleteness
