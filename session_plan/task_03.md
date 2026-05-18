Title: Add /tips command — context-sensitive feature discovery
Files: src/commands_info.rs, src/dispatch.rs, src/help.rs
Issue: none

## Goal

Address the assessment's key insight: "The biggest achievable gap to close is not a feature but
a quality: making existing features more discoverable and polished." With 85 slash commands,
most users will never discover the most useful ones for their workflow.

Add a `/tips` command that shows contextual, actionable tips based on what the user is doing
right now. This is the "did you know?" feature that turns yoyo from overwhelming to welcoming.

## What to build

### 1. `fn generate_tips() -> Vec<String>` in `commands_info.rs`

Generate tips based on current context signals:

**Project-type tips:**
- Rust project detected → "💡 `/watch cargo test` auto-runs tests after every change"
- Rust project → "💡 `/lint fix` runs clippy and auto-fixes warnings"
- Node project → "💡 `/watch npm test` monitors your test suite"
- Git repo → "💡 `/diff --stat` shows a compact summary of your changes"

**Session-state tips:**
- No watch command set → "💡 Set `/watch <cmd>` to auto-check after every agent edit"
- Long conversation (>10 turns, check via messages len if accessible; if not, skip) → "💡 `/compact` summarizes old context to free up token space"
- No goal set → "💡 `/goal set <description>` gives the agent persistent focus"

**Feature-discovery tips (randomly sampled, 2-3 per invocation):**
- "💡 `/spawn <task>` runs a sub-agent for parallel work"
- "💡 `/add <file>` injects file contents into the conversation" 
- "💡 `/find <query>` does fuzzy file search across your project"
- "💡 `/grep <pattern>` searches file contents with context"
- "💡 `/map` shows a symbol map of your codebase"
- "💡 `/fork` creates a conversation branch to try a different approach"
- "💡 `/checkpoint save <name>` snapshots your files for easy rollback"
- "💡 `/review` gets an AI code review of your current diff"
- "💡 `/export` saves the conversation as markdown"
- "💡 `/doctor` checks your environment for common issues"
- "💡 `/profile` shows where time was spent this session"
- "💡 `/bg <cmd>` runs commands in the background"
- "💡 Use `@file.rs` in your prompt to auto-inject file contents"
- "💡 `/plan` enables plan mode — think before acting"
- "💡 `/open <file>` opens files in your editor"

### 2. `fn handle_tips()` in `commands_info.rs`

- Call `generate_tips()`
- Print a header: "🐙 **Tips for your current session:**"
- Print all contextual tips first (project-type + session-state)
- Then print 2-3 randomly sampled feature-discovery tips
- Use colored output (CYAN for the 💡 prefix, DIM for descriptions)

### 3. Wire it up

- Add `/tips` to `KNOWN_COMMANDS` in `commands.rs`
- Add dispatch route in `dispatch.rs` → `commands_info::handle_tips()`  
- Add to `/help` output in `help.rs` (in the info/status group)
- Add short description: "tips — Context-sensitive feature suggestions"
- Add `command_arg_hint` entry (no args)

### 4. Tests

- Test `generate_tips()` returns non-empty vec
- Test that Rust-project tips include watch/lint hints (mock project detection)
- Test that feature-discovery tips are sampled (call twice, may differ)
- Test tip text format (all start with "💡")

## Constraints
- Touch at most 3 source files: `commands_info.rs`, `dispatch.rs`, `help.rs`
  (adding to `KNOWN_COMMANDS` in `commands.rs` counts as a minor edit — if it would
  make 4 files, add `/tips` to KNOWN_COMMANDS inside `commands_info.rs` conditionally
  or just include the dispatch.rs change with the commands.rs KNOWN_COMMANDS addition
  together as they're both one-line changes. The 3-file rule is about *substantial* changes.)
- Keep it simple — no persistent state, no tracking which tips were shown before
- Tips should feel helpful, not preachy — short, actionable, specific
- Use existing project detection from `commands_project.rs` (ProjectType enum)
