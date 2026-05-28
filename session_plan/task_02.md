Title: Redesign /todo board to use session_plan/* as source of truth
Files: src/commands_todo.rs, src/help_data.rs
Issue: #433

## Problem

The `/todo board` subsystem (introduced Day 86) manages tasks through a standalone `TODO.md`
file with Markdown-based Kanban sections. This creates a separate task model that conflicts
with the existing `session_plan/task_*.md` task handling used by the evolution pipeline.

Community feedback (issue #433, @voku) correctly identifies:
1. `TODO.md` should NOT be a separate task source — `session_plan/*` files are the truth
2. Title-based task matching is fragile (e.g., "Fix parser" vs "Fix parser edge cases")
3. The board should be a VIEW of task state, not the DATABASE

## Design

**Source of truth:** `session_plan/task_*.md` files. Each file already has a structured format:
```
Title: [task title]
Files: [files to modify]
Issue: #N (or "none")

[description]
```

**Task IDs:** File-based — `task_01`, `task_02`, etc. (derived from filename).

**Task status:** Add a `Status:` line to task files. Valid values: `backlog`, `active`, `done`.
If no Status line exists, default to `backlog`.

**Commands (revised):**
- `/todo board` — Read `session_plan/task_*.md`, render a Kanban view (backlog / active / done)
- `/todo board init [goal]` — Create `session_plan/` dir with a `goal.md` file (NOT TODO.md)
- `/todo board add <title>` — Create next `session_plan/task_NN.md` with Status: backlog
- `/todo board move <task_id> <status>` — Update the Status line in the task file
  Example: `/todo board move task_02 active`
- `/todo board done <task_id>` — Set Status: done in the task file
- `/todo board goal <text>` — Write/update `session_plan/goal.md`
- `/todo board evidence <text>` — Append to `session_plan/evidence.md`

**Rendering:** The board display is computed by reading all `session_plan/task_*.md` files,
grouping by Status, and rendering a Kanban-style view. It is never written to disk as the
source of truth.

## Implementation

1. Replace `TODO_MD_PATH` constant with `SESSION_PLAN_DIR` = `"session_plan"`.

2. Rewrite `board_template()` → not needed anymore (no TODO.md to template).

3. New helper: `read_task_files()` → reads `session_plan/task_*.md`, parses Title/Status/Issue
   from each, returns a Vec of task structs. Uses file-based IDs.

4. New helper: `render_board(tasks, goal)` → formats the Kanban view string from task data.

5. Rewrite `handle_todo_board()`:
   - `board` (bare): call `read_task_files()` + `render_board()`
   - `board init`: create `session_plan/` dir + optional `goal.md`
   - `board add`: create next `session_plan/task_NN.md` with Status: backlog
   - `board move <id> <status>`: find file, update/add Status line
   - `board done <id>`: shortcut for move to done
   - `board goal`: write `session_plan/goal.md`
   - `board evidence`: append to `session_plan/evidence.md`

6. Remove old helpers: `board_template`, `board_has_task`, `board_task_exists_anywhere`,
   `board_add_task`, `board_move_task`, `board_set_goal`, `board_add_evidence` — replace
   with new file-based implementations.

7. Update tests to use temp directories with `session_plan/` structure instead of `TODO.md`.

8. Update help text in `help_data.rs` for `/todo board` to reflect file-based IDs.

## Verification

- `cargo build && cargo test` must pass
- Tests should cover: init, add, move by ID, done, reading existing task files, board rendering
- NO `TODO.md` should be created or referenced anywhere in the new code
