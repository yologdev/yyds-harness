Title: Add bounded event-reading helper to state.rs and use it in state doctor
Files: src/state.rs, src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Six diagnostic tools have been individually patched with the same
  "read at most N events, tail-sample if larger" pattern to prevent timeouts
  with large event histories (50k+ events):
  - state doctor (src/commands_state.rs:170-183, `handle_doctor`)
  - crash scanner
  - benchmark scorer
  - cache reporter (src/commands_deepseek.rs:1978 with EVENT_SCAN_CAP)
  - terminal-state script (scripts/append_terminal_state_events.py)
  - state why (src/commands_state.rs, last-failure path)
- Each tool independently reimplements: count total lines → if over threshold →
  read tail N lines. The pattern is correct but duplicated.
- The `read_compatibility_events` function in `src/state.rs:3193` reads ALL
  events with no limit — it's the "old pattern" that new code should avoid.
- Assessment: "NOT a current bug — but the shared utility lesson remains
  unencoded." The lesson from Day 117-125 is that event-reading must be bounded.
- Encoding this as a single `pub` function in `src/state.rs` prevents future
  diagnostic tools from inheriting the unbounded-read bug.

Edit Surface:
- src/state.rs: add a `pub fn read_events_bounded(path, limit) -> (Vec<Value>, String)`
  that automatically tail-samples when the file has more than `limit` lines
- src/commands_state.rs: replace the inline tail-sampling logic in
  `handle_doctor` (lines 170-183) with a call to the new helper

Verifier:
- cargo build
- cargo test -- --test-threads=1
- cargo run -- yyds state doctor (must still work, show sampling note when
  events exceed limit)

Fallback:
- If the `handle_doctor` refactor would touch more than 10 lines or the
  existing `read_events` / `read_tail_events` functions are deeply coupled to
  their current callers, add only the `read_events_bounded` helper to
  `src/state.rs` as a standalone function and leave `handle_doctor` unchanged.
  The helper alone is useful for future consumers even without retrofitting
  existing callers.
- Do NOT refactor any other diagnostic command beyond `handle_doctor`.
- Do NOT modify scripts/append_terminal_state_events.py or any Python scripts.

Objective:
Add a single `pub fn read_events_bounded` to `src/state.rs` that encapsulates
the "tail-sample if too large" pattern, and use it in the state doctor to prove
it works. This encodes the lesson from Days 117-125 into a reusable utility.

Why this matters:
The assessment documents this as a pattern that's been individually patched 6
times. New diagnostic tools added in the future will likely repeat the old
"read everything" pattern unless a bounded helper is available. Encoding this as
a library function in `src/state.rs` (the canonical state-reading module) makes
the right thing the easy thing. This follows the Day 125 lesson: "after N
instances of the same fix, the fix is a utility."

Success Criteria:
- `src/state.rs` has a new `pub fn read_events_bounded(path: &Path, limit: usize)
  -> Result<(Vec<Value>, String), String>` that:
  - Counts total lines in the events file
  - If total ≤ limit: reads all events via existing `read_compatibility_events`
    (or equivalent JSONL reading), returns them with an empty note
  - If total > limit: reads the last `limit` lines, parses them as JSONL
    events, returns them with a note like "sampled from last {limit} of
    {total} total"
- `src/commands_state.rs` `handle_doctor` uses the new helper instead of its
  inline tail-sampling logic (lines 164-187)
- `yyds state doctor` output is identical to before for both small and large
  event histories
- `cargo build && cargo test` pass

Verification:
- cargo build && cargo test -- --test-threads=1
- cargo run -- yyds state doctor
  Expected: same output as current, sampling note appears when events > default
  limit (1000)
- cargo run -- yyds state doctor --limit 100
  Expected: works with custom limit

Expected Evidence:
- New `read_events_bounded` function in `src/state.rs`
- `handle_doctor` in `src/commands_state.rs` simplified by ~15 lines
- State doctor output unchanged
- The function is `pub` and documented so future diagnostic tools can use it

Implementation:
1. In `src/state.rs`, add after `read_compatibility_events` (around line 3232):

```rust
/// Read state events from a JSONL file, tail-sampling to at most `limit`
/// events when the file is larger. Returns (events, sampling_note) where
/// `sampling_note` is empty when all events fit within the limit.
pub fn read_events_bounded(path: &Path, limit: usize) -> Result<(Vec<Value>, String), String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("read state events '{}': {e}", path.display()))?;
    let total_lines = raw.lines().filter(|l| !l.trim().is_empty()).count();
    if total_lines <= limit {
        let events = read_compatibility_events(path)?;
        Ok((events, String::new()))
    } else {
        // Tail-sample: read only the last `limit` lines
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        let start = lines.len().saturating_sub(limit);
        let mut events = Vec::new();
        for line in &lines[start..] {
            let normalized = compatibility_event_json_line(line)?;
            match serde_json::from_str::<Value>(&normalized) {
                Ok(v) => events.push(v),
                Err(e) => eprintln!("warning: skipping unparseable event: {e}"),
            }
        }
        let note = format!("sampled from last {limit} of {total_lines} total");
        Ok((events, note))
    }
}
```

2. In `src/commands_state.rs`, refactor `handle_doctor` (lines 164-196) to use
   the new helper. The current code does line-count + tail-sampling inline.
   Replace with:
```rust
let (sampled_events, sample_note) = match read_events_bounded(&events_path, limit) {
    Ok((events, note)) => (events, note),
    Err(e) => {
        eprintln!("{RED}  Events: error reading {} — {e}{RESET}", events_path.display());
        (Vec::new(), String::new())
    }
};
```

3. Add a `use` import for `read_events_bounded` from `crate::state` if not
   already imported. Check existing imports in `commands_state.rs`.

4. Keep the existing `read_events` and `read_tail_events` functions in
   `commands_state.rs` unchanged — other callers may depend on them.
