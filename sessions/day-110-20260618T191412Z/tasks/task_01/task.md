Title: Fix `state summary` empty when SQLite projection has event data
Files: src/commands_state.rs
Issue: none
Origin: planner

Evidence:
- Phase A1 self-test (Day 110, 2026-06-18 19:14): `state summary` returned "empty (no events recorded yet)" while `state failures --recent` showed 12 failures and `state evals` showed 19 log-feedback evals. Both `state failures` and `state evals` read from the SQLite projection; `state summary` only reads from the raw events.jsonl file.
- `handle_state_summary` (line 1166) calls `read_tail_events(&path, limit)` on `default_events_path()`. If events.jsonl is missing or empty, it prints "no state log found" and returns — never consulting the SQLite projection that other state commands use.
- Assessment Notable: "The `state summary` command reported 'empty (no events recorded yet)' while `state failures --recent` and `state evals` showed data. The events are there (from a prior session's SQLite projection) but the summary path isn't finding them."

Edit Surface:
- src/commands_state.rs

Verifier:
- cargo test commands_state state -- --test-threads=1
- Manual: in an env where events.jsonl is missing but .yoyo/state/projection.sqlite exists from prior CI, `yyds state summary` should show data from SQLite instead of "empty"

Fallback:
- If `state summary` already falls back to SQLite when events.jsonl is missing (check the code, not assumption), mark this task obsolete.
- If the SQLite projection also has no useful summary data, note the limitation and close: the fix is still correct to attempt fallback, but the output may remain thin.

Objective:
Make `yyds state summary` produce useful output when the raw events.jsonl is missing or empty but the SQLite projection contains event data from prior sessions.

Why this matters:
The `state summary` command is the first diagnostic a user (or the assessment agent) runs to understand harness state health. When it says "empty" while other state commands show real data, it creates confusion and hides usable diagnostic information. The SQLite projection is the canonical query surface for state; `summary` should use it as a fallback.

Success Criteria:
- `state summary` returns meaningful data when events.jsonl is missing but SQLite projection exists
- When both events.jsonl and SQLite are available, events.jsonl is preferred (no regression)
- When neither exists, the existing "empty" or "no state log" message is unchanged
- The fallback also works for `state why <id>` when `--summary` is passed

Verification:
- cargo build && cargo test commands_state state -- --test-threads=1
- cargo clippy --all-targets -- -D warnings (on changed code only)
- Manual smoke: run `yyds state summary` after removing events.jsonl but keeping projection.sqlite

Expected Evidence:
- Assessment self-test: `state summary` shows useful data instead of "empty" when SQLite has events
- State event: FileEdited on src/commands_state.rs
- No regression in existing `state summary` behavior when events.jsonl is present

Implementation Notes:
- `handle_state_summary` (line 1166) currently calls `read_tail_events` → `build_state_summary`. If the events file can't be read, it prints "no state log found" and returns.
- Add a fallback path: when `read_tail_events` fails, try reading from the SQLite projection. The projection path is likely `state_sqlite_projection_path()` or similar in `state.rs`. Query for recent events that can populate a summary.
- `handle_why` (line 969) also uses events.jsonl for `--summary` mode and may benefit from the same fallback.
- Keep the change minimal: add the fallback, don't restructure the entire state subsystem. A helper function like `read_events_with_sqlite_fallback(path, limit)` that tries events.jsonl first, then SQLite, would be clean.
- The SQLite projection functions are in `src/state.rs` (e.g., `rebuild_sqlite_projection`, `query_sqlite_relations`). Use an existing query path rather than adding new SQL.
