Title: Skip corrupted events.jsonl lines instead of failing entire read
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Assessment bug #1 [MEDIUM]: "events.jsonl parse error at line 46956 — state doctor reports 'EOF while parsing a string' during event parsing. This blocks full event scanning for diagnostic commands that don't use tail-based reading."
- Self-test output: `yyds state doctor` reports "parse error at line 46956 of events.jsonl: EOF while parsing a string at line 1 column 917"
- Code evidence: `read_compatibility_events()` (src/state.rs:3152-3165) uses `Iterator::collect()` which fails-fast on the first parse error. If any single line is corrupted, the entire events file becomes unreadable for all callers — blocking state doctor, state why, and other diagnostic commands.
- The events file is 51.4MB; one corrupted line (likely a truncated write from a prior session crash) at line 46956 deep in history should not prevent reading all other lines.

Edit Surface:
- src/state.rs (read_compatibility_events function, ~lines 3152-3165)

Verifier:
- cargo test -- test_read_compatibility
- cargo test -- state

Fallback:
- If changing the return type has too many call-site impacts, add a new `read_compatibility_events_lossy()` function instead and use it in diagnostic commands. Do not refactor all callers unless the change is trivial.

Objective:
Make `read_compatibility_events()` gracefully skip corrupted JSON lines instead of failing the entire read, so that one bad line deep in history doesn't block diagnostic commands from reading the rest of the events file.

Why this matters:
The events.jsonl file is the primary diagnostic surface for state health. When `state doctor` can't read past line 46956, it can't compute gnome metrics, detect lifecycle gaps, or surface patterns from the full history. One truncated write from a prior session crash shouldn't permanently blind all diagnostic tools. This is a 20-minute fix with high diagnostic leverage — it unblocks future state inspection without touching protected files.

Success Criteria:
- `read_compatibility_events()` returns all parseable lines instead of failing on the first corrupted line
- Corrupted lines are counted and reported (e.g., via a warning or a separate counter) so the user knows data was skipped
- The function signature may change to return `(Vec<Value>, Vec<String>)` where the second element is parse errors, OR it may log warnings via eprintln! and return only valid lines
- A unit test demonstrates: a file with 5 valid lines + 1 corrupted line returns 5 events and 1 warning

Verification:
- cargo build
- cargo test -- state
- cargo test -- test_read_compatibility

Expected Evidence:
- `yyds state doctor` no longer reports a parse error blocking full scan
- State doctor can compute gnome metrics from the full events file (minus the one skipped line)
- Future events file corruption from truncated writes doesn't block diagnostics

Implementation Notes:
- Change `read_compatibility_events` from fail-fast collect() to a loop that filters out errors
- Options for error reporting:
  a) Return `Result<Vec<Value>, String>` but skip individual bad lines and eprintln! a warning with the line number
  b) Change return type to `(Vec<Value>, Vec<String>)` where the second vec holds error messages
  Option (a) is simpler and keeps callers unchanged; option (b) is more testable
- Choose (a) for minimal caller impact — log warnings for skipped lines, return only valid events
- Add a unit test: create a temp file with mixed valid and invalid JSON lines, verify the function returns only valid lines
- Update any doc comments on the function to note the lossy behavior
- Check callers of `read_compatibility_events` to ensure they handle the result correctly (they already expect Result, so the change is backward-compatible if using option a)
