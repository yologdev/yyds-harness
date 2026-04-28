Title: Support bare positional prompts — `yoyo "fix this bug"` without --prompt
Files: src/cli.rs, src/dispatch.rs
Issue: none

## Problem

Every competitor (Claude Code, Codex, Aider) supports bare positional prompts:

    yoyo "fix this bug"
    yoyo "explain what this function does"

yoyo currently requires `--prompt "fix this bug"` or `-p "fix this bug"`. The assessment
specifically calls this out as a practical gap: "Minor but every competitor supports bare prompts."

## Implementation

In `cli.rs` `parse_args()`, after all flag parsing is done, collect any remaining
positional arguments that aren't flags (don't start with `-` and aren't consumed by
a flag) and aren't recognized subcommands. If there are remaining positional args and
no `--prompt` was specified, join them with spaces and use that as `prompt_arg`.

The key logic:
1. After the main flag-parsing loop, gather unclaimed positional args
2. Skip args[0] (binary name) and any that were consumed by flags
3. If `prompt_arg` is still `None` and there are remaining positional args, join them
4. This must NOT interfere with existing subcommand dispatch in `dispatch.rs` —
   subcommands like `help`, `version`, `doctor`, `setup`, `update` are already handled
   by `try_dispatch_subcommand()` which runs first

Edge cases to handle:
- `yoyo --model gpt-4 "do something"` — flag consumes next arg, "do something" is the prompt
- `yoyo "do something" --json` — prompt is positional, flags still work
- `yoyo` with no args — still launches REPL (no change)
- `yoyo help` — still dispatched as subcommand (no change)

Add tests:
- Bare prompt `yoyo "fix bug"` sets prompt_arg
- Bare prompt with flags `yoyo --model gpt-4 "fix bug"` works
- No positional args still results in None prompt_arg
- Existing subcommands still dispatch correctly (regression)

Update `--help` text in `help.rs` to mention bare prompt usage in the USAGE section.
