Title: Add unit tests for tools.rs — StreamingBashTool, RenameSymbolTool, build_tools coverage
Files: src/tools.rs
Issue: none

## Problem

`tools.rs` has the lowest test ratio of any large file: 1,987 lines with only 29 tests (1.4 per 100 lines). This file defines every tool the agent can use — `StreamingBashTool`, `RenameSymbolTool`, `AskUserTool`, `TodoTool`, and `build_tools`. These are structurally critical. The existing tests likely cover `TodoTool` and basic helpers, leaving the bash tool, rename tool, and toolbox assembly under-tested.

## What to test

Focus on unit-testable aspects that don't require a live agent:

### StreamingBashTool
- Tool name, description, and parameter schema are correct
- `working_dir` is applied (construct with a temp dir, verify it's set)
- Confirm-mode flag is stored correctly
- Tool input validation (missing "command" key returns error)

### RenameSymbolTool  
- Tool name, description, and parameter schema are correct
- Input parsing: old_name, new_name, optional path parameter
- Error cases: missing required parameters

### build_tools
- Returns the expected number of tools
- All expected tool names are present in the returned set
- Tool names don't have duplicates
- `build_sub_agent_tool` returns both a tool and a shared state handle

### AskUserTool
- Tool name and description
- Parameter schema has "question" field

### TodoTool
- Any gaps in existing TodoTool tests (check what's already covered and add edge cases)

Target: 15-25 new tests. Each test should be self-contained with no filesystem side effects on the real repo (use temp dirs where needed).

## Verification

```bash
cargo test --lib tools
cargo test  # full suite
cargo clippy --all-targets -- -D warnings
```
