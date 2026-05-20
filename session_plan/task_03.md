Title: AI-powered commit message generation for /commit
Files: src/commands_git.rs, src/dispatch.rs, src/help.rs
Issue: none

## What to do

Currently `/commit` (with no message) generates a heuristic commit message by parsing the diff for file paths and insertion/deletion counts. The result is generic: "feat(main): update code (+12, -3)". AI-powered commit message generation would produce messages like "refactor: extract symbol types into dedicated module for reuse" — actually describing *what* and *why*.

### Implementation plan

**1. Add `handle_commit_ai` async function in `commands_git.rs`:**

This function generates a commit message using a side agent (via `build_side_agent`), then presents it to the user for confirmation (same y/n/e flow as the current `handle_commit`).

```rust
pub async fn handle_commit_ai(
    input: &str,
    agent_config: &AgentConfig,
) {
    let arg = input.strip_prefix("/commit").unwrap_or("").trim();

    // If user provided an explicit message (not just flags), use it directly
    let explicit_msg = arg.replace("--generate", "").replace("--ai", "").trim().to_string();
    if !explicit_msg.is_empty() {
        // Just commit with the explicit message (existing behavior)
        let (ok, output) = run_git_commit_with_trailer(&explicit_msg);
        // print result...
        return;
    }

    // Get staged diff
    let diff = match get_staged_diff() {
        None => { eprintln!("error: not in a git repo"); return; }
        Some(d) if d.trim().is_empty() => { println!("nothing staged"); return; }
        Some(d) => d,
    };

    // Truncate diff for context (max ~30KB)
    let truncated = if diff.len() > 30_000 {
        format!("{}...\n(truncated)", &diff[..diff.floor_char_boundary(30_000)])
    } else {
        diff.clone()
    };

    eprintln!("  generating commit message...");

    // Build side agent and ask for a commit message
    let mut side_agent = agent_config.build_side_agent();
    let prompt = format!(
        "Generate a concise git commit message for the following diff. \
         Use conventional commit format (type: description). \
         The message should be a single line, max 72 characters. \
         No quotes, no backticks, just the message.\n\n```diff\n{truncated}\n```"
    );

    let mut rx = side_agent.prompt(&prompt).await;
    let mut message = String::new();
    loop {
        match rx.recv().await {
            Some(AgentEvent::MessageUpdate { delta: StreamDelta::Text { delta }, .. }) => {
                message.push_str(&delta);
            }
            Some(AgentEvent::AgentEnd { .. }) | None => break,
            _ => {}
        }
    }
    side_agent.finish().await;

    let message = message.trim().trim_matches('"').trim_matches('`').to_string();
    if message.is_empty() {
        // Fall back to heuristic
        let heuristic = generate_commit_message(&diff);
        // present heuristic with y/n/e flow...
        return;
    }

    // Present the AI-generated message for confirmation (same y/n/e flow)
    println!("  Suggested commit message:");
    println!("    {message}");
    // y/n/e prompt...
}
```

**2. Make `/commit` default to AI-generated when no message is given:**

The cleanest approach: when the user types `/commit` with no arguments, try the AI path. If the side agent fails (no API key, error), fall back silently to the heuristic.

Alternatively, add `--ai` / `--generate` flag and keep the default behavior unchanged. The safer approach is the flag approach — it's additive and doesn't change existing behavior.

**Decision: Add `--ai` flag.** `/commit --ai` uses AI generation. `/commit` (no args) still uses the heuristic. This is safe and additive.

**3. Update dispatch routing in `dispatch.rs`:**

The commit command route needs to check for `--ai` and call the async handler:
```rust
CommandRoute::Commit => {
    if ctx.input.contains("--ai") || ctx.input.contains("--generate") {
        commands::handle_commit_ai(ctx.input, ctx.agent_config).await;
    } else {
        commands::handle_commit(ctx.input);
    }
    CommandResult::Continue
}
```

**4. Add to help text in `help.rs`:**
Add `--ai` flag documentation to the `/commit` help entry.

**5. Tests:**
- Test that the flag is parsed correctly in the dispatch
- Test the prompt construction (unit test the prompt building without actually calling the API)
- Test fallback behavior when no diff is staged

### Verification:
- `cargo build && cargo test`
- `cargo clippy --all-targets -- -D warnings`
