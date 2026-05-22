Title: Add /pr <number> comment --inline for posting review comments to GitHub
Files: src/commands_git.rs, src/commands_git_review.rs
Issue: none

## What

Close the "inline PR comments" competitive gap vs Claude Code. Currently `/pr <N> review`
generates a code review as text output in the terminal. Add the ability to post the review
as an actual GitHub PR review with inline comments.

New syntax: `/pr <number> review --post`

When `--post` is passed, instead of just printing the review, the agent:
1. Generates the review as structured JSON (file, line, comment)
2. Posts it as a GitHub PR review using `gh api` with inline comments

## Implementation

### In `src/commands_git.rs`:

1. Extend `PrSubcommand::Review` to carry a `post: bool` flag:
   ```rust
   Review(u64, bool),  // (PR number, post_to_github)
   ```

2. Update `parse_pr_args` to detect `--post` flag after `review`:
   - `/pr 42 review` → `Review(42, false)` (existing behavior)
   - `/pr 42 review --post` → `Review(42, true)`

3. In the `handle_pr` match arm for `Review`, when `post` is true, modify the review prompt
   to request structured JSON output, then call a new function `post_pr_review` that uses
   `gh api` to post the review.

### In `src/commands_git_review.rs`:

4. Add `pub fn post_pr_review(pr_number: u64, review_json: &str) -> Result<String, String>`
   that:
   - Parses the JSON review (array of `{path, line, body}` objects)
   - Calls `gh api repos/{owner}/{repo}/pulls/{pr}/reviews` with the review body and comments
   - Returns success/failure message

5. Add `pub fn build_review_prompt_structured(diff: &str) -> String` that asks the AI to
   produce a JSON review format alongside the human-readable review.

6. Add tests:
   - Test parsing of structured review JSON
   - Test `parse_pr_args` with `--post` flag

### Help text

7. Update `/pr` help in `src/help_data.rs` to document the `--post` flag.

## Verification
- `cargo build && cargo test`
- Parse tests verify `--post` flag handling
- JSON parsing tests verify review structure
