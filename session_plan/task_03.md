Title: Add effort levels to /review command
Files: src/commands_git_review.rs, src/help_data.rs
Issue: none

## What

Add effort levels to the `/review` command: `/review --quick`, `/review --thorough`, and the default (normal). The assessment specifically identifies this as a competitive gap: "Claude Code renamed /simplify to /code-review with effort levels and inline PR comments. Yoyo has the pieces (/pr review) but hasn't unified them into a polished code-review workflow."

Effort levels change the review prompt to be more or less detailed:
- `--quick`: Focus on bugs and security only. Skip style and minor suggestions. Be terse.
- (default): Current behavior — bugs, security, style, performance, suggestions.
- `--thorough`: Deep review. Also check error handling edge cases, API contract violations, test coverage gaps, documentation accuracy, concurrency safety. Be exhaustive.

## Implementation

### In `src/commands_git_review.rs`:

1. Add enum `ReviewEffort { Quick, Normal, Thorough }` with a `label()` method.

2. Add function `parse_review_effort(input: &str) -> (ReviewEffort, String)`:
   - Strip `--quick` or `--thorough` flags from the input
   - Return the effort level and the remaining input (file path or empty)
   - Default to `Normal` if no flag present

3. Modify `build_review_prompt(label: &str, content: &str) -> String` to accept an effort parameter:
   - Change signature to `build_review_prompt(label: &str, content: &str, effort: ReviewEffort) -> String`
   - For `Quick`: shorter prompt focusing on bugs and security only, asking for terse output
   - For `Normal`: current prompt (unchanged)
   - For `Thorough`: expanded prompt with additional review dimensions (error handling, concurrency, test gaps, docs)

4. Update `handle_review()` to parse effort from input and pass to `build_review_prompt`.

5. Update `run_non_interactive_review()` to accept effort and pass through.

6. Update all call sites of `build_review_prompt` (check: there may be 2-3 call sites).

### In `src/help_data.rs`:

7. Update the help text for `/review` to mention effort levels:
   ```
   /review [--quick|--thorough] [file]  — review staged changes or a file
   ```

### Tests

Add/update tests in `src/commands_git_review.rs`:
- `test_parse_review_effort_default` — no flag → Normal
- `test_parse_review_effort_quick` — `--quick` → Quick with remaining args
- `test_parse_review_effort_thorough` — `--thorough` → Thorough
- `test_build_review_prompt_quick` — quick prompt is shorter, mentions bugs/security
- `test_build_review_prompt_thorough` — thorough prompt mentions additional dimensions
- Update existing `build_review_prompt_contains_label` test to pass effort parameter
