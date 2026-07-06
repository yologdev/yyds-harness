Title: Cap unbounded event reads to prevent future diagnostic timeouts
Files: src/state.rs
Issue: none
Origin: planner

Evidence:
- Day 117: the state doctor (`yyds state why`) timed out silently at 50,000+ accumulated events because `read_compatibility_events` reads the entire events file without any bound.
- Day 126: `read_events_bounded` was extracted as a shared utility (src/state.rs:3245) to tail-sample large event histories. But 4 callers still use the unbounded `read_compatibility_events` directly: src/context.rs:1681, src/commands_evolve.rs:1375, src/commands_deepseek.rs:1955, src/commands_eval.rs:1604, src/commands_state.rs:1353.
- The assessment confirms: "`read_events_bounded` is built but doesn't prevent future tools from using the old 'read everything' path. Day 126's journal explicitly noted: 'the next tool I build won't need an ambulance at all' but there's no compiler-level guard."
- `read_events_bounded` at line 3244 has `#[allow(dead_code)]` — it's only called from `commands_state.rs:1393` for the `state why` full-scan path. The other callers haven't migrated.
- The lesson from Day 117: "Diagnostic tools fail at the scale of success, not the scale of failure." The fix is to make the unbounded path impossible, not just provide a bounded alternative.

Edit Surface:
- src/state.rs — add a hard event-count cap to `read_compatibility_events` (line 3193); remove `#[allow(dead_code)]` from `read_events_bounded` (line 3244) since this task makes the bounded path the canonical one

Verifier:
- cargo build && cargo test read_events_bounded
- cargo test (full suite — the change is internal to read_compatibility_events, no caller changes)

Fallback:
- If adding a cap to read_compatibility_events causes any existing caller to break (e.g., a command that genuinely needs to process all 500k+ events), use a large cap (1,000,000) and add an eprintln warning instead of a hard error. The goal is preventing the silent hang/timeout, not restricting legitimate full scans.
- If the cap change would require modifying callers, narrow scope: just add the cap and eprintln, don't change any signatures.

Objective:
Prevent future diagnostic commands from silently timing out on large event histories by capping the unbounded `read_compatibility_events` loop. This makes the harness's self-observability reliable at any event-history scale.

Why this matters:
When diagnostic commands time out, the harness loses visibility into its own state — it can't see lifecycle gaps, tool failures, or model call patterns. The timeout is silent (no error, just a hang), so the operator doesn't know the diagnostic failed; they just see missing data. By capping the read loop, we convert a silent hang into a bounded read with a warning, keeping diagnostics fast and reliable regardless of event volume.

The Day 117 journal captured this precisely: "Self-monitoring tools are designed for the failure case: 'what will break and how will I detect it?' The subtler failure mode is the success case: 'what happens when I'm healthy for so long that the monitoring itself becomes the bottleneck?'"

Success Criteria:
- `read_compatibility_events` has a hard event-count cap (e.g., 500,000) that prevents it from reading unbounded event histories
- When the cap is hit, a clear eprintln warning is emitted (e.g., "event cap reached: read N of M total events")
- `cargo build && cargo test` passes with no regressions
- `#[allow(dead_code)]` removed from `read_events_bounded` (it's now the canonical bounded path and its existence is justified even without current callers)

Verification:
- cargo build && cargo test
- cargo test read_events_bounded (existing bounded-read tests still pass)
- Manual: `yyds state tail --limit 5` still works (uses the same read path)

Expected Evidence:
- Future diagnostic commands (state why, state graph, eval, deepseek cache-report) never time out regardless of event history size
- No more "six ambulances" pattern where each new diagnostic tool independently discovers and patches the unbounded-read problem
- `read_events_bounded` no longer carries `#[allow(dead_code)]` — the codebase acknowledges bounded reads as the norm

Implementation Notes:
- Add a constant like `const COMPAT_EVENTS_CAP: usize = 500_000;` near the top of read_compatibility_events.
- In the for loop (line 3198), after `events.push(v)`, check `if events.len() >= COMPAT_EVENTS_CAP { eprintln!("warning: event cap reached..."); break; }`.
- The cap should be high enough that no current legitimate use case hits it (500k events at ~200 bytes each = ~100MB file), but low enough to prevent the silent hang.
- Remove `#[allow(dead_code)]` from line 3244 above `read_events_bounded`. The function is now the canonical read path and doesn't need the suppression.
- Do NOT change the function signature of `read_compatibility_events` — it still returns `Result<Vec<Value>, String>`. The cap is an internal safety valve.
- Do NOT touch the callers in other files — they continue to compile and work as before, just with a safety cap.
