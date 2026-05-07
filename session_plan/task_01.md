Title: Fix silent .ok() error swallowing in piped mode and retry path
Files: src/main.rs, src/prompt.rs
Issue: none

## Problem

Two `.ok()` calls silently swallow errors, giving misleading diagnostics:

1. **`src/main.rs` line 349** — `io::stdin().read_to_string(&mut input).ok()` in `run_piped_mode`. 
   When stdin read fails (broken pipe, encoding error, etc.), the error is silently dropped and the
   user sees "No input on stdin" instead of the real I/O error. This is misleading.

2. **`src/prompt.rs` lines 707 and 957** — `agent.save_messages().ok()` in the retry path.
   When message state save fails before retry, the `saved_state` is `None`, so retry can't restore
   conversation state. The user gets duplicate messages or broken retries with zero visibility into
   why.

## Fix

### main.rs (line 349)
Replace:
```rust
io::stdin().read_to_string(&mut input).ok();
```
With proper error handling:
```rust
if let Err(e) = io::stdin().read_to_string(&mut input) {
    eprintln!("Error reading stdin: {e}");
    std::process::exit(1);
}
```

### prompt.rs (lines 707 and 957)
Replace both instances of:
```rust
let saved_state = agent.save_messages().ok();
```
With:
```rust
let saved_state = match agent.save_messages() {
    Ok(state) => Some(state),
    Err(e) => {
        eprintln!("{DIM}  ⚠ Could not save message state for retry: {e}{RESET}");
        None
    }
};
```

This preserves the existing fallback behavior (retries still work, just without state restore)
but now the user/operator can see WHY retry state restore failed.

## Tests
- Existing tests should still pass (the change is backward-compatible in behavior)
- Add a test for the prompt.rs case that verifies the pattern compiles correctly
- The main.rs change can be verified by checking that the function still handles the empty-input case
