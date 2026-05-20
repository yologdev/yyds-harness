Title: Add /pr review <number> — AI-powered pull request code review
Files: src/commands_git.rs, src/help.rs (or src/help_data.rs if Task 1 ran first)
Issue: none

## What

Add a `review` subcommand to `/pr` that fetches a PR's diff and optionally its description, then sends them to the agent for an AI-powered code review.

Usage:
```
/pr 42 review       — review PR #42
/pr review 42       — alternative syntax (review first, then number)
```

## Why

Code review is one of the most common developer workflows. Claude Code, Cursor, and other competitors offer PR review capabilities. Currently yoyo has `/review` for local staged/unstaged changes, but reviewing a *remote PR* requires manually fetching the diff. `/pr review` closes this gap — a developer can review any PR directly from the CLI.

## How

### 1. Add `Review(u32)` variant to `PrSubcommand` enum

```rust
pub enum PrSubcommand {
    List,
    View(u32),
    Diff(u32),
    Review(u32),  // NEW
    Comment(u32, String),
    Checkout(u32),
    Create { draft: bool },
    Help,
}
```

### 2. Parse "review" in `parse_pr_args`

In the match on `parts[1]`, add:
```rust
"review" => PrSubcommand::Review(number),
```

Also handle `/pr review <number>` syntax (review as first word):
```rust
if parts[0].eq_ignore_ascii_case("review") {
    if let Some(num_str) = parts.get(1) {
        if let Ok(n) = num_str.parse::<u32>() {
            return PrSubcommand::Review(n);
        }
    }
    return PrSubcommand::Help;
}
```

### 3. Handle `PrSubcommand::Review(number)` in `handle_pr`

```rust
PrSubcommand::Review(number) => {
    let num_str = number.to_string();
    
    // Fetch PR diff
    let diff = match std::process::Command::new("gh")
        .args(["pr", "diff", &num_str])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).to_string()
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("{RED}  error: {}{RESET}\n", stderr.trim());
            return;
        }
        Err(_) => {
            eprintln!("{RED}  error: `gh` CLI not found{RESET}\n");
            return;
        }
    };
    
    if diff.trim().is_empty() {
        eprintln!("{DIM}  PR #{number} has no diff{RESET}\n");
        return;
    }
    
    // Optionally fetch PR title/body for context
    let pr_info = std::process::Command::new("gh")
        .args(["pr", "view", &num_str, "--json", "title,body", "--jq", ".title + \"\\n\\n\" + .body"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    
    // Build review prompt
    let prompt = format!(
        "Review this pull request (PR #{number}). Analyze the diff for:\n\
         - Potential bugs or logic errors\n\
         - Code quality issues\n\
         - Missing error handling\n\
         - Performance concerns\n\
         - Suggestions for improvement\n\n\
         Be specific — reference file names and line numbers from the diff.\n\
         Praise good patterns too. Be constructive.\n\n\
         {}\
         ## Diff\n\n```diff\n{}\n```",
        if pr_info.trim().is_empty() { String::new() } else { format!("## PR Description\n\n{}\n\n", pr_info.trim()) },
        // Truncate diff if very large
        if diff.len() > 50_000 { &diff[..safe_truncate_pos(&diff, 50_000)] } else { &diff }
    );
    
    // Send to agent (similar to existing /diff --explain flow)
    eprintln!("{DIM}  [review] analyzing PR #{number}...{RESET}");
    auto_compact_if_needed(agent, session_total, model).await;
    run_prompt(agent, &prompt, session_total, model).await;
}
```

Use `crate::format::safe_truncate` or compute a safe truncation point with `is_char_boundary()`.

### 4. Add tests for `parse_pr_args` with "review"

```rust
assert_eq!(parse_pr_args("42 review"), PrSubcommand::Review(42));
assert_eq!(parse_pr_args("review 42"), PrSubcommand::Review(42));
```

### 5. Update help text

In the command_help match for "pr" (in help.rs or help_data.rs), add the review subcommand to the usage docs.

Also update `PR_SUBCOMMANDS` in `commands.rs` to include "review" for tab completion.

## Verification

- `cargo build` clean
- `cargo test` passes (new parse tests + existing tests)
- `cargo clippy --all-targets -- -D warnings` clean
- Manual test: `/pr review 1` should work if `gh` is available (won't work in CI but parser tests verify the routing)
