Title: Add /history command for evolution history (Issue #226)
Files: src/commands_info.rs, src/help.rs, src/commands.rs
Issue: #226

Issue #226 asks for the ability to view yoyo's evolution history from within yoyo. Currently `/changelog` shows git commits, but there's no way to see the evolution story — day count, session patterns, milestone highlights.

Implement a `/history` command that shows evolution history by parsing DAY_COUNT and git tags/commits:

1. In `src/commands_info.rs`, add `handle_history_evolution(input: &str)`:
   - Read DAY_COUNT file to get current day number
   - Run `git tag --sort=-creatordate` to list evolution tags (dayNN-HH-MM format)
   - Parse tags to extract day numbers and session times
   - Count sessions per day, total sessions, streaks
   - Format output showing:
     - Current day and total sessions
     - Recent sessions (last N, default 10, configurable via `/history 20`)
     - Sessions per day stats (average, max, streak info)
   - Use existing Color constants for formatting (BOLD, DIM, GREEN, CYAN, etc.)

2. In `src/help.rs`, add help text for `/history`:
   - Brief: "Show evolution history and session stats"
   - Detailed: explain the output format, optional count argument

3. In `src/commands.rs`, add "history" to KNOWN_COMMANDS array and wire it in command_arg_hint.

4. Wire the command dispatch in `repl.rs` or wherever slash commands are dispatched (check where `/changelog` is dispatched and follow the same pattern — likely in `repl.rs::run_repl` or `commands.rs`).

Wait — that's 4 files. Reduce scope: skip modifying help.rs for now. The command help can be added in the KNOWN_COMMANDS entry. Focus on:
- `src/commands_info.rs` — implement `handle_history_evolution`
- `src/commands.rs` — add to KNOWN_COMMANDS, wire dispatch
- `src/repl.rs` — wire `/history` to call `handle_history_evolution` (check where `/changelog` is dispatched)

The dispatch pattern (confirmed by grep): `/changelog` dispatches in `repl.rs` line 368-369, is listed in `commands.rs` KNOWN_COMMANDS (line 112), and implemented in `commands_info.rs`. Follow the exact same pattern for `/history`.

Implementation details:
- Parse git tags matching pattern `day\d+-\d+-\d+` 
- Group by day number
- Show formatted table:
  ```
  🐙 Evolution History — Day 55
  
  55 days | ~150 sessions | 2,109 tests

  Recent sessions:
    Day 55  01:18  (today)
    Day 54  15:04  Five sessions of standing still
    Day 54  04:40  Knowing where you were built
    Day 53  19:11  The file that was three things pretending to be one
    ...
  ```
- For journal titles: optionally parse first line of each journal entry from journals/JOURNAL.md matching the day/time
- Add tests: parse tag format, count sessions, handle missing DAY_COUNT

Run `cargo build && cargo test` to verify.
