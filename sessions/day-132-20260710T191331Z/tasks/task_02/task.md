Title: Add progress feedback to state why bounded reads
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Assessment self-test: `timeout 30 ./target/debug/yyds state why last-failure`
  timed out (exit 124); `timeout 10` also timed out. The command eventually
  succeeds but takes 10-30s with zero user feedback.
- `read_events_bounded` at src/state.rs:3267 does `std::fs::read_to_string(path)`
  which reads the ENTIRE events file into memory before counting lines. A
  100K-line JSONL file can be tens of MB. The read blocks for seconds with no
  output — the user sees a hung command, not a working one.
- The Day 132 fix (commit 41a4006) capped full-scan to 100K events but did not
  address the read-time silence. The assessment's self-test still shows 10-30s
  waits: "reading 10K JSONL lines apparently takes 10-30s. Not a correctness
  bug, but a UX friction: the first thing a diagnostic user runs shouldn't feel
  broken."
- Assessment bug #1 (MEDIUM): "state why last-failure still slow — Profile
  read_events_bounded for 10K events; optimize JSONL parsing or add a progress
  indicator."

Edit Surface:
- src/state.rs (read_events_bounded function, lines 3266-3288)

Verifier:
- cargo build && timeout 30 ./target/debug/yyds state why last-failure 2>&1
  should show a progress message on stderr before the final output.
- cargo test read_events_bounded -- --test-threads=1

Fallback:
- If the events file is small (< limit) and the read is fast anyway, the
  progress message may not appear. That's fine — the code path works correctly
  for small files. The fix should only emit progress when the total_lines >
  limit (i.e., when sampling is happening).
- If the build fails, revert with `git checkout -- src/state.rs`.

Objective:
Make `yyds state why` feel responsive by emitting a progress indicator before
blocking on large event-file reads, so the user knows the command is working
rather than hung.

Why this matters:
The first diagnostic command a user runs (`state why last-failure`) currently
blocks silently for 10-30 seconds. This breaks the trust loop: "is it working or
is it hung?" A one-line stderr message ("reading state events...") converts an
anxiety-inducing silence into a predictable wait. This is a small code change
that has outsized UX impact for anyone debugging harness state.

Success Criteria:
- `yyds state why last-failure` emits a progress message to stderr before the
  blocking read when the events file exceeds the limit.
- The message is not shown when the file is small enough to read entirely
  (avoid noise in the common fast-path case).
- Existing tests pass: `cargo test read_events_bounded -- --test-threads=1`.

Verification:
- cargo build
- timeout 30 ./target/debug/yyds state why last-failure 2>&1 | head -5
  should show something like "reading state events (sampling last N of M lines)..."
- cargo test -- --test-threads=1
- cargo fmt -- --check

Expected Evidence:
- `state why last-failure` no longer feels hung — user sees immediate feedback.
- Next assessment self-test no longer reports timeout/slowness as a MEDIUM bug.

Implementation Notes:
- In `read_events_bounded` (src/state.rs:3266-3288), after counting `total_lines`
  and determining that `total_lines > limit` (line 3273), emit an `eprintln!`
  before collecting lines and parsing JSONL.
- The message should be concise: `eprintln!("reading {} state events (sampling last {} of {} lines)...", limit, limit, total_lines);`
- If `total_lines <= limit`, no progress message — the fast path doesn't need it.
- The eprintln goes to stderr, so it won't interfere with stdout output parsing
  by scripts or the harness.
- The `lines: Vec<&str>` collection at line 3275 reads all lines into a Vec
  before slicing. This is the main bottleneck. Adding eprintln before this
  line makes the wait predictable; a follow-up optimization (not in this task)
  could use a buffered reader with reverse-line seeking for true tail-reads.
- Keep the change minimal: add the eprintln, don't restructure the read loop.
- Update CLAUDE.md if this changes the `state why` behavior description
  (unlikely — this is an internal UX polish, not a feature change).
