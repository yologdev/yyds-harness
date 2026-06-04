Title: Auto-memory: capture fix insights after successful watch-mode recovery
Files: src/watch.rs, src/memory.rs
Issue: none

## What

Claude Code automatically saves build commands and debugging insights across sessions. yoyo currently requires manual `/remember`. This task adds automatic memory capture after the watch-mode fix loop successfully recovers from a build/test failure.

## Why

This is the #1 actionable competitive gap identified in the assessment. Auto-memory turns yoyo's watch-fix loop from a one-time recovery into cumulative learning. Over time, the agent accumulates project-specific knowledge about what kinds of errors occur and how they were fixed, making it smarter on familiar codebases.

## How

### In `src/memory.rs`:
1. Add a new function `auto_remember(note: &str)` that:
   - Calls `load_memories()`
   - Checks for duplicate/near-duplicate notes (simple substring match of the first 50 chars to avoid flooding)
   - If not duplicate, adds a new `MemoryEntry` with the note and current timestamp
   - Calls `save_memories()`
   - Prints a dim message: `"  💾 Auto-remembered: {truncated_note}"`
   - Returns `bool` indicating whether a memory was actually saved

2. Add a helper `build_fix_memory_note(watch_cmd: &str, attempt: u32) -> String` that creates a concise note like: `"Watch fix: '{watch_cmd}' failed, fixed on attempt {attempt}"`

### In `src/watch.rs`:
3. After the successful fix in `run_watch_after_prompt` (the `phase_passed = true; break;` line around line 1262), call `crate::memory::auto_remember()` with a note describing what happened:
   - Include the watch command that failed
   - Include which attempt succeeded
   - Keep it concise (one line)

4. This is a lightweight insertion — just 2-3 lines in the existing fix loop success path.

### Tests:
5. In `src/memory.rs`, add tests for:
   - `auto_remember` creates a memory entry in a temp dir
   - `auto_remember` deduplicates (calling twice with same note only stores once)
   - `build_fix_memory_note` formats correctly

## Constraints
- Do NOT change the memory file format — same JSON structure as manual `/remember`
- Do NOT add auto-memory to any other code path yet (just watch-fix for now — we can expand later)
- Keep the note concise — this goes into the prompt context, so brevity matters
- Use a temp directory for tests, not the real `.yoyo/` directory
