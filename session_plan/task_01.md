Title: Add /review --fix mode to auto-apply code review findings
Files: src/commands_git_review.rs, src/dispatch.rs
Issue: none

## What

Claude Code has `/code-review --fix` that applies review findings directly to the working tree. yoyo's `/review` only shows findings. Add a `--fix` flag that, after generating the review, feeds the findings back to the main agent as a prompt to fix them.

## Implementation

1. **In `commands_git_review.rs`:**
   - Add `--fix` to `parse_review_effort()` — return a new boolean alongside the effort level. Rename to `parse_review_flags()` or add a `ReviewFlags` struct with `effort: ReviewEffort` and `fix: bool`.
   - Modify `handle_review()` to accept the fix flag. When `--fix` is set:
     - Run the review as normal (collect the output).
     - After the review completes, construct a follow-up prompt like: "Based on the code review above, apply the suggested fixes to the codebase. Fix each issue found. Here are the review findings:\n\n{review_output}"
     - Return this prompt string so the REPL can feed it back to the agent (similar to how watch-mode fix prompts work).
   - Add `--fix` to the help text / doc comment for handle_review.

2. **In `dispatch.rs`:**
   - Where `/review` is dispatched, handle the returned fix prompt. If `handle_review` returns a fix prompt (when `--fix` was used), feed it into the agent as a follow-up turn using `run_prompt()`.

3. **Tests:**
   - Test `parse_review_flags` parses `--fix` correctly alongside `--quick`/`--thorough`.
   - Test that `--fix` can combine with effort flags: `/review --fix --thorough src/main.rs`.
   - Test that the fix prompt contains the review output.

## Sizing
Touches 2 source files. The core change is flag parsing + prompt construction — well-scoped for 20 minutes.
