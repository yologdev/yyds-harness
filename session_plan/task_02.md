Title: Add /diff --explain for AI-powered diff explanation
Files: src/commands_git.rs, src/dispatch.rs
Issue: none

## What to do

Add an `--explain` flag to `/diff` that sends the current diff to the agent for a natural-language summary explaining what changed and why. This is a competitive feature — Claude Code naturally explains diffs in conversation, but having a one-command workflow for "explain my current changes" is a real developer productivity gain.

### Implementation plan

**1. Add `explain` field to `DiffOptions` (in `commands_git.rs`):**
```rust
pub struct DiffOptions {
    pub staged_only: bool,
    pub name_only: bool,
    pub stat_only: bool,
    pub explain: bool,  // NEW
    pub file: Option<String>,
}
```

Parse `--explain` in `parse_diff_args`.

**2. Add `handle_diff_explain` async function (in `commands_git.rs`):**

```rust
pub async fn handle_diff_explain(
    input: &str,
    agent: &mut Agent,
    session_total: &mut Usage,
    model: &str,
) -> Option<String> {
    // Parse diff args, gather the diff text (both staged + unstaged, or just staged if --staged)
    // Build a prompt like:
    //   "Explain the following code changes. Describe what was changed, why it might have been
    //    changed, and any potential issues. Be concise.\n\n```diff\n{diff}\n```"
    // Call run_prompt(agent, &prompt, session_total, model).await
    // Call auto_compact_if_needed(agent)
    // Return Some(prompt) so it's tracked in conversation
}
```

Follow the same pattern as `handle_review` in `commands_git_review.rs` — it gathers diff content, builds a prompt, and calls `run_prompt`.

The diff should include both unstaged and staged changes (unless `--staged` is specified). Truncate the diff to a reasonable size (e.g., 50KB) to avoid overwhelming the context.

**3. Update dispatch routing (in `dispatch.rs`):**

In the `CommandRoute::Diff` match arm, check if the input contains `--explain`:
```rust
CommandRoute::Diff => {
    let opts = commands::parse_diff_args(ctx.input);
    if opts.explain {
        if let Some(prompt) = commands::handle_diff_explain(
            ctx.input, ctx.agent, ctx.session_total, ctx.model,
        ).await {
            last_prompt = Some(prompt);
        }
    } else {
        commands::handle_diff(ctx.input);
    }
    CommandResult::Continue
}
```

**4. Add tests:**
- Test that `parse_diff_args("/diff --explain")` sets `explain: true`
- Test that `parse_diff_args("/diff --staged --explain")` sets both
- Test that `parse_diff_args("/diff --explain src/main.rs")` sets explain + file

**5. Update help text in help.rs:**
Add `--explain` to the `/diff` command help:
```
/diff --explain     AI-powered explanation of current changes
```

Wait — help.rs is a 3rd file. Let me be precise about the 3-file limit:
- `src/commands_git.rs` — add explain flag + handler
- `src/dispatch.rs` — route --explain to async handler

That's 2 files. Help text update can be a follow-up or part of another task (or the help.rs update is small enough to include as a 3rd file).

Actually, include `src/help.rs` as the 3rd file — just add one line to the /diff help section documenting --explain.

### Verification:
- `cargo build && cargo test`
- `cargo clippy --all-targets -- -D warnings`
- New tests for parse_diff_args with --explain flag
