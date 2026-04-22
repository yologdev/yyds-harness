Title: Add /side command for quick questions during work
Files: src/repl.rs, src/commands.rs, src/help.rs
Issue: none (competitive gap — Codex has this)

## What to do

Codex 0.122.0 shipped `/side` conversations — the ability to ask a quick question without polluting the main conversation context. This is a meaningful UX gap. When a developer is mid-task and needs to check something ("what's the syntax for X?" or "what does this error mean?"), they currently have to either break their flow by asking in the main context (which adds noise to the agent's conversation history) or open a separate terminal.

### Implementation

1. **In `src/repl.rs`**: Add a `handle_side` async function that:
   - Takes the question text from `/side <question>`
   - Creates a fresh one-shot agent (using the same model/provider as the main agent) via `build_agent()` or a simpler construction
   - Sends the question as a single prompt
   - Streams the response to the terminal with a visual indicator (e.g., `{DIM}[side]{RESET}` prefix or a bordered section)
   - Does NOT add anything to the main agent's message history
   - Returns without modifying `session_total` tokens (side conversations are "free" from the main context perspective, though they do cost real tokens — track and display the side cost separately)
   
   For the simplest viable version: use `yoagent::Agent` with a minimal system prompt ("Answer concisely. This is a quick side question."), run `prompt()` → drain events → `finish()`, display the response, and return. No need for tool access in side conversations initially.

2. **In `src/commands.rs`**: Add `"/side"` to `KNOWN_COMMANDS` array.

3. **In `src/help.rs`**: Add help text for `/side`:
   ```
   /side <question>  — Ask a quick question without affecting the main conversation
   ```

4. **In `src/repl.rs` dispatch_command**: Add the `/side` arm that calls `handle_side`.

5. **Tests**: Add a test for parsing side input (extracting the question from `/side what is X`). The actual agent call can't be tested without an API key, but the parsing and help text registration can be verified.

### Key design decisions
- Side conversations get NO tools (just text Q&A) — this keeps them fast and cheap
- Side conversations don't affect main context — the whole point is isolation
- Show cost of side conversation separately so users know what it costs
- If no question is provided, show usage help

This directly closes a competitive gap with Codex and is a genuinely useful feature for developers mid-task.
