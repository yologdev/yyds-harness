Title: Deduplicate lock-recovery helpers into a shared sync module
Files: src/tools.rs (or new src/sync_util.rs), src/commands_bg.rs, src/session.rs
Issue: none

## Problem

The assessment found 5 copies of lock-recovery helpers across the codebase:
- `lock_or_recover<T>(mutex)` in: `commands_bg.rs`, `commands_spawn.rs`, `session.rs`
- `rw_read_or_recover<T>(lock)` / `rw_write_or_recover<T>(lock)` in: `commands_project.rs`, `prompt.rs`

All are identical 3-line functions that call `.lock()` or `.read()`/`.write()` and recover from
poisoned locks. This is copy-paste debt from the Day 52 sweep.

## What to do

**This task consolidates only the `Mutex` helpers** (3 copies). The RwLock helpers are a
second task if needed — keeping this scoped to 3 files max.

1. Add public `lock_or_recover<T>` to a new small module `src/sync_util.rs` (just this one
   function + its tests). Re-export from there.
2. In `src/commands_bg.rs`: remove the local `lock_or_recover` definition, add
   `use crate::sync_util::lock_or_recover;`. Keep all call sites unchanged.
3. In `src/session.rs`: same — remove local definition, add the import.
4. `src/commands_spawn.rs` already has its own copy but that's file #4, so leave it for now
   (or if the agent finds it trivial, include it as a stretch — but don't exceed 3 files
   if it requires more changes).

Actually, since creating a new file (sync_util.rs) requires adding `mod sync_util;` in
main.rs, that's 4 files. Simpler approach: put `lock_or_recover` in an existing shared
module. `src/tools.rs` or `src/hooks.rs` are candidates, but semantically it's infrastructure.

**Simplest approach:** Create `src/sync_util.rs` with the helper + add `pub mod sync_util;`
in `main.rs` + update `commands_bg.rs` and `session.rs`. That's 4 files but the main.rs
change is a single line. The agent should be able to handle this.

## Tests

Move the existing `test_lock_or_recover_normal` and `test_lock_or_recover_poisoned` tests
from `commands_bg.rs` to `sync_util.rs`. The tests in `commands_bg.rs` can be removed since
the function lives elsewhere now.

## Verification

`cargo build && cargo test` must pass. `grep -rn "fn lock_or_recover" src/` should show
exactly ONE definition (in sync_util.rs).
