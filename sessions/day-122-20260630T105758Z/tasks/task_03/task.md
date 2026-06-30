Title: Fix yyds deepseek cache-report timeout — add event sampling cap
Files: src/commands_deepseek.rs
Issue: none
Origin: planner

Evidence:
- `yyds deepseek cache-report` timed out at 15s during Day 122 preflight
  self-test (Assessment §Self-Test Results, line 35).
- The command reads the full 63K-line `.yoyo/state/events.jsonl` (67.9MB)
  to compute cache statistics.
- The same read-everything pattern was fixed in `state doctor` (Day 117),
  `state crashes` (Day 122), and is being fixed in `state why` (Task 2
  this session).
- `handle_cache_report` at src/commands_deepseek.rs:71 dispatches to
  `render_cache_report` at line 2192, which processes events for cache
  hit/miss statistics.

Edit Surface:
- src/commands_deepseek.rs (handle_cache_report and render_cache_report,
  ~line 71 and ~line 2192)

Verifier:
- timeout 15 cargo run -- yyds deepseek cache-report
- cargo test --lib commands_deepseek

Fallback:
- If cache events are sparse and sampling misses them, add a "sampled N
  events, found M cache events" note in the output.
- If the timeout is caused by per-event computation (not volume), add
  early termination after a fixed number of cache events or a time budget.
- If the command was already fixed by a concurrent session, verify and
  write an obsolete note.

Objective:
Make `yyds deepseek cache-report` complete within 15 seconds by sampling
the most recent events rather than scanning the entire event history.

Why this matters:
Cache behavior is a key DeepSeek reliability signal — cache hits save
tokens and money, cache misses mean repeated work. When `cache-report`
times out, the agent loses visibility into its own cost efficiency. The
trajectory graph pressure calls for bounding failing commands
(bash_tool_error=10). All three timeout bugs (eval score, state why,
cache-report) follow the same read-everything pattern that was already
fixed in two other commands. Completing the sweep makes all diagnostic
commands responsive.

Success Criteria:
- `timeout 15 yyds deepseek cache-report` completes and prints cache
  statistics (hit rate, token savings, etc.)
- Output clearly indicates when results are from a sample vs. full history
- `cargo build && cargo test --lib commands_deepseek` passes

Verification:
- cargo build
- cargo test --lib commands_deepseek
- timeout 15 cargo run -- yyds deepseek cache-report

Expected Evidence:
- `yyds deepseek cache-report` completes within 15 seconds
- Task lineage shows file edits in src/commands_deepseek.rs
- Self-test report shows zero timeouts across all diagnostic commands

Implementation:
1. In `src/commands_deepseek.rs`, find where `render_cache_report` or its
   caller loads events. Add a sampling cap: read only the most recent N
   events (e.g., 10000) when computing cache statistics.
2. Add a note in the output like "Sampled last N events" so users know the
   report is not from the full history.
3. Consider adding a `--full` flag for users who want complete history
   (though this is not required for the timeout fix).
4. Follow the same sampling pattern used in `state doctor` (Day 117) and
   `state crashes` (Day 122) for consistency.
