Title: Add `state crashes` subcommand to surface silent-crash sessions
Files: src/commands_state.rs
Issue: none
Origin: planner

Objective:
Add a `yyds state crashes` subcommand that scans recent state events for the silent-crash signature (SessionStarted with api_key_present=false, followed by RunCompleted with status=error, with no tool calls between them) and displays them in a human-readable table.

Why this matters:
Task 1 captures crash data. This task makes it discoverable. Without a query interface, crash diagnostics exist in the event log but require manual JSONL inspection to find — which means they won't be found. The `state why last-failure` command only looks at the most recent FailureObserved event; it doesn't detect the silent-crash pattern (which often produces no FailureObserved event, just SessionStarted→RunCompleted).

A dedicated `state crashes` command makes the evidence actionable during assessment and evolution.

Success Criteria:
- `yyds state crashes` outputs a table of recent crash sessions with: run ID, timestamp, api_key_present, error_detail (if any), and time between SessionStarted and RunCompleted
- `yyds state crashes --limit N` limits output to N most recent crashes
- `yyds state crashes --json` outputs machine-readable JSON
- Command works correctly when there are zero crashes (displays "No crash sessions found")
- `cargo build && cargo test` passes

Verification:
- `cargo build` — compiles
- `cargo test` — all 89 tests pass
- `yyds state crashes --limit 5` — runs and produces output
- `yyds state crashes --json --limit 5` — produces valid JSON

Expected Evidence:
- The `state crashes` command becomes the go-to diagnostic during assessment
- Future assessment phases can query crash history without manual JSONL inspection
- If Task 1 captures good error_detail, this command will show it

Detailed description:

Add a `crashes` subcommand to the `state` command dispatch in `src/commands_state.rs`. The implementation:

1. **Detection algorithm**: Scan recent RunCompleted events. For each (run_id):
   - Find the SessionStarted event for the same run_id
   - Check if SessionStarted.api_key_present == false OR there are zero ToolCallStarted events between SessionStarted and RunCompleted
   - If either condition holds, it's a "crash session"
   - Extract: timestamp, api_key_present, error_detail (from RunCompleted payload), duration_ms

2. **Output format** (default, human-readable):
   ```
   Crashed sessions (last 5):
   RUN                                  WHEN          KEY?  ERROR
   run-1780943887123-12345              2h ago        no    exit code 1
   run-1780943123456-67890              5h ago        no    API key invalid
   ```

3. **JSON output** (`--json`):
   ```json
   {"crashes": [{"run_id": "...", "ts_ms": ..., "api_key_present": false, "error_detail": "...", "duration_ms": 12}], "total_crashes": 2, "window_sessions": 50}
   ```

4. **Implementation approach**: Add a handler function `handle_state_crashes(limit: usize, json_output: bool)` that:
   - Reads events from the state event store
   - Scans backward from most recent RunCompleted
   - Matches against SessionStarted for the same run_id
   - Checks for absence of tool calls
   - Formats and prints results

5. **Wire into command dispatch**: Add `"crashes"` to the state subcommand routing in `handle_state_subcommand()`.

6. **Edge cases**:
   - Zero crashes: print "No crash sessions found in recent history."
   - No state events at all: print "No state events available."
   - SessionStarted missing for a RunCompleted: skip that run (incomplete data)

TASK SIZING: This touches exactly 1 source file (commands_state.rs). The addition is a focused subcommand handler (~80-120 lines) plus a dispatch entry (~3 lines). It's additive — no existing behavior is modified.
