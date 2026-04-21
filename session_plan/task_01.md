Title: Finish poison-proofing: RwLock/Mutex .unwrap() → safe recovery in remaining 3 files
Files: src/commands_project.rs, src/commands_session.rs, src/prompt.rs
Issue: none

Day 52's earlier session poison-proofed commands_bg.rs (13 sites) and commands_spawn.rs (8 sites) using a `lock_or_recover` helper. But 16 production `.lock()/.read()/.write().unwrap()` calls remain across 3 files. Complete the sweep.

**What to do:**

1. In `src/commands_project.rs`:
   - The `TODO_LIST` RwLock has 5 `.write().unwrap()` / `.read().unwrap()` calls in `todo_add`, `todo_update`, `todo_list`, `todo_clear`, `todo_remove` (lines 56, 62, 74, 79, 85).
   - Add a local `rw_write_or_recover` and `rw_read_or_recover` helper (or import from a shared location) that calls `.write()` / `.read()` and on `PoisonError` calls `.into_inner()` to recover.
   - Replace all 5 sites.

2. In `src/commands_session.rs`:
   - The `CONVERSATION_STASH` RwLock has 5 sites (lines 537, 572, 592, 617, 629).
   - Apply the same `rw_write_or_recover` / `rw_read_or_recover` pattern.

3. In `src/prompt.rs`:
   - `WATCH_COMMAND` RwLock has 3 sites (lines 21, 27, 33).
   - `SessionChanges.inner` Mutex has 3 sites (lines 191, 205, 210).
   - For the Mutex, use a `lock_or_recover` pattern like commands_bg.rs.
   - For the RwLock, use the same rw helpers.

**Design decision:** Each file should have its own local helper functions (like commands_bg.rs and commands_spawn.rs already do) rather than trying to create a shared module — that would touch more files and add coupling. Keep it simple: copy the 4-line helper pattern.

**Helper patterns:**
```rust
fn lock_or_recover<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn rw_read_or_recover<T>(lock: &std::sync::RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

fn rw_write_or_recover<T>(lock: &std::sync::RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}
```

**Verification:** `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`

All existing tests should pass unchanged — the behavior is identical except on poisoned locks, which previously would panic.
