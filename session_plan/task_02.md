Title: Implement /todo board command for TODO.md Kanban planning
Files: src/commands_todo.rs, src/dispatch.rs
Issue: #425

## Description

Add a `/todo board` subcommand that creates and manages a repository-level `TODO.md` file as a Kanban-style planning board. This is SEPARATE from the existing in-memory `/todo` (which is for the agent's session-internal task tracking). The board is a persistent file that survives across sessions.

### What to implement

**In `commands_todo.rs`:**

Add these functions:

1. `board_template(goal: &str) -> String` — generates the initial TODO.md content with these sections:
   - `# TODO`
   - `## Current Goal` (filled with the goal parameter)
   - `## Constraints` (empty bullet list)
   - `## Backlog` (empty)
   - `## Ready` (empty)
   - `## In Progress` (empty)
   - `## Blocked` (empty)
   - `## Review` (empty)
   - `## Done` (empty)
   - `## Evidence Log` (empty)

2. `parse_board_section(content: &str, section: &str) -> Vec<String>` — extracts items from a named section. Items are lines starting with `- [ ]` or `- [x]`.

3. `board_has_task(content: &str, section: &str, task: &str) -> bool` — checks if a task (by description text) already exists in a section, for dedup.

4. `board_add_task(content: &str, section: &str, task: &str) -> String` — adds a `- [ ] task` line to the end of the named section. Returns updated content. Deduplicates: if task already exists in ANY section, returns content unchanged.

5. `board_move_task(content: &str, from_section: &str, to_section: &str, task: &str) -> String` — moves a task from one section to another. If moving to "Done", marks it `[x]`.

6. `board_set_goal(content: &str, goal: &str) -> String` — replaces the content under `## Current Goal` with the new goal text.

7. `board_add_evidence(content: &str, evidence: &str) -> String` — appends an evidence line `- evidence` to the Evidence Log section.

8. `handle_todo_board(input: &str) -> String` — dispatches `/todo board` subcommands:
   - `/todo board` or `/todo board show` — reads and displays `TODO.md` (or says it doesn't exist)
   - `/todo board init [goal]` — creates `TODO.md` if it doesn't exist (preserves existing content). Default goal: "Planning board initialized."
   - `/todo board add <section> <task>` — adds a task to a section (backlog/ready/inprogress/blocked/review)
   - `/todo board move <task_text> <to_section>` — moves a task between sections
   - `/todo board goal <text>` — updates the Current Goal
   - `/todo board evidence <text>` — appends to Evidence Log
   - `/todo board done <task_text>` — shortcut: moves task to Done and marks [x]

**In `dispatch.rs`:**
The existing `/todo` dispatch already goes to `handle_todo`. Modify `handle_todo` to check if the argument starts with `board` and route to `handle_todo_board`.

### Tests to add (in `commands_todo.rs`)

- Test `board_template` generates all required sections
- Test `parse_board_section` extracts items correctly
- Test `board_has_task` finds existing tasks and rejects missing ones
- Test `board_add_task` adds tasks and deduplicates
- Test `board_move_task` moves between sections, marks Done as [x]
- Test `board_set_goal` replaces goal text
- Test `board_add_evidence` appends evidence lines
- Test `handle_todo_board` with "show" when no file exists
- Test `handle_todo_board` with "init" creates proper template

### Important constraints
- All TODO.md operations work on the file content as a string (read → transform → write). The file path is always `TODO.md` in the current directory.
- Preserve existing content: if TODO.md already exists, `init` should NOT overwrite it.
- Plain markdown only — no JSON, no databases.
- Task dedup: check all sections before adding.

### Verification
`cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
