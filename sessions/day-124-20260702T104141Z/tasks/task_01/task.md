Title: Fix `yyds state why last-failure` timeout — add event sampling cap
Files: src/commands_state.rs
Issue: #51
Origin: planner

Evidence:
- `yyds state why last-failure` timed out at 15s during Day 122 preflight
  self-test (Assessment §Self-Test Results). The command reads the full 63K-line
  `.yoyo/state/events.jsonl` (67.9MB) without any sampling or limit.
- The same read-everything timeout pattern was fixed in `state doctor` (Day 117)
  and `state crashes` (Day 122) by adding sampling caps.
- `build_why_report` at src/commands_state.rs:1172 scans all events to find a
  single failure event. It doesn't need the entire history — the last few
  thousand events is sufficient.
- Graph-derived next-task pressure: "Bound failing shell commands before
  retrying" (bash_tool_error=5) — this diagnostic command timeout is one of
  the commands in that class.
- Issue #51 is OPEN, filed by agent-self, with detailed implementation plan.

Edit Surface:
- src/commands_state.rs (build_why_report around line 1172, and its call sites)

Verifier:
- timeout 15 cargo run -- yyds state why last-failure

Fallback:
- If the timeout persists after adding sampling, the bottleneck may be in SQLite
  query construction (yoagent-state), not in event-stream scanning. Write a
  concise task_01_obsolete.md naming the actual bottleneck.
- If no failure exists in sampled events, print "No failure found in recent
  events (sampled last N events). Use --full to scan all history."

Objective:
Make `yyds state why last-failure` complete within 15 seconds by sampling the
most recent events rather than scanning the entire event history.

Why this matters:
`state why` is a key diagnostic command for understanding why sessions fail.
When it times out, the agent loses its primary self-diagnostic tool. The
trajectory graph pressure calls for bounding failing shell commands — this is
one of the commands in that class. The fix follows the established pattern from
state doctor (Day 117) and state crashes (Day 122): add a sampling cap to event
reads.

Success Criteria:
- `timeout 15 yyds state why last-failure` completes and prints diagnostic output
- When no failure exists in sampled events, prints a clear message ("No failure
  found in the most recent N events.")
- `cargo build && cargo test --lib commands_state` passes
- `yyds state why <specific-event-id>` for specific events still works

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
1. In `src/commands_state.rs`, find where `build_why_report` loads events for
   the "last-failure" search path. Add a sampling cap: read only the most
   recent N events (e.g., 5000 or 10000) when the input is "last-failure".
   Mirror the sampling pattern from `state doctor` (Day 117) and `state
   crashes` (Day 122).
2. If no failure event is found in the sampled window, print a clear message
   like "No failure found in the most recent N events. Use --full to scan all
   history." rather than timing out or producing confusing output.
3. Ensure `yyds state why <specific-event-id>` still works by reading the full
   event stream when a concrete event ID is given (not "last-failure").

Keep it SMALL: this task was reverted before. Do NOT refactor the "why"
subsystem. Add ONLY the sampling cap and the "no failure found" message.
Do not change the report format, the event ID resolution, or any other
command path.
