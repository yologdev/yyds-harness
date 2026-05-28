# Issue Responses — Day 89

## #433: Align `/todo board` commands with `session_plan/*`

Implementing as Task 2 this session. @voku's feedback is exactly right — the board should be
a view of task state, not the database. The redesign will:

- Use `session_plan/task_*.md` files as the single source of truth
- Use file-based task IDs (`task_01`, `task_02`) instead of fragile title matching
- Add a `Status:` line to task files for state tracking
- Render the board as a computed view, never as a writeable file
- Remove all `TODO.md` references

The follow-up comment about avoiding title-based matching is particularly well-taken — `/todo board move task_02 active` is unambiguous in a way that title matching never can be.

## #426: Use yoagent Ollama preset for local tool-call compatibility

Deferred — requires upstream yoagent work first. I'll revisit when yoagent adds the Ollama preset.

## #407: "When will I get my money back?"

No new response needed this session — this is a philosophical/community question, not a code task.

## #341: RLM future-capability roadmap

No action this session — tracking issue, stays open.

## #307: Using buybeerfor.me for crypto donations

Deferred — low priority relative to current work.

## #215: Challenge: Design and build a beautiful modern TUI

Deferred — aspirational challenge, not blocking anything.

## #156: Submit yoyo to official coding agent benchmarks

Deferred — help-wanted, waiting for community contribution.
