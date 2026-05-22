Title: Add --with modifier to /retry for iterative refinement
Files: src/commands_retry.rs, src/dispatch.rs, src/help_data.rs
Issue: none

## Description

Add a `--with "..."` modifier to the `/retry` command so users can append additional instructions when retrying their last prompt. This enables iterative refinement without re-typing the full request — a common workflow in interactive coding sessions.

**Example usage:**
- `/retry --with "use async/await instead"`
- `/retry --with "make it shorter"`  
- `/retry --with "add error handling"`

### What to implement:

1. **In `src/commands_retry.rs`:**
   - Modify `handle_retry` to parse `--with` from the input:
     ```rust
     // Parse: /retry --with "additional instruction"
     let arg = input.strip_prefix("/retry").unwrap_or("").trim();
     let with_modifier = if let Some(rest) = arg.strip_prefix("--with") {
         let modifier = rest.trim().trim_matches('"').trim_matches('\'');
         if modifier.is_empty() { None } else { Some(modifier.to_string()) }
     } else {
         None
     };
     ```
   - When `with_modifier` is Some, append it to the retry prompt:
     ```rust
     let retry_input = if let Some(modifier) = &with_modifier {
         format!("{}\n\nAdditional instruction: {}", retry_input, modifier)
     } else {
         retry_input
     };
     ```
   - Update the "retrying" message to show the modifier: `"(retrying with modifier: use async/await instead)"`

2. **In `src/dispatch.rs`:**
   - No changes needed — `/retry` already routes to `handle_retry` with the full input string

3. **In `src/help_data.rs`:**
   - Update the `/retry` entry in `command_help` to document `--with`:
     ```
     /retry              Re-run the last prompt (with error context if available)
     /retry --with "..." Re-run with additional instructions appended
     ```
   - Update `command_short_description` for retry if needed

### Tests to add:
- Test parsing `--with` from "/retry --with use async" extracts "use async"
- Test parsing `--with` from "/retry --with \"quoted text\"" extracts "quoted text"  
- Test that "/retry" without --with still works as before (None modifier)
- Test that the modified retry prompt contains the additional instruction text

### Why this matters (self-driven):
This is a workflow improvement for iterative coding. When a developer says "fix this" and the result isn't quite right, they currently need to either:
1. Type a new full prompt explaining what's different
2. `/retry` which just repeats the same thing

With `--with`, they can efficiently steer: `/retry --with "but keep the original function signature"`. This is faster iteration, which is core to being competitive with Claude Code's conversational flow.
