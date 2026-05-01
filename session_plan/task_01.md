Title: Create multi-source-research synthesis skill (origin: yoyo)
Files: skills/synthesis/SKILL.md
Issue: #353

Create a new skill `synthesis` at `skills/synthesis/SKILL.md` with `origin: yoyo` that provides multi-source research synthesis using the RLM substrate (sub-agents + SharedState).

This resolves the blocked issue #353/#359 — previous attempts tried to modify `skills/research/SKILL.md` which is `origin: creator` (protected). The correct approach is a NEW skill that complements the existing research skill.

### Skill frontmatter:

```yaml
name: synthesis
description: "Multi-source research synthesis — aggregate and compare 3+ sources or any source >5KB using sub-agent dispatch and SharedState"
version: "1.0"
origin: yoyo
core: false
tools:
  - bash
  - read_file
  - write_file
  - search
  - sub_agent
  - shared_state
keywords: ["synthesis", "multi-source", "shared_state", "sub_agent", "research"]
```

### Skill content:

The skill should describe:

1. **When to use** (trigger conditions):
   - 3 or more sources on a single topic (papers, blog posts, docs, discussions)
   - Any single source exceeding 5KB
   - If sources are 1-2 and all under 5KB, use the existing `research` skill's single-source procedure

2. **Procedure:**
   - Parent fetches each source via `bash` (curl) and stores it in shared state under `synthesis.<topic>.source-<N>` using the `shared_state` tool's `set` operation
   - Parent dispatches one sub-agent per source with a focused question — the sub-agent reads its assigned source by reference via `shared_state get key="synthesis.<topic>.source-<N>"`
   - Each sub-agent returns a JSON-shaped summary: `{"key_claims": [...], "key_quotes": [...], "relevance": "high|medium|low", "confidence": 0.0-1.0}`
   - Parent stores summaries under `synthesis.<topic>.summaries`, then dispatches a final synthesis sub-agent that reads all summaries and produces the composed answer
   - Hard depth cap = 3 (matches analyze-trajectory's pattern)
   - On sub-agent failure or non-JSON response, fall back to direct `curl | head` and produce a low-confidence summary

3. **Chunking for large sources** (>30KB):
   - Split into chunks of ~25KB at paragraph boundaries
   - Store each chunk as `synthesis.<topic>.source-<N>.chunk-<M>`
   - Dispatch one sub-agent per chunk

4. **Relationship to research skill:**
   - This skill handles the multi-source and large-source cases
   - The existing research skill handles single small sources
   - When the research skill is invoked and the trigger conditions above are met, the agent should use synthesis instead

### Acceptance criteria:
- `skills/synthesis/SKILL.md` exists with correct frontmatter
- `origin: yoyo` (NOT `origin: creator`)
- `tools:` includes `sub_agent` and `shared_state`
- JSON contract for sub-agent responses documented
- Chunking procedure for sources >30KB documented
- Fallback on sub-agent failure documented
- Hard depth cap = 3 documented
- No edits to `skills/research/SKILL.md` or any other existing skill
- `cargo build && cargo test` still passes
