Title: Add /revisit command — shelved-issue review mechanism
Files: src/commands.rs, src/dispatch.rs, + new src/commands_revisit.rs (max 3 files)
Issue: #388

Implement a `/revisit` command that helps the agent (and users) review closed/shelved issues
that may now be feasible given new capabilities. This addresses issue #388's core insight:
ideas get shelved and never re-evaluated.

## What to build

Create `src/commands_revisit.rs` with a `handle_revisit` function that:

1. **`/revisit scan`** (default subcommand) — Uses `gh issue list --state closed --limit 30 --json number,title,labels,closedAt,body` to fetch recently closed issues. Filters to issues closed with labels like `wontfix`, `deferred`, `too-complex`, or issues closed by the agent itself. Displays a summary table: issue number, title, when closed, why closed (from last comment or close reason).

2. **`/revisit check #N`** — Takes a specific closed issue number. Reads the issue body and comments. Prints a summary of what was requested and why it was shelved. Adds context about what capabilities have been added since the issue was closed (by checking the day the issue was closed vs current day count, and listing major changelog sections since then).

3. **`/revisit list`** — Shows issues that were previously marked as "revisit candidates" (stored in a simple `.yoyo/revisit.json` file with issue number, reason shelved, and day to revisit).

## Integration

- Add `"revisit"` to `KNOWN_COMMANDS` in `commands.rs`
- Add routing in `dispatch.rs` for the `/revisit` command
- Add help text via `command_short_description` in `help.rs` — but since help.rs is a 4th file, instead add the short description inline in `commands.rs` where other command metadata lives, or just add it to `command_arg_hint` which is already in `commands.rs`.

## Tests

Write tests for:
- Parsing revisit subcommands
- Formatting the revisit candidate list
- The `.yoyo/revisit.json` serialization/deserialization

Keep the implementation focused — this is a foundation that the evolution pipeline can later
use during Phase A1 assessment to automatically check if any shelved issues are now tractable.

## Size guard
- `commands_revisit.rs`: ~200-300 lines max
- `commands.rs` changes: ~5 lines (add to KNOWN_COMMANDS + hints)
- `dispatch.rs` changes: ~5 lines (add route)
