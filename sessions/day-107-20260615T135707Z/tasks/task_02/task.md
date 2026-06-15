Title: Filter harness preflight crashes from state crashes output
Files: src/commands_state_crashes.rs
Issue: none
Origin: planner

Objective:
Prevent `yyds state crashes` from showing harness preflight artifacts (empty_input, slash_command_in_piped_mode) as real crash sessions — label them distinctly or filter them by default with a `--all` flag to include them.

Why this matters:
The assessment found `state crashes` shows many `empty_input` crashes from harness preflight/test invocations. These aren't real agent failures — they're the harness validating input before starting, getting empty input, and recording a RunCompleted error without any tool calls. They clutter the crash log and make it harder to spot real crashes. A developer running `state crashes` to diagnose a failure shouldn't have to mentally filter out harness plumbing noise.

The trajectory confirms: the crash log shows `empty_input` and `slash_command_in_piped_mode` errors that are harness artifacts, not agent failures.

Success Criteria:
- `yyds state crashes` (default) hides crashes with `error_detail` containing "empty_input" or "slash_command_in_piped_mode".
- A summary line reports how many preflight crashes were hidden, e.g., "(2 harness preflight crashes hidden; use --all to show)".
- `yyds state crashes --all` shows all crashes including preflight, with preflight entries labeled as "(preflight)" in the error column.
- `yyds state crashes --json` includes a `preflight_crashes_hidden` count in the JSON output.
- Real crash sessions (api_key missing, actual tool-call failures) continue to appear normally.

Verification:
- cargo test commands_state crashes -- --test-threads=1
- cargo build && cargo check
- Manual: run `yyds state crashes` on the current state store and verify preflight entries are hidden.

Expected Evidence:
- Assessment's future crash-log observation changes from "shows many empty_input crashes" to "shows only real crashes, preflight artifacts filtered."
- The `state crashes` output is cleaner and more actionable for debugging real harness failures.

Implementation Notes:
- Modify `build_crashes_report` in `src/commands_state_crashes.rs` (line 36).
- Add a `show_all: bool` parameter.
- After collecting crashes (line 132), split into real vs preflight based on `error_detail` string matching:
  - Preflight patterns: contains "empty_input", contains "slash_command_in_piped_mode"
  - Everything else is a real crash.
- In default mode (show_all=false), use only real crashes for display.
- In `--all` mode, show all crashes but append "(preflight)" to the error column for preflight entries.
- The `--all` flag is detected in `handle_crashes` (line 11) and passed through.
- In JSON mode, add `"preflight_crashes_hidden": N` to the output object.
- Keep changes scoped to `src/commands_state_crashes.rs` only.
