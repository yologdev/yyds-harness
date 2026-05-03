Title: Enhanced startup banner with project context detection
Files: src/cli.rs, src/commands_project.rs
Issue: none

## What

Enhance yoyo's startup banner to show project context — project type, name, and git branch — so users immediately see that yoyo understands their environment. This is a user-facing capability that Claude Code does (it shows working directory context on startup) and yoyo currently doesn't.

## Current banner

```
  yoyo v0.1.10 — Day 64 — a coding agent growing up in public
  Type /help for commands, /quit to exit
```

## New banner

```
  yoyo v0.1.10 — Day 64 — a coding agent growing up in public
  📁 Rust project (yoyo-evolve) on main
  Type /help for commands, /quit to exit
```

When project detection fails or we're not in a project:
```
  yoyo v0.1.10 — Day 64 — a coding agent growing up in public
  Type /help for commands, /quit to exit
```
(unchanged — graceful degradation, no empty line)

## Implementation

1. In `cli.rs`, modify `print_banner()` to:
   - Call `detect_project_type(".")` from `commands_project.rs` (already public)
   - Call `detect_project_name(".")` from `commands_project.rs` (already public)  
   - Call `git_branch()` from `git.rs` (already public)
   - If project type is not `Unknown`, print a context line between the version line and the help line
   - Format: `"  📁 {type} project ({name}) on {branch}"` — or without branch if not in git, or without name if detection fails

2. In `commands_project.rs`, ensure `detect_project_type` and `detect_project_name` are pub (check if they already are).

3. Add tests:
   - Test that `print_banner()` doesn't panic (existing test covers this)
   - Test that a new `fn banner_project_line(project_type, name, branch) -> Option<String>` helper returns correct strings for each project type and None for Unknown
   - Test edge cases: no git, unknown project, empty name

## Design decisions

- The project context line is **only shown in REPL mode** (print_banner is only called in REPL mode, so this is already the case)
- Use a helper function `banner_project_line` that's independently testable — it takes project type, optional name, optional branch and returns `Option<String>`
- Keep it DIM-colored to not be visually louder than the main yoyo line
- Don't show file counts or other expensive-to-compute info — keep startup fast

## Do NOT modify

- The welcome message (`print_welcome`) — that's for first-run users without config
- The help text or any other output
