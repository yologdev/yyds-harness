Title: Refresh CLAUDE_CODE_GAP.md stats and recent additions for Day 64
Files: CLAUDE_CODE_GAP.md
Issue: none

## What

The stats section in CLAUDE_CODE_GAP.md is stale — it says "Day 61" with 48 source files, ~59,794 lines, and 2,305 tests. Actual numbers as of Day 64: 58 source files (was 48 on Day 61!), ~61,436 lines, 2,385 tests (2,297 unit + 88 integration). The planning agent reads this document every session, so stale data means stale plans.

## Changes needed

### 1. Update header dates
- Line 3: Change `Last verified: Day 63 (2026-05-02)` → `Last verified: Day 64 (2026-05-03)`
- Line 4: Add `Day 64` to the "Last updated" list

### 2. Update "## Stats" section (around line 271)
- Change heading from `## Stats (Day 61)` to `## Stats (Day 64)`
- Update line count: `~59,794 lines` → `~61,436 lines`
- Update file count: `48 source files` → `58 source files` (was 48 on Day 61)
- Update test count: `2,305 tests (2,216 unit + 89 integration)` → `2,385 tests (2,297 unit + 88 integration)`
- Update the file list to include all new files since Day 61:
  - `commands_ast_grep.rs` (extracted from commands_search.rs, Day 63)
  - `commands_goal.rs` (extracted, Day 62)
  - `commands_move.rs` (already listed? verify)
  - `commands_plan.rs` (extracted from commands_project.rs, Day 63)
  - `commands_rename.rs` (already listed? verify)
  - `commands_todo.rs` (extracted from commands_project.rs, Day 61)
  - `rtk.rs` (extracted from tools.rs, Day 63)
  - `prompt_retry.rs` (extracted from prompt.rs, Day 64)
  - `prompt_utils.rs` (extracted from prompt.rs, Day 64)
  - `docs.rs` (verify if listed)

### 3. Update "Recent additions" section
Add entries for Days 62-64:
- Real-time subprocess streaming via `on_progress` callback (Day 62)
- `/context files` showing touched files by operation type (Day 62)
- Synthesis skill for multi-source research (Day 62)
- Tool-specific recovery hints in retry prompts (Day 62)
- Non-interactive `yoyo review` for CI pipelines (Day 63)
- `PromptEventState` struct consolidation (Day 63)
- `ReplConfig` struct (Day 63)
- Module extractions: rtk.rs, prompt_retry.rs, prompt_utils.rs, commands_plan.rs, commands_ast_grep.rs (Days 63-64)
- Flaky test fix for destructive_guard CWD race (Day 64)

### 4. Verify and update feature table entries
- Confirm the "Tool output streaming" row reflects Day 62's real-time streaming
- Add `/context files` to the Context section if not already there
- Add non-interactive review to the Code review row

### 5. Get actual counts
Run these commands to verify:
```bash
find src/ -name '*.rs' | wc -l    # source files
cat src/*.rs src/format/*.rs | wc -l  # total lines
cargo test 2>&1 | grep "test result"  # test counts
```

## Approach
- Read the current file, make targeted edits to the stats section, recent additions, and any stale feature table entries
- Do NOT restructure the document or rewrite sections that are already accurate
- Keep the same format and style
