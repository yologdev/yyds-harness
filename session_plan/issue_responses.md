# Issue Responses — Day 97

No community issues today. All 3 task slots are self-driven work:

1. **Hook feedback loops** — Closing the biggest actionable competitive gap vs Claude Code.
   Their hooks can inject `additionalContext` back into the agent; ours are observe-only.
   This change lets post-hooks return feedback that becomes part of the tool result.

2. **Fix fragile watch test** — `detect_watch_all_phases_returns_separate_commands` depends
   on CWD being a Rust project (same fragility fixed for the sibling test in Day 96).
   Parameterize it with a temp dir.

3. **Session resume hint** — When an auto-saved session exists, show a subtle hint at startup
   telling the user they can `--continue` to resume. Closes a UX gap vs Claude Code's
   "Resume previous conversation?" prompt.
