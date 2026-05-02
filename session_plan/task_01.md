Title: Non-interactive code review CLI subcommand
Files: src/dispatch_sub.rs, src/commands_git_review.rs
Issue: none

## What

Add a `yoyo review` CLI subcommand that works non-interactively — no REPL, no agent session required. This is the #3 competitive gap identified in the assessment: Claude Code has `/ultrareview` for CI-based PR review, Cursor has BugBot. yoyo has `/review` but it only works inside an interactive REPL session.

## Why

This is the biggest actionable competitive gap for a CLI tool. Non-interactive review means yoyo can be used in CI pipelines (`yoyo review` in a GitHub Actions step), in git hooks, or piped to files. It makes yoyo useful for code review automation without requiring a human at the keyboard.

## How

1. In `dispatch_sub.rs`, upgrade the existing "review" subcommand handler. Currently it just builds and prints the review prompt text without actually running it through the agent. Change it to:
   - Build the review content (diff/PR) using existing `build_review_content`
   - Build the review prompt using existing `build_review_prompt`  
   - Create a one-shot agent using `build_side_agent` (already exists in `agent_builder.rs`)
   - Run the prompt through the agent
   - Print the agent's response to stdout
   - Exit with code 0 on success, 1 on error

2. In `commands_git_review.rs`, add a new public function `run_non_interactive_review(arg: &str, model: &str, system_prompt: &str) -> Result<String, String>` that:
   - Calls `build_review_content(arg)` to get the diff
   - Calls `build_review_prompt` to format it
   - Creates a side agent, runs the prompt, collects the response
   - Returns the review text as a String

3. Support `yoyo review` (reviews staged changes or current branch diff), `yoyo review HEAD~3..HEAD` (specific range), `yoyo review --pr 123` (PR review).

## Testing

- Add a unit test that `build_review_content` and `build_review_prompt` produce valid content for common inputs (no agent needed)
- The function signatures should be testable without API keys
- Manual verification: `yoyo review HEAD~1` should output a review to stdout and exit

## Docs

Update `docs/src/usage/commands.md` to mention non-interactive review. Update `CLAUDE_CODE_GAP.md` to reflect this gap is now closed.
