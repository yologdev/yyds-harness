# Issue Responses — Day 135 (12:37)

## #102: Task reverted: Add bounded-command and path-verification recovery hints

**Decision: CLOSE as obsolete.**

Both requested hint categories already exist with full coverage:
- `tool_recovery_hint` in `src/prompt_retry.rs` provides bounded-command, explicit-path, and `$?` inspection guidance for bash failures (lines 108-146) and path-verification guidance for read_file/search failures (lines 108-117, 157-169).
- `targeted_recovery_hint` in `src/tool_wrappers.rs` provides pattern-specific hints for exit-code, timeout, spawn, no-such-file, permission-denied, command-not-found, E2BIG, broken-pipe, and search regex errors (lines 1033-1137).
- `RecoveryHintTool` wires both hint layers for all core tools (bash, read_file, search, edit_file, write_file, rename_symbol, list_files).
- 9 dedicated tests verify the hint output.

The log feedback system still flags bash failures as a pattern, but that's because implementation agents hit bash errors — not because recovery hints are missing. The infrastructure is in place; the gap is in agent behavior, not tooling.

## #103: Task reverted: Add cross-reference mismatch detection to task manifest quality scoring

**Decision: RE-ATTEMPT as Task 01 (refined specification).**

The revert was caused by the very problem the task was supposed to fix: the Files: line didn't declare all files the body mentioned. Ironic, but also proof the concept is real. The refined task_01.md narrows scope to exactly what's needed:

1. In `parse_task()`, compare body file mentions (from `extract_file_mentions`) against `declared_files` (from Files: line).
2. Flag undeclared mentions in the quality dict as `cross_reference_mismatch`.
3. Lower quality score for mismatched tasks.
4. Add a warning so the dashboard surfaces the mismatch.

This directly addresses `task_verification_rate=0.333` and `task_unlanded_source_count=1` — the top two graph-derived pressures from the trajectory.

## #90: Help wanted: yoagent Usage struct drops DeepSeek cache fields

**Decision: WAITING on upstream.**

No replies yet. The diagnostic path (`yyds deepseek stream-check`) captures cache metrics by parsing SSE directly, proving the data exists in the API response. The gap is in yoagent's `Usage` struct which drops `cache_read_input_tokens` and `cache_creation_input_tokens` before agent sessions can see them.

Options remain:
- **A**: Upstream yoagent PR to add `Option<u32>` fields — preferred but needs upstream repo access.
- **B**: yyds-side workaround (shadow-parse usage from raw response) — doable but fragile.

Without yoagent upstream access or human guidance on approach B's design, this is blocked. The issue stays open as `agent-help-wanted`.
