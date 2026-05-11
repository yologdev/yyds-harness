Title: Add help coverage guard test — cross-reference KNOWN_COMMANDS against help entries
Files: src/help.rs
Issue: none

## What

Add a test in `help.rs` that iterates over every entry in `KNOWN_COMMANDS` (from `commands.rs`) and verifies:

1. Every command (except aliases like `/exit`, `/clear!`, `/quit`) has a `command_help()` entry that returns `Some(...)`.
2. Every command has a `command_short_description()` entry that returns `Some(...)`.
3. Every command that has `command_help()` also appears in the `help_text()` output.

This is a regression guard — right now all 82 commands are covered, but as new commands are added, this test catches any that lack help documentation.

## Implementation

In the `#[cfg(test)]` section of `help.rs`, add:

```rust
#[test]
fn all_known_commands_have_help() {
    use crate::commands::KNOWN_COMMANDS;
    // Aliases and special variants that share help with their base command
    let aliases = ["/exit", "/clear!", "/quit"];
    
    for &cmd in KNOWN_COMMANDS {
        let name = cmd.strip_prefix('/').unwrap_or(cmd);
        if aliases.contains(&cmd) {
            continue;
        }
        assert!(
            command_help(name).is_some(),
            "command_help(\"{name}\") returned None — add help text for /{name}"
        );
        assert!(
            command_short_description(name).is_some(),
            "command_short_description(\"{name}\") returned None — add a short description for /{name}"
        );
    }
}

#[test]
fn all_help_commands_in_help_text() {
    use crate::commands::KNOWN_COMMANDS;
    let text = help_text();
    let aliases = ["/exit", "/clear!", "/quit"];
    
    for &cmd in KNOWN_COMMANDS {
        if aliases.contains(&cmd) {
            continue;
        }
        assert!(
            text.contains(cmd),
            "{cmd} is in KNOWN_COMMANDS but missing from help_text() output"
        );
    }
}
```

Also add a `help_command_completions_covers_known_commands` test that verifies `help_command_completions()` returns entries for all non-alias commands.

## Verification
`cargo test help` should pass. These tests will fail loudly if a future command is added to `KNOWN_COMMANDS` without corresponding help entries.
