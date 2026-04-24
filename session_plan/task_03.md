Title: Add /quick command for fast single-turn answers without agent loop
Files: src/commands.rs, src/repl.rs, src/dispatch.rs (if dispatch_command moved by task 1, otherwise src/repl.rs)
Issue: none

**Why this matters competitively:** Claude Code and Gemini CLI both handle quick questions efficiently. yoyo's current flow always enters the full agent loop with tool access, which is heavyweight for questions like "what does this error mean?" or "how do I use sed to replace X?" A `/quick` command that sends a single prompt without tool access and streams the response is a quality-of-life feature that makes yoyo feel snappier for simple questions.

**What to do:**

1. Add `/quick` to the KNOWN_COMMANDS list in `commands.rs` with description "Quick answer without tools".

2. Add an arg hint in `command_arg_hint`: `"quick" => Some("[question]")`.

3. Implement the handler: when the user types `/quick how do I reverse a list in python`, create a one-shot prompt to the current model WITHOUT tool access and stream the response. Use the existing agent's provider/model config but skip tool registration.

4. Implementation approach:
   - Build a minimal agent (or reuse `build_side_agent` from `main.rs` if accessible) with no tools
   - Send the question as a single user message
   - Stream and render the response using the existing markdown renderer
   - Don't add the exchange to the main conversation history (it's a side-channel)

5. If building a side agent is too complex, a simpler approach: use the existing agent but prepend the prompt with an instruction like "Answer this question directly without using any tools:" — this is less clean but achieves the same UX.

6. Add the command to help text.

7. Tests:
   - Test that `/quick` appears in KNOWN_COMMANDS
   - Test argument parsing
   - Test that the command is recognized (not "unknown command")

**Key constraint:** Keep it simple. The MVP is "send question, get streamed answer, don't pollute main conversation." Don't add configuration, history, or caching.
