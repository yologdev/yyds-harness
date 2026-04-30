Title: Create explore-codebase skill for RLM-style large-codebase comprehension
Files: skills/explore-codebase/SKILL.md
Issue: #354

Create the `explore-codebase` skill as requested by @yuanhao in issue #354 (child of #341 RLM roadmap).

This is a NEW skill (not an extension of self-assess) because self-assess targets yoyo's own code, while explore-codebase covers ANY codebase the agent encounters — forks, dependencies, unfamiliar regions, user projects.

## What to create

`skills/explore-codebase/SKILL.md` with proper YAML frontmatter:

```yaml
---
name: explore-codebase
description: RLM-style large-codebase comprehension — build a mental map of any codebase by dispatching sub-agents to explore regions without bloating main context
tools: [bash, read_file, list_files, search, sub_agent, shared_state]
core: false
origin: yoyo
keywords: ["explore", "codebase", "archaeology", "comprehension", "map", "understand"]
---
```

## Key design decisions (from issue #354)

1. **Scope**: Any codebase the agent encounters, not just its own source
2. **RLM pattern**: Uses sub-agents to explore large codebases without bloating main context
3. **Procedure**: 
   - Phase 1: Orientation — list files, read README/docs, build initial map (use `/map` if available)
   - Phase 2: Targeted exploration — dispatch sub-agents to explore specific regions/modules
   - Phase 3: Synthesis — merge sub-agent findings into a coherent mental model
4. **Artifact management**: Store exploration results in SharedState under `explore.<key>` namespace
5. **Size thresholds**: Direct read for files ≤5KB, sub-agent dispatch for larger artifacts (consistent with RLM substrate conventions in CLAUDE.md)
6. **Depth cap**: Maximum 3 levels of sub-agent recursion (same as analyze-trajectory)

## Model after analyze-trajectory

Use `skills/analyze-trajectory/SKILL.md` as the structural template — it's the canonical RLM skill. Key sections to mirror:
- When to use / When NOT to use
- Procedure with numbered steps
- Sub-agent dispatch contract (JSON schema for sub-agent responses)
- Depth cap and fallback behavior
- SharedState namespace conventions

## Triggers

The skill should activate when:
- Agent encounters an unfamiliar codebase (fork, dependency, user project)
- Agent needs to understand a large region it hasn't touched recently
- User explicitly asks to explore/understand a codebase
- `/add` brings in a large project and the agent needs context

## What NOT to do

- Don't modify any source files (this is a skill file only)
- Don't modify CLAUDE.md (the skill system auto-discovers skills from the directory)
- Don't duplicate self-assess — this skill is about comprehension, not bug-finding
