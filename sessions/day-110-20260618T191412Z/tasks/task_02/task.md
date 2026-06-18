Title: Fix `deepseek cache-report` "no state log" when SQLite projection has data
Files: src/commands_deepseek.rs
Issue: none
Origin: planner

Evidence:
- Phase A1 self-test (Day 110, 2026-06-18 19:14): `deepseek cache-report` returned "no state log found at .yoyo/state/events.jsonl" while `state failures --recent` and `state evals` showed data from the SQLite projection.
- `handle_cache_report` (src/commands_deepseek.rs line 1878) calls `read_events(&path)` on `default_events_path()`, then bails with "no state log found" if the file can't be read — never consulting the SQLite projection.
- `read_events` (line 1895) uses `crate::state::read_compatibility_events` which only reads the raw events.jsonl file.
- The graph-based cache report (`build_graph_cache_report` in commands_state.rs line 9414) already reads from SQLite. The `deepseek cache-report` CLI command should have the same fallback.
- Assessment: "`deepseek cache-report`: returns 'no state log' despite events.jsonl existing in `.yoyo/state/`" — the assessment reveals this command is fragile even when the file exists (may be a stale path or format issue).

Edit Surface:
- src/commands_deepseek.rs

Verifier:
- cargo test commands_deepseek cache -- --test-threads=1
- Manual: in an env where events.jsonl is missing but .yoyo/state/projection.sqlite exists, `yyds deepseek cache-report` should show data instead of "no state log found"

Fallback:
- If `handle_cache_report` already has a SQLite fallback path (check the code), mark this task obsolete.
- If the SQLite projection has no cache metrics events, the report will show "no cache metrics recorded" — that's acceptable; the fix is about the fallback path, not about populating missing cache data.

Objective:
Make `yyds deepseek cache-report` fall back to the SQLite projection when the raw events.jsonl file is unavailable, so cache metrics remain queryable across session boundaries and CI env resets.

Why this matters:
Cache metrics are a critical DeepSeek harness KPI — they show whether the deterministic prompt layout and cache-stable prefix strategy is saving tokens. When `cache-report` breaks because events.jsonl was cleaned up but SQLite persists, the harness loses visibility into its primary cost-saving mechanism. The SQLite projection is designed to be the durable query surface; cache-report should use it.

Success Criteria:
- `deepseek cache-report` falls back to SQLite when events.jsonl is unavailable
- When events.jsonl IS available, existing behavior is unchanged (no regression)
- `--json` output format still works with the fallback
- The `build_cache_report` function still works with events from either source

Verification:
- cargo build && cargo test commands_deepseek cache -- --test-threads=1
- cargo clippy --all-targets -- -D warnings
- Manual: run `yyds deepseek cache-report` after removing events.jsonl but keeping SQLite

Expected Evidence:
- Assessment self-test: `deepseek cache-report` shows cache data instead of "no state log found"
- State event: FileEdited on src/commands_deepseek.rs
- The `build_graph_cache_report` already uses SQLite; this task aligns the CLI `cache-report` path with the same data source

Implementation Notes:
- `handle_cache_report` at line 1878 has its own local `read_events` (line 1895) that only reads events.jsonl. Add a SQLite fallback.
- The simplest approach: after `read_events(&path)` fails, try `crate::state::query_sqlite_relations` or similar to pull cache-related events from the projection. Alternatively, extract `read_events_with_fallback` as a shared helper that both `handle_cache_report` and `handle_state_summary` can use — but keep the change scoped to commands_deepseek.rs unless the shared helper lives in state.rs.
- If adding a shared helper to state.rs, update Files: to include src/state.rs. Prefer keeping the fallback inline in commands_deepseek.rs to stay within the 1-file scope.
- The `build_cache_report` function (line 1899) consumes `&[Value]` events and filters for `CacheMetricsRecorded`. It works the same regardless of whether events came from JSONL or SQLite.
- Do NOT change `build_cache_report`'s logic — only change how events are sourced.
