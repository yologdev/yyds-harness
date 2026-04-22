Title: Add /checkpoint command for named file-state snapshots
Files: src/commands_session.rs, src/repl.rs, src/help.rs
Issue: none

## What to do

Add a `/checkpoint` command that lets users create, list, restore, and diff named file-state snapshots within a session. This closes a real capability gap vs Claude Code, which has built-in checkpoint/rewind.

The existing `TurnSnapshot` (in `prompt.rs`) captures file state per turn for `/undo`. `/checkpoint` builds on the same concept but with explicit user-named save points.

## Commands

- `/checkpoint <name>` or `/checkpoint save <name>` — snapshot all currently modified files (tracked by SessionChanges) and any explicitly listed files
- `/checkpoint list` — show all saved checkpoints with timestamp and file count  
- `/checkpoint restore <name>` — restore all files to their state at checkpoint time
- `/checkpoint diff <name>` — show what changed since the named checkpoint
- `/checkpoint delete <name>` — remove a named checkpoint

## Implementation

### In `src/commands_session.rs`:

1. Add a `Checkpoint` struct:
```rust
pub struct Checkpoint {
    pub name: String,
    pub created: std::time::Instant,
    pub files: HashMap<String, String>,  // path -> content at checkpoint time
}
```

2. Add a `CheckpointStore` (a simple `Vec<Checkpoint>` or `HashMap<String, Checkpoint>`):
```rust
pub struct CheckpointStore {
    checkpoints: HashMap<String, Checkpoint>,
}
```

With methods:
- `new() -> Self`
- `save(name: &str, changes: &SessionChanges)` — reads current content of all files in changes, stores snapshot
- `restore(name: &str) -> Result<Vec<String>, String>` — restores files, returns action list
- `list() -> Vec<(&str, usize, Instant)>` — returns (name, file_count, created)
- `diff(name: &str) -> Result<String, String>` — diffs checkpoint state against current files
- `delete(name: &str) -> bool`

3. Add `pub fn handle_checkpoint(input: &str, store: &mut CheckpointStore, changes: &SessionChanges)` that parses the subcommand and dispatches.

### In `src/repl.rs`:

1. Add `checkpoint_store: CheckpointStore` to the REPL state (near where `turn_history`, `session_changes`, etc. are initialized)
2. Add command dispatch for `/checkpoint`:
```rust
s if s == "/checkpoint" || s.starts_with("/checkpoint ") => {
    handle_checkpoint(input, &mut checkpoint_store, &session_changes);
}
```

### In `src/help.rs`:

1. Add help entry for "checkpoint" in `command_help()` function
2. Add to the Session Management category in help listings
3. Add to `help_command_completions()` for tab-completion

### In `src/commands.rs`:

1. Add "/checkpoint" to `KNOWN_COMMANDS`

## Tests (in commands_session.rs)

- `test_checkpoint_save_and_list` — save a checkpoint, verify it appears in list
- `test_checkpoint_restore` — save checkpoint, modify file, restore, verify content restored
- `test_checkpoint_diff` — save checkpoint, modify file, diff shows changes
- `test_checkpoint_delete` — save and delete, verify removed
- `test_checkpoint_duplicate_name_overwrites` — saving with same name replaces
- `test_checkpoint_restore_nonexistent` — returns error message

## Key considerations

- Use `std::time::Instant` for timing (relative, not wall-clock) — display as "2m ago" etc.
- Checkpoint names should be alphanumeric + hyphens/underscores, reject spaces
- When restoring, warn about files that have been deleted since checkpoint
- `/checkpoint` with no args should show usage help
- The checkpoint store lives in REPL state and is NOT persisted to disk (session-scoped)
- Import SessionChanges from `crate::prompt::SessionChanges`

## Verification

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt -- --check
```

Note: this task touches 3 files for implementation (commands_session.rs, repl.rs, help.rs) plus commands.rs for the KNOWN_COMMANDS addition. If the 3-file limit is strict, the commands.rs change (adding one string to an array) can be included as a minimal touch.
