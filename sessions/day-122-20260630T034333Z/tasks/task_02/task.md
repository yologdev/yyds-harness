Title: Fix yyds state crashes timeout — add event sampling
Files: src/commands_state_crashes.rs
Issue: none
Origin: planner

Evidence:
- `yyds state crashes --limit 5` timed out at 10s during Day 122 preflight self-test.
  The state events file is 66.9MB with ~62K events. The crash analyzer scans the full
  event stream, which exceeds practical runtime at this volume.
- State doctor (`yyds state doctor`) already has a sampling fix: it limits event
  scanning to the most recent 20K events to avoid timeout. Other state subcommands
  haven't received similar treatment.
- `src/commands_state_crashes.rs` exists as a separate module (74 lines visible in
  dispatch). `handle_crashes` at line 11 reads events without a volume cap.
- The assessment confirms "State doctor already has a sampling fix; other state
  subcommands need similar treatment."

Edit Surface:
- src/commands_state_crashes.rs

Verifier:
- cargo build && cargo test
- timeout 10 yyds state crashes --limit 5

Fallback:
- If the crashes handler already has a sampling/limit but the bottleneck is JSON
  deserialization of the full file (not event scanning), the fix moves to the
  event reader in src/state.rs to support a line-offset+limit read mode.
- If the `--limit` flag already bounds output but not input scanning, check whether
  the scan itself needs a cap distinct from the display limit.

Objective:
Make `yyds state crashes --limit 5` complete within 10 seconds by capping the
number of events scanned, matching the approach already used by state doctor.

Why this matters:
The crashes command is a diagnostics tool that can't complete — it fails silently
(timeout) on the current event volume. A diagnostic tool that can't run is not a
diagnostic tool. The state doctor already proved the fix pattern works (sampling
the most recent N events). This task extends that pattern to crashes.

Success Criteria:
- `timeout 10 yyds state crashes --limit 5` completes and prints crash evidence
  or "no crashes found"
- `yyds state crashes --limit 50` still works (tests higher limit)
- Existing tests in `commands_state_crashes.rs` still pass
- `cargo build && cargo test` passes

Verification:
- cargo build
- cargo test --lib
- timeout 10 cargo run -- yyds state crashes --limit 5
- timeout 15 cargo run -- yyds state crashes --limit 20

Expected Evidence:
- `yyds state crashes --limit 5` completes within timeout
- Task lineage shows file edits in src/commands_state_crashes.rs
- State crashes subcommand becomes usable in CI self-test reports

Implementation:
1. In `src/commands_state_crashes.rs`, find where the event stream is read/scanned.
   The `handle_crashes` function (line 11) and `build_crashes_report` (line 42)
   likely iterate over all events.
2. Add a `MAX_EVENTS_SCAN` constant (e.g., 20_000, matching state doctor's cap).
   When the events file has more than this many lines, only scan the most recent
   `MAX_EVENTS_SCAN` events.
3. Print an informational line when events were truncated:
   "Scanned most recent {scanned} of {total} events (capped for performance)."
4. Respect the existing `--limit` flag for output display count; the scan cap is
   separate from the display cap.
5. If the module uses `state.rs` event-reading functions, check whether a
   parameterized read (offset + limit) already exists and use it instead of reading
   the whole file into memory.
