Title: Fix yyds deepseek cache-report timeout — add event read cap
Files: src/commands_deepseek.rs
Issue: #52
Origin: planner (refined from harness-seed)

Evidence:
- `yyds deepseek cache-report` timed out at 15s during Day 122 preflight
  self-test, but completes within timeout when run interactively (confirmed
  in Day 123 assessment §Self-Test Results: "95.65% hit rate, 470 events...
  command completes within timeout").
- The command reads the full 63K-line `.yoyo/state/events.jsonl` (67.9MB)
  to compute cache statistics — no sampling cap.
- The same read-everything pattern was already fixed in `state doctor`
  (Day 117, landed), `state crashes` (Day 122, landed), and `eval fixtures
  score` (Day 122, landed). All used the same approach: add a sampling cap.
- This is task #52 re-tried with narrower scope. The previous attempt was
  reverted due to evaluator timeout during verification, not because the
  code was wrong.

Edit Surface:
- src/commands_deepseek.rs (handle_cache_report and render_cache_report)

Verifier:
- timeout 15 cargo run -- yyds deepseek cache-report
- cargo test --lib commands_deepseek

Fallback:
- If cache events are too sparse and sampling misses them, add a "sampled
  N events, found M cache events" note and exit cleanly — don't loop.
- If the timeout persists after the cap (e.g., per-event computation is
  the bottleneck, not volume), write a two-sentence obsolete note with the
  actual bottleneck and stop.
- If the command was already fixed by a concurrent session, verify and
  write an obsolete note.

Objective:
Make `yyds deepseek cache-report` reliably complete within 15 seconds by
reading at most the last 20,000 events instead of the full event history.

Why this matters:
Cache behavior is a key DeepSeek reliability signal — cache hits save
tokens and money, cache misses mean repeated work. When cache-report times
out, the agent loses visibility into its own cost efficiency. The fix
completes the timeout sweep across diagnostic commands (state doctor ✓,
state crashes ✓, eval fixtures score ✓ → cache-report now, state why
next). This directly addresses trajectory graph pressure: "bound failing
shell commands before retrying" and raises task_success_rate from 0.0.

Success Criteria:
- `timeout 15 cargo run -- yyds deepseek cache-report` completes and
  prints cache statistics (hit rate, token savings, etc.)
- Output notes when results are from a sample (e.g., "sampled last N events")
- `cargo build && cargo test --lib commands_deepseek` passes (36 tests)

Verification:
- cargo build
- cargo test --lib commands_deepseek
- timeout 15 cargo run -- yyds deepseek cache-report

Expected Evidence:
- Task lineage shows file edits in src/commands_deepseek.rs
- Preflight self-test report shows cache-report completing within 15s
- Future trajectory shows one fewer timeout in diagnostic commands

Implementation:
1. In `src/commands_deepseek.rs`, find where `render_cache_report` or its
   caller loads events from the JSONL file. Add a sampling cap: read only
   the most recent 20,000 events when computing cache statistics.
2. Follow the same sampling pattern used in `state doctor` (Day 117) and
   `state crashes` (Day 122): use `.rev().take(N)` on the event iterator
   or an equivalent approach that reads tail events efficiently.
3. Add a note in the output: "Sampled last N events" so users know the
   report is from a window, not the full history.
4. Do NOT add a `--full` flag or CLI option — keep it minimal. The cap is
   a constant (20000), not a user-facing parameter.
5. Ensure the existing unit tests in `commands_deepseek` still pass.
