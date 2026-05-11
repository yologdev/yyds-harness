Title: Extract testable command routing from dispatch.rs and add tests
Files: src/dispatch.rs
Issue: none

## What

The assessment notes that `dispatch.rs` (766 lines) has **zero tests** — it's the REPL's traffic cop and has no test coverage at all. The problem is that the entire module is one big `async fn dispatch_command()` that requires a full `DispatchContext` with a live `Agent` to test.

Extract the pure command-routing logic into a testable synchronous function, then add comprehensive tests for it.

## Why

Test coverage is our moat (2,502 tests). Having the central command router completely untested weakens that claim. The routing logic (which input maps to which command category) is inherently pure and testable — we just need to separate it from the side effects.

## Design

### Step 1: Define a CommandRoute enum

```rust
/// What category of command the input maps to, determined purely from the input string.
/// This is the testable, pure routing layer — no agent state needed.
#[derive(Debug, PartialEq)]
enum CommandRoute {
    Quit,
    Help,
    Version,
    Status,
    Tokens,
    Cost,
    Changelog,
    Evolution,
    Clear,
    ClearForce,
    Model,
    Provider,
    Think,
    Save,
    Load,
    Stash,
    Checkpoint,
    Diff,
    Blame,
    Undo,
    Commit,
    Pr,
    Git,
    Review,
    Rename,
    Extract,
    Move,
    Refactor,
    Grep,
    Find,
    Index,
    Outline,
    Map,
    Ast,
    Add,
    Web,
    Apply,
    Copy,
    Open,
    Explain,
    Export,
    Context,
    Init,
    Docs,
    History,
    Search,
    Mark,
    Jump,
    Marks,
    Config,
    Hooks,
    Permissions,
    Mcp,
    Teach,
    Architect,
    Run,
    Test,
    Lint,
    Todo,
    Tree,
    Watch,
    Loop,
    Spawn,
    Update,
    Doctor,
    Health,
    Fix,
    Goal,
    Skill,
    Plan,
    Remember,
    Memories,
    Changes,
    Compact,
    Profile,
    Retry,
    Forget,
    Bg,
    Fork,  // if task 1 lands first
    Side,
    Quick,
    Extended,
    CustomCommand(String),
    UnknownCommand,
    NotACommand,
}
```

### Step 2: Add `fn route_command(input: &str) -> CommandRoute`

A pure synchronous function that takes an input string and returns which route it matches. This extracts the pattern matching from `dispatch_command` without any side effects. The function should handle:
- Exact matches (`/quit`, `/exit`, `/version`)
- Prefix matches (`/help `, `/model `, etc.)
- Unknown commands (starts with `/` but not recognized)
- Non-commands (no leading `/`)

### Step 3: Update `dispatch_command` to use `route_command`

Replace the outer `match ctx.input { ... }` with:
```rust
match route_command(ctx.input) {
    CommandRoute::Quit => CommandResult::Quit,
    CommandRoute::Help => { ... }
    // etc
}
```

This keeps the existing behavior identical but the routing decision is now in a testable function.

### Step 4: Add tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_quit() {
        assert_eq!(route_command("/quit"), CommandRoute::Quit);
        assert_eq!(route_command("/exit"), CommandRoute::Quit);
    }

    #[test]
    fn test_route_help() {
        assert_eq!(route_command("/help"), CommandRoute::Help);
        assert_eq!(route_command("/help commands"), CommandRoute::Help);
    }

    #[test]
    fn test_route_not_a_command() {
        assert_eq!(route_command("hello world"), CommandRoute::NotACommand);
        assert_eq!(route_command(""), CommandRoute::NotACommand);
    }

    #[test]
    fn test_route_unknown_command() {
        assert_eq!(route_command("/nonexistent"), CommandRoute::UnknownCommand);
        assert_eq!(route_command("/foobar arg"), CommandRoute::UnknownCommand);
    }

    #[test]
    fn test_route_model_subcommands() {
        assert_eq!(route_command("/model sonnet"), CommandRoute::Model);
        assert_eq!(route_command("/model list"), CommandRoute::Model);
    }

    // Test every single command route to ensure full coverage
    #[test]
    fn test_route_all_commands() {
        // Test at least one representative for each variant
        let cases = vec![
            ("/version", CommandRoute::Version),
            ("/status", CommandRoute::Status),
            ("/tokens", CommandRoute::Tokens),
            ("/cost", CommandRoute::Cost),
            ("/clear", CommandRoute::Clear),
            ("/clear!", CommandRoute::ClearForce),
            ("/save", CommandRoute::Save),
            ("/save mysession", CommandRoute::Save),
            ("/diff", CommandRoute::Diff),
            ("/diff --staged", CommandRoute::Diff),
            ("/commit", CommandRoute::Commit),
            ("/commit fix typo", CommandRoute::Commit),
            // ... etc for all routes
        ];
        for (input, expected) in cases {
            assert_eq!(route_command(input), expected, "failed for input: {input}");
        }
    }
}
```

### Important constraints
- Do NOT change any observable behavior — this is a pure refactor + test addition
- The `route_command` function must handle custom commands by checking `is_custom_command()` / `get_custom_command_content()` — but since those have side effects (file I/O), the routing for custom commands can remain in `dispatch_command`. `route_command` should return `UnknownCommand` for unrecognized `/` prefixed input, and `dispatch_command` can then check for custom commands before reporting unknown.
- Actually, since `is_unknown_command()` in commands.rs checks KNOWN_COMMANDS, use that same logic. If input starts with `/` and doesn't match a known pattern, route_command returns UnknownCommand. The custom command check stays in dispatch_command.
- Keep the function `pub(crate)` for testability within the crate
- Aim for 15-20 test cases covering the main branches
