Title: Introduce PostPromptContext struct to eliminate 13-parameter handle_post_prompt
Files: src/repl.rs
Issue: none

## Goal

Remove the `#[allow(clippy::too_many_arguments)]` on `handle_post_prompt` (line 444) by bundling its 13 parameters into a `PostPromptContext` struct. This was identified in the Day 66 assessment as a code smell from Day 65's extraction — the function was extracted but not restructured.

## What to implement

1. **Define `PostPromptContext`** struct near the top of `repl.rs` (or just above `handle_post_prompt`):

```rust
struct PostPromptContext<'a> {
    outcome: &'a PromptOutcome,
    agent: &'a mut yoagent::agent::Agent,
    agent_config: &'a mut AgentConfig,
    session_total: &'a mut Usage,
    session_changes: &'a SessionChanges,
    turn_history: &'a mut TurnHistory,
    turn_snap: TurnSnapshot,
    changes_before: &'a [String],
    last_error: &'a mut Option<String>,
    prompt_start: Instant,
    effective_input: &'a str,
}
```

2. **Rewrite `handle_post_prompt`** to take `ctx: PostPromptContext<'_>` as a single argument instead of 13 separate parameters. Remove the `#[allow(clippy::too_many_arguments)]` attribute.

3. **Update the call site** (around line 858) to construct a `PostPromptContext` and pass it.

4. **Verify** clippy is clean without the allow attribute.

## Verification

```bash
cargo build && cargo clippy --all-targets -- -D warnings && cargo test
```

The function body stays the same — this is purely a signature refactor. No behavioral change.
