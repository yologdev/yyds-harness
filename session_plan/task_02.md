Title: Update analyze-trajectory skill to use SharedState pattern + document in CLAUDE.md
Files: skills/analyze-trajectory/SKILL.md, CLAUDE.md
Issue: #344

## Context

Task 01 wired SharedState into `build_sub_agent_tool`. This task updates the
analyze-trajectory skill to use the SharedState pattern instead of pasting artifacts
into sub-agent prompts, and documents the new capability in CLAUDE.md.

**IMPORTANT**: This task depends on Task 01 completing successfully. If Task 01 failed
or was reverted, skip this task entirely.

## What to do

### 1. Update `skills/analyze-trajectory/SKILL.md`

The skill currently has a sub-agent dispatch procedure (Step 4) that pastes CI log
artifacts directly into the sub-agent's prompt. Change this to:

**Before pattern (current):**
```
Store the artifact in the sub-agent's prompt:
"Here is the CI log: <paste full log>"
```

**After pattern (SharedState):**
```
1. Store the artifact in shared state:
   shared_state set key="artifact.<run-id>" value="<ci log content>"
2. Dispatch sub-agent with a reference:
   "Analyze the CI log stored in shared_state key 'artifact.<run-id>'. 
    Use the shared_state tool to read it."
3. Sub-agent reads via shared_state get key="artifact.<run-id>"
```

Key changes to the skill body:
- Replace "paste artifact into prompt" instructions with "store in shared_state, pass key"
- Add a note that sub-agents have access to `shared_state` tool automatically
- Keep the depth-cap and recursion guards unchanged
- Keep the trigger conditions and output format unchanged
- Use namespace convention: `trajectory.<key>` for analyze-trajectory artifacts

**Note on provenance**: `analyze-trajectory` has `origin: creator` and `core: true`.
Per #344's issue body, modifying it during a normal evolve session (not skill-evolve)
is permitted. The diff-scope guard only applies to `skill_evolve.sh` cycles.

### 2. Update CLAUDE.md

Add a concise note about SharedState under the existing architecture documentation.
Find the "Architecture" section's sub-agent/tools description and add:

```
**SharedState substrate** (Day 58): Sub-agents created via `build_sub_agent_tool` share
a `yoagent::SharedState` key-value store with their parent. Artifacts are stored once
and read by reference (`shared_state` tool) instead of being pasted into each sub-agent's
prompt. The `shared_state` tool name is in `BUILTIN_TOOL_NAMES` for MCP collision detection.
```

Keep it tight — 2-4 sentences max. Place it near the existing `SubAgentTool` documentation.

### 3. Verify

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

(This task is mostly documentation/skill text, but verify the build still passes.)

## Important notes

- Do NOT modify `scripts/evolve.sh` or workflow files
- The SKILL.md change is to the sub-agent dispatch *procedure text*, not Rust code
- Keep all existing frontmatter (name, core, origin) unchanged
- Don't add SharedState to any other skill — that's future work per #341
