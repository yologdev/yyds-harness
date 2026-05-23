Title: Contextual command hints after prompt turns (discoverability)
Files: src/repl.rs, src/format/mod.rs
Issue: none

## Problem
The assessment identifies discoverability as yoyo's biggest practical gap: "I have 50+ slash commands, many features, but users don't know they exist. The `/tips` command helps but isn't enough. Better onboarding, contextual hints, and documentation are the real gaps."

Users don't know about features because they only discover them by reading `/help` or stumbling on them. Modern tools teach you as you go — showing relevant suggestions at the moment they'd be useful.

## What to Build
Add a contextual hint system that shows a dim, one-line suggestion after certain prompt turns based on what just happened during the turn. The hint should appear after the turn-change summary line (the `format_turn_changes` output), at most once per turn, and only when relevant.

### Hint Rules (in `repl.rs` or a new small function in `format/mod.rs`):

Create a function `contextual_hint(outcome: &PromptOutcome, session_changes: &SessionChanges, turn_count: usize) -> Option<String>` that returns a hint based on signals:

1. **First turn ever (turn_count == 1):** "💡 Type /help to see available commands"
2. **Files were modified and no watch command set:** "💡 /watch to auto-test after every prompt"
3. **Tool errors occurred (outcome.last_tool_error is Some):** "💡 /retry to re-run with the error context"
4. **Many tokens used (session_total > 50% of context):** "💡 /compact to free context space"
5. **Files modified and no git commit:** "💡 /diff to review changes, /commit to save"
6. **3+ turns with no slash command used:** "💡 Try /tips to discover features"

Rules:
- Only show ONE hint per turn (pick the first matching rule)
- Show in `DIM` color so it doesn't distract
- Don't repeat the same hint category within a session (track shown categories in a `HashSet<&str>` or similar)
- Don't show hints in quiet mode or piped mode
- Prefix with 💡 for visual recognition

### Integration Point
In `handle_post_prompt` in `repl.rs`, after the turn-change summary and before the watch/auto-commit logic, call the hint function and print if it returns Some.

### Implementation Details
- Add a `shown_hints: HashSet<String>` to the REPL state (or use a static/thread-local similar to context_budget_warning's pattern)
- The `contextual_hint` function checks conditions in priority order, skips categories already shown
- Keep hint text short — one line max, under 80 chars
- Add tests for each hint condition (unit tests for the hint selection function)

### Testing
- Test that each condition produces the expected hint
- Test that shown hints aren't repeated
- Test that None is returned when no conditions match
- Test priority ordering (first match wins)
