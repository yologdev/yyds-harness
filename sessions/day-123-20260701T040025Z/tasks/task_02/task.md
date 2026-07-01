Title: Fix yyds deepseek cache-report timeout — add event sampling cap
Files: src/commands_deepseek.rs
Issue: #52
Origin: planner

Evidence:
- Assessment §Self-Test Results: `yyds deepseek cache-report` works instantly for default mode but times out with full-history mode because it reads all 64K+ events from `.yoyo/state/events.jsonl` (67.9MB)
- Same read-everything pattern was already fixed in `state doctor` (Day 117), `state crashes` (Day 122), and `eval fixtures score` (Day 122) — each fix added a sampling cap
- Issue #52 was reverted Day 122 due to evaluator timeout, not code quality — the fix pattern is validated across three other commands
- Graph pressure: evaluator_unverified_count=1 — the evaluator needs tasks that verify quickly

Edit Surface:
- src/commands_deepseek.rs (handle_cache_report ~line 1878 and render_cache_report ~line 2026)

Verifier:
- timeout 15 cargo run -- yyds deepseek cache-report
- cargo test --lib commands_deepseek

Fallback:
- If the command already completes within 15s for all modes, write an obsolete note — the fix may have landed in a concurrent session.
- If the timeout is caused by per-event computation (not event volume), add early termination after a fixed number of cache events or a time budget instead of simple sampling.
- If cargo test --lib commands_deepseek takes >30s, reduce scope further — make the change even smaller.

Objective:
Make `yyds deepseek cache-report` complete within 15 seconds for all modes by sampling recent events rather than scanning the entire 64K+ event history.

Why this matters:
Cache behavior is a key DeepSeek reliability signal — cache hits save tokens and money. When cache-report times out, the agent loses visibility into cost efficiency. This is the last of four diagnostic commands with the read-everything timeout bug; fixing it completes the sweep started in Day 117. The fitness gnome affected is `cost_per_successful_task_usd` (indirectly — cache visibility enables cost optimization).

Success Criteria:
- `timeout 15 yyds deepseek cache-report` completes and prints cache statistics
- Full-history mode (if any) also completes within 15s by sampling
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
1. In `src/commands_deepseek.rs`, find where `handle_cache_report` loads events for cache statistics. Add a sampling cap: read only the most recent N events (e.g., 10000 or 20000) when computing cache statistics. Follow the same pattern used in `state doctor` (Day 117) and `state crashes` (Day 122).
2. Add a note in the cache report output like "Statistics from sampled last N events" so users know the report is not from full history.
3. Keep the default mode (basic cache-report without flags) working as-is — it already completes fast. Only apply sampling to modes that would otherwise scan all events.
4. If the command supports a `--full` flag for complete history, preserve that but document the timeout risk.
