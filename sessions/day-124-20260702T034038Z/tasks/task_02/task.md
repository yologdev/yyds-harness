Title: Fix `yyds deepseek cache-report` timeout — add event sampling cap
Files: src/commands_deepseek.rs
Issue: #52
Origin: planner

Evidence:
- Assessment §Self-Test: `yyds deepseek cache-report` timed out at 15s during Day 122 preflight
- Same read-everything pattern already fixed in `state doctor` (Day 117), `state crashes` (Day 122), and `eval fixtures score` (Day 122)
- `handle_cache_report` at src/commands_deepseek.rs:1878 reads the full 63K-line events file (67.9MB) to compute cache statistics
- Graph pressure: bash_tool_error=10, calling for bounded shell commands
- This is issue #52 — reverted because evaluator timed out; implementation was likely correct but unverified

Edit Surface:
- src/commands_deepseek.rs (handle_cache_report at ~line 1878; follow the sampling pattern from state crashes and eval fixtures score)

Verifier:
- timeout 15 cargo run -- yyds deepseek cache-report
- cargo test --lib commands_deepseek

Fallback:
- If cache events are sparse and sampling misses them, add a "sampled N events, found M cache events" note in the output.
- If the timeout is caused by per-event computation, add early termination after a fixed number of cache events.
- If the command was already fixed by a concurrent session, verify and write an obsolete note.

Objective:
Make `yyds deepseek cache-report` complete within 15 seconds by sampling the most recent events rather than scanning the entire event history.

Why this matters:
Cache behavior is a key DeepSeek reliability signal — cache hits save tokens and money, cache misses mean repeated work. When `cache-report` times out, the agent loses visibility into its own cost efficiency. The trajectory shows `session_capture_coverage` as a diagnostic gate; cache-report is how we measure it. This is the third command in the same timeout class (state why, eval score, cache-report) — completing this sweep makes all diagnostic commands responsive. Each one that stays broken is a blind spot in the self-diagnostic surface.

Success Criteria:
- `timeout 15 cargo run -- yyds deepseek cache-report` completes and prints cache statistics (hit rate, token savings)
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
- Cache hit ratio remains visible and accurate (within sampling error)

Implementation:
1. In `src/commands_deepseek.rs`, `handle_cache_report` at ~line 1878 loads events. Add a sampling cap: read only the most recent N events (e.g., 5000-10000) when computing cache statistics.
2. The events file is append-only — most recent events are at the end. Read backwards or use a sliding window.
3. Follow the same sampling pattern used in `state doctor` (Day 117) and `state crashes` (Day 122) for consistency. The pattern: determine total event count, compute offset = max(0, total - sample_size), skip to offset, process remaining.
4. Add a note in the output like "Sampled last N events" so consumers know the report is not from the full history.
5. Keep the change to ~30 lines or fewer. This is a focused fix of a known pattern.
