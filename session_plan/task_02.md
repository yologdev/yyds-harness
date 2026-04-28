Title: Add /architect toggle for dual-model plan+implement
Files: src/commands_config.rs, src/repl.rs, src/help.rs
Issue: none

## What

Add an `/architect` command that toggles "architect mode" — a dual-model workflow where a strong reasoning model plans the changes and a cheaper/faster model implements them. This is Aider's killer feature and saves 60-80% on costs for complex tasks.

## Behavior

```
/architect              → toggle architect mode on/off
/architect on           → enable architect mode
/architect off          → disable architect mode
/architect <model>      → enable architect mode with a specific architect model
```

When architect mode is ON:
1. The user's prompt goes to the **architect model** (the current model, or a specified stronger model)
2. The architect responds with a plan (text-only, no tool use) — using a system prompt suffix that says "You are in architect mode. Describe exactly what changes to make, in which files, but do NOT make them. Be specific about line numbers and code."
3. The plan is automatically fed to the **editor model** (the cheaper model) as: "Implement the following plan exactly: <plan>"
4. The editor model executes with full tool access

Default architect model: the current model (e.g., claude-sonnet-4-20250514)
Default editor model: if the current model is sonnet → use haiku; if opus → use sonnet; otherwise use the cheapest available model from the same provider.

## Implementation

In `src/commands_config.rs`:
- Add `static ARCHITECT_MODE: AtomicBool` and `static ARCHITECT_MODEL: Mutex<Option<String>>`
- Add `fn set_architect_mode(on: bool, model: Option<String>)`
- Add `fn is_architect_mode() -> bool`
- Add `fn architect_model() -> Option<String>`
- Add `fn default_editor_model(current_model: &str) -> String` — maps strong models to cheap editors
- Add `pub fn handle_architect(input: &str)` that parses the command and toggles the mode
- Print status: `"  architect mode: ON (architect: claude-sonnet-4-20250514, editor: claude-haiku-3)"`

In `src/repl.rs`:
- In the main prompt path (where user input becomes a prompt), check `is_architect_mode()`
- If ON: 
  1. Build a modified system prompt for the architect turn: append "\n\nYou are in ARCHITECT mode. Describe the changes to make — which files, what to add/remove/modify, with specific code snippets. Do NOT use any tools. Just describe the plan."
  2. Run the prompt with `handle_quick`-style approach (no tools) to get the plan text
  3. Build the editor prompt: "Implement the following plan exactly:\n\n{plan}"  
  4. Run the editor prompt through the normal `run_prompt` path with full tool access
- The architect response should be displayed to the user before the editor starts

In `src/help.rs`:
- Add `/architect` to help text
- Short description: "Toggle architect mode (plan with strong model, implement with cheap model)"

In `src/commands.rs`:
- Add `"architect"` to KNOWN_COMMANDS

## Tests

- `default_editor_model("claude-sonnet-4-20250514")` → contains "haiku" 
- `default_editor_model("claude-opus-4-20250115")` → contains "sonnet"
- `default_editor_model("gpt-4o")` → contains "gpt-4o-mini" or similar
- Toggle on/off works correctly
- Parse `/architect sonnet` sets the model

## Notes

- This is the single highest-value competitive feature from Aider
- First version keeps it simple: same provider for both models, no cross-provider architect
- The architect prompt is text-only (no tools) which makes it fast and cheap
- The editor prompt gets the full tool suite
- This naturally composes with `/loop` — you could `/loop 3 /architect fix all test failures`
- Update CLAUDE.md architecture section to mention architect mode if it ships
