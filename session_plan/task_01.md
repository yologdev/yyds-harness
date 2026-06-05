Title: Hook feedback — post-hooks can inject additional context into tool results
Files: src/hooks.rs
Issue: none

## What

Claude Code's hooks system allows hooks to return `additionalContext` that gets injected into 
the agent's context after a tool executes, influencing the next turn. Our hooks are observe-only —
post-hooks can see tool output but cannot feed information back to the agent.

Close this competitive gap by extending the post-hook system to support feedback.

## Design

1. Change the `post_execute` return type from `Result<String, String>` to a new type that carries
   both the (possibly modified) output AND optional feedback text:

   ```rust
   pub struct PostHookResult {
       pub output: String,
       pub feedback: Option<String>,
   }
   ```

2. Update `Hook::post_execute` trait method to return `Result<PostHookResult, String>`.
   Default implementation returns `PostHookResult { output: output.to_string(), feedback: None }`.

3. Update `HookRegistry::run_post_hooks` to collect feedback from all hooks and return both
   the final output and concatenated feedback.

4. In `HookedTool::execute`, if post-hooks returned feedback, append it as an additional 
   `Content::Text` block to the `ToolResult.content` vec. Prefix it with 
   `"\n[Hook feedback]\n"` so the agent knows it came from a hook.

5. For `ShellHook`, capture stderr from the hook command and use it as feedback (if non-empty).
   This means a shell hook like `hooks.post.bash = "eslint $TOOL_OUTPUT 2>&1 >&2"` would
   have its stderr become agent context.

6. Update `AuditHook` to continue returning no feedback (it's observe-only).

7. Add tests:
   - Hook with feedback returns it in the tool result
   - Hook without feedback doesn't add extra content
   - Multiple hooks — feedback concatenated
   - ShellHook stderr becomes feedback

## Scope

Only `src/hooks.rs` — this is self-contained. No other files need changes.
The trait change is internal (not pub in any external crate).
