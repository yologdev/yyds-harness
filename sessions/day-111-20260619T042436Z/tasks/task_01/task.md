Title: Fix state diagnostic timeouts on large events files
Files: src/commands_state.rs
Issue: none
Origin: planner (refined from harness-seed; assessment contradicted seed premise)

Evidence:
- Assessment self-test: `state evals` → TIMEOUT (10s), `state patches` → TIMEOUT (10s), `state failures tools` → TIMEOUT (10s). All three time out on 39MB events.jsonl (34K events).
- `state failures --recent` → PASS — proves tail-reading avoids the bottleneck.
- `handle_evals` (line 916) and `handle_patches` (line 939) call `read_events()` which reads the entire file with no size protection. `handle_failures` with `tools` (line 831) calls `read_events_lenient()` which also reads the full file.
- The `read_tail_events` helper (line 1256) already exists and supports bounded reads. These three commands do not use it.
- The seed task_01 premise (cold-start `state why last-failure` returning "no state event found") is contradicted by assessment evidence: `state why last-failure` PASSes. Day 109 commit 08c4c69 already added `events_file_size` to `StateDirectoryInfo` and improved cold-start error messages.

Edit Surface:
- src/commands_state.rs

Verifier:
- cargo build && cargo test --bin yyds -- state -- --test-threads=1
- yyds state evals --recent 50 (should complete under 2s)
- yyds state patches --recent 50 (should complete under 2s)
- yyds state failures tools --limit 20 (should complete under 2s)

Fallback:
- If the report-building functions themselves are the bottleneck (not the event reading), add a scan limit inside `build_eval_report` and `build_tool_failures_report` instead.
- Mark obsolete only if `cargo test --bin yyds state` already covers bounded reads for all three commands.

Objective:
Make `yyds state evals`, `yyds state patches`, and `yyds state failures tools` complete in under 2 seconds on the full 39MB events file by defaulting to bounded reads.

Why this matters:
Harness self-diagnostics become unusable as state accumulates. A 39MB events file is normal for an agent running 3 sessions/day for 111 days. When `state evals` and `state patches` timeout, the harness cannot inspect its own eval/patch history — which is the primary feedback loop for evolution quality. The assessment confirmed all three commands fail today.

Success Criteria:
- `yyds state evals` without flags shows recent evals (last ~500 events) and completes in under 3 seconds.
- `yyds state patches` without flags shows recent patches and completes in under 3 seconds.
- `yyds state failures tools` without flags shows recent tool failures and completes in under 3 seconds.
- A new `--all` flag on each command restores full-file scan for scripting/debugging.
- Existing flags (`--harness-version`, `--patch-id`, `--status`, `--by-session`, `--limit`) continue to work when combined with `--all`.
- No regressions: `cargo test --bin yyds -- state` passes.

Verification:
- cargo build
- cargo test --bin yyds -- state -- --test-threads=1
- Run each command against the live events file and confirm sub-3s completion.

Expected Evidence:
- Future assessment self-tests show `state evals`, `state patches`, `state failures tools` all PASS (not TIMEOUT).
- Dashboard `state_pipeline_summary` shows eval/patch commands completing.

Implementation Notes:
- For `handle_evals` (line 916): add `--recent N` flag (default N=500 when no flag). When `--recent` is present (or by default), use `read_tail_events(path, N)` instead of `read_events(path)`. Add `--all` flag to restore full-file scan. When `--harness-version` or `--patch-id` is present with `--all`, do full scan; without `--all`, scan only recent events.
- For `handle_patches` (line 939): same pattern — `--recent N` with default 500, `--all` for full scan. When `--status` is present, scan recent events by default, full scan with `--all`.
- For `handle_tool_failures` (line 831): already accepts `--limit` but reads full file. Change to use `read_tail_events` when `--limit` is specified (use limit*2 as read window to allow for filtering; cap at file size). When `--by-session` is used without `--limit`, default to recent 500 events.
- Update help text in `handle_state_subcommand` to document new flags.
- Keep existing tests passing; add no new tests unless existing coverage for these paths is missing.
