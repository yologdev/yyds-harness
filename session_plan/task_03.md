Title: Extract todo and context handling from commands_project.rs into commands_todo.rs
Files: src/commands_todo.rs, src/commands_project.rs, src/dispatch.rs
Issue: none

## What to do

`commands_project.rs` is 2,113 lines handling four distinct concerns: todo, context, init/docs, and plan. Extract the todo concern (the largest self-contained chunk) into its own `commands_todo.rs` module.

### What to extract:

From `commands_project.rs`, move these items into a new `src/commands_todo.rs`:
- `enum TodoStatus`
- `struct TodoItem`
- `fn todo_add`
- `fn todo_update`
- `fn todo_list`
- `fn todo_clear`
- `fn todo_remove`
- `fn format_todo_list`
- `fn handle_todo`

These functions form a self-contained unit — the `/todo` command implementation with its own data types and display logic. They have minimal dependencies on the rest of `commands_project.rs`.

### Steps:

1. Create `src/commands_todo.rs` with the extracted functions and types
2. Add appropriate `use` statements in the new file for any dependencies (format utilities, etc.)
3. In `commands_project.rs`, remove the moved items and add `pub use commands_todo::*;` or update the dispatch to call from the new module
4. In `src/main.rs` (or wherever modules are declared), add `mod commands_todo;`
5. Update `src/dispatch.rs` if it dispatches `/todo` directly — make sure the import path points to the new module
6. Run `cargo build && cargo test` to verify nothing broke
7. Run `cargo clippy --all-targets -- -D warnings` to check for warnings

### Sizing note:
This is a focused extraction — one concern (todo) from one file, into one new file. The todo functions are self-contained with their own types (`TodoStatus`, `TodoItem`) and don't interleave with plan/context/init logic. This should be a clean cut.

### Why this matters:
`commands_project.rs` at 2,113 lines is the 10th largest file. After the `/skill` extraction last session (which created `commands_skill.rs`), the todo concern is the next cleanest extraction target. Each extraction makes the remaining file more focused and easier to work with.

### Do NOT update CLAUDE.md:
The project structure section in CLAUDE.md lists files and their contents. The implementation agent should NOT update CLAUDE.md — that's maintained separately. Just make sure the code compiles and tests pass.
