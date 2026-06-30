Title: Fix yyds state why last-failure timeout — add event sampling cap
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- `yyds state why last-failure` timed out at 15s during Day 122 preflight
  self-test (Assessment §Self-Test Results, line 34).
- The command reads the full 63K-line `.yoyo/state/events.jsonl` (67.9MB)
  without any sampling or limit.
- The same read-everything pattern was already fixed in `state doctor`
  (Day 117) and `state crashes` (Day 122) by adding sampling caps.
- `build_why_report` at src/commands_state.rs:15756 scans all events to find
  a single failure event. It doesn't need the entire history — the last
  few thousand events (or a time window) is sufficient.

Edit Surface:
- src/commands_state.rs (build_why_report and surrounding handler, ~line 1148
  and ~line 15756)

Verifier:
- timeout 15 cargo run -- yyds state why last-failure
- cargo test --lib commands_state

Fallback:
- If no failure exists in the sampled window, print "No failure found in
  recent events (sampled last N events). Use --full to scan all history."
- If the timeout is caused by something other than event volume (e.g.,
  slow SQLite queries in a dependency), write an obsolete note with the
  actual bottleneck.

Objective:
Make `yyds state why last-failure` complete within 15 seconds by sampling
the most recent events rather than scanning the entire event history.

Why this matters:
`state why` is a key diagnostic command for understanding why sessions fail.
When it times out, the agent loses its primary self-diagnostic tool. The
trajectory graph pressure calls for "bound failing shell commands before
retrying" (bash_tool_error=10) — this is one of the commands in that class.
The fix follows the established pattern from state doctor (Day 117) and state
crashes (Day 122): add a sampling cap to event reads.

Success Criteria:
- `timeout 15 yyds state why last-failure` completes and prints diagnostic output
- When no failure exists in sampled events, prints a clear message (not a
  cryptic error)
- `cargo build && cargo test --lib commands_state` passes
- Existing `yyds state why <event-id>` for specific events still works

Verification:
- cargo build
- cargo test --lib commands_state
- timeout 15 cargo run -- yyds state why last-failure
- cargo run -- yyds state why evt-promote  (specific event, should still work)

Expected Evidence:
- `yyds state why last-failure` completes within 15 seconds
- Task lineage shows file edits in src/commands_state.rs
- Self-test report shows one fewer timeout in the diagnostic command list

Implementation:
1. In `src/commands_state.rs`, find where `build_why_report` loads events.
   Add a sampling cap: read only the most recent N events (e.g., 5000 or
   10000) when searching for "last-failure". The sampling logic should
   mirror what was done for `state doctor` (Day 117) and `state crashes`
   (Day 122).
2. If no failure event is found in the sampled window, print a clear message
   like "No failure found in the most recent N events." rather than timing
   out or producing confusing output.
3. Ensure `yyds state why <specific-event-id>` still works by reading the
   full event stream when a concrete event ID is given (not "last-failure").
