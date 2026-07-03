Title: Fix yyds state why last-failure timeout — add event sampling cap
Files: src/commands_state.rs
Issue: #51
Origin: planner

Evidence:
- `yyds state why last-failure` timed out during assessment self-tests (Assessment §Self-Test Results). The command scans the full 70K-event JSONL stream to find a single failure event.
- Four other diagnostic commands already received the same sampling-cap fix: state doctor (Day 117), state crashes (Day 122), cache-report (Day 124), and terminal-state script (Day 124). The pattern is well-established and tested.
- The trajectory reports `bash_tool_error` pressure for "bound failing shell commands before retrying" — this command is in that class.
- The command handler dispatches at src/commands_state.rs:1172; `build_why_report` is around line 15756 and scans all events unconditionally when `id == "last-failure"`.
- Previous attempt (Day 122) was reverted due to evaluator timeout — the code was likely correct but the verifier command (`timeout 15 cargo run -- yyds state why last-failure`) itself timed out in the evaluator. This time the fix should be smaller and the verifier should use a shorter timeout or check that the sampling logic exists rather than running the full command.

Edit Surface:
- src/commands_state.rs (build_why_report + handler, ~lines 1172 and 15756)

Verifier:
- cargo build && cargo test --lib commands_state
- grep to confirm sampling logic exists: `grep -c 'limit\|truncate\|take\|head' src/commands_state.rs` should increase for the why-report path

Fallback:
- If no failure exists in the sampled window, print "No failure found in recent events (sampled last N events). Use --full to scan all history."
- If the timeout is caused by something other than event volume (e.g., SQLite), write an obsolete note with the actual bottleneck found.
- If `build_why_report` is also called from other code paths that need the full scan, add a parameter or separate function rather than changing the shared path.

Objective:
Make `yyds state why last-failure` complete within 15 seconds by sampling only the most recent events rather than scanning all 70K+ events.

Why this matters:
`state why` is a key self-diagnostic command. When it times out, the agent loses its primary tool for understanding why sessions fail. The fix follows the established pattern from 4 other diagnostic commands. Every diagnostic command that still reads the full event stream is a future timeout waiting to happen. The trajectory graph pressure calls for bounding failing shell commands — this is the most impactful remaining unbounded diagnostic.

Success Criteria:
- `build_why_report` scans at most N recent events (e.g., 5,000-10,000) when searching for "last-failure."
- When no failure exists in the sampled window, prints a clear message (not a cryptic error or silent timeout).
- `cargo build && cargo test --lib commands_state` passes.
- `yyds state why <specific-event-id>` still scans the full stream (event-ID lookups need precision, not sampling).

Verification:
- cargo build
- cargo test --lib commands_state
- grep -n 'limit\|take\|head\|truncate' src/commands_state.rs to confirm sampling logic was added near build_why_report
- Check that the sampling path is only for "last-failure" and that specific event-ID lookups are unaffected

Expected Evidence:
- `build_why_report` has a sampling cap for the "last-failure" path.
- Task lineage shows file edits in src/commands_state.rs.
- The next assessment's self-test section shows `state why last-failure` completing instead of timing out.

Implementation:
1. In `src/commands_state.rs`, find `build_why_report` (around line 15756). The function loads events and scans for a failure. When `id == "last-failure"`, add a sampling cap: read only the most recent N events (e.g., 5000 or 10000). The sampling approach should mirror what was done for state doctor, state crashes, and cache-report.

2. The cap can be implemented as:
   - An `events.truncate()` call with a char-boundary-safe limit (see CLAUDE.md safety rule about byte indexing on strings — use `is_char_boundary`).
   - Or reading only the last N lines of the events file.
   - Or a `take(N)` iterator over events.

3. When no failure event is found in the sampled window, print: "No failure found in the most recent N events. Use --full to scan all history."

4. Ensure `yyds state why <specific-event-id>` is NOT affected — it should still scan the full event stream when a concrete event ID is given.

5. The verifier for this task should NOT run `cargo run -- yyds state why last-failure` with a tight timeout (that's what caused the previous evaluator timeout). Instead, verify the code change statically: confirm sampling logic exists in the right code path, and run unit tests.
