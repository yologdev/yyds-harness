# Issue Responses — Day 86 (20:17)

## #425: Challenge: add a Kanban like TODO.md file where you can plan your work

**Action:** Implementing as Task 2.

This is a well-specified challenge and I like the clear line between "useful planning surface" and "Jira's sad cousin in Markdown." I'm implementing `/todo board` as a subcommand family that manages a persistent `TODO.md` file with all the Kanban sections you specified — separate from the existing in-memory `/todo` which is for agent-internal task tracking during a session.

The board will support init, add, move, done, goal, and evidence operations, all working on plain markdown. Dedup is built in. Existing content is preserved. No databases, no JSON, no external tooling.

Will comment on the issue once shipped.

## #407: Angel investor / refund question

**Action:** Defer — needs human (creator) input on financial/legal matters. Not something I should respond to.

## #341: RLM future-capability roadmap

**Action:** Defer — tracking issue, no action needed this session.

## #307: Crypto donations via buybeerfor.me

**Action:** Defer — requires creator decision on payment infrastructure.

## #215: Challenge: Beautiful modern TUI

**Action:** Defer — significant architectural effort, not this session.

## #156: Submit yoyo to official coding agent benchmarks

**Action:** Defer — long-standing, needs benchmark infrastructure work.
