Title: Enrich the default system prompt with behavioral guidance
Files: src/cli_config.rs
Issue: none

## Problem

The current default `SYSTEM_PROMPT` in `cli_config.rs` is only 5 lines:

```
You are a coding assistant working in the user's terminal.
You have access to the filesystem and shell. Be direct and concise.
When the user asks you to do something, do it — don't just explain how.
Use tools proactively: read files to understand context, run commands to verify your work.
After making changes, run tests or verify the result when appropriate.
```

This is the single biggest competitive gap against Claude Code, whose system prompt provides detailed behavioral guidance on:
- How to approach multi-file changes
- When to search vs read files
- Error recovery strategies
- How to verify work
- Working with git
- Being mindful of context

## Implementation

Expand `SYSTEM_PROMPT` in `src/cli_config.rs` to include behavioral guidance. Keep it concise but actionable. Target: ~15-25 lines (not a wall of text — still fits well within context budgets). The prompt should guide the agent to:

1. **Search before reading** — Use `search` and `list_files` to find relevant code before reading whole files. Don't guess at file paths.
2. **Verify changes** — After making edits, run the project's test/build commands to verify nothing is broken.
3. **Think before multi-file edits** — When a change spans multiple files, plan the approach first. Make changes incrementally and test between steps.
4. **Handle errors gracefully** — If a command fails or an edit doesn't match, read the error carefully. Check the actual file content before retrying.
5. **Use git awareness** — Check `git status` and `git diff` to understand the current state. Don't make changes that conflict with uncommitted work without asking.
6. **Be efficient with context** — Don't read entire large files when you only need a specific function. Use search to find the right section first.
7. **Confirm destructive operations** — Before deleting files, resetting git state, or other irreversible actions, confirm with the user.

Also update `LITE_SYSTEM_PROMPT` to stay meaningfully shorter (it's for small local models with limited context) — just add one line about verifying changes.

**Important constraints:**
- Keep the raw string format (`r#"..."#`)
- Maintain the existing 5 lines as the opening (don't break the existing voice/tone)
- Add new guidance as a separate paragraph or bullet list below the opening
- Update existing tests that check `SYSTEM_PROMPT` content (there's one that checks it contains "coding assistant") — verify it still passes
- The `LITE_SYSTEM_PROMPT` test checks it's shorter than `SYSTEM_PROMPT` — that must still hold

Run `cargo test` and `cargo clippy --all-targets -- -D warnings` to verify.
