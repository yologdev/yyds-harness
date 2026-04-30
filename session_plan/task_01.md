Title: Extend research skill with RLM-style multi-source synthesis
Files: skills/research/SKILL.md
Issue: #353

## What to do

Extend the research skill (`skills/research/SKILL.md`) with a new procedure branch for multi-source research synthesis using the RLM substrate (sub-agents + SharedState).

### Changes to `skills/research/SKILL.md`:

1. **Frontmatter**: Add `sub_agent` and `shared_state` to the `tools:` list. Keep `core: true` and `origin: creator` unchanged.

2. **New section: "Multi-source synthesis"** — Add after the existing "When to research" section. This section describes the procedure for when yoyo encounters 3+ sources on a single topic OR any single source >5KB:

   **Trigger conditions** (when to use multi-source instead of single-source):
   - 3 or more sources on a single topic (papers, blog posts, docs, discussions)
   - Any single source exceeding 5KB
   - If sources are 1-2 and all under 5KB, use the existing single-source procedure

   **Procedure:**
   1. Parent fetches each source via `bash` (curl) and stores it in shared state under `research.<topic>.source-<N>` using the `shared_state` tool's `set` operation
   2. Parent dispatches one sub-agent per source with a focused question (e.g., "What does this source say about <aspect>?") — the sub-agent reads its assigned source by reference via `shared_state get key="research.<topic>.source-<N>"`
   3. Each sub-agent returns a JSON-shaped summary: `{"key_claims": [...], "key_quotes": [...], "relevance": "high|medium|low", "confidence": 0.0-1.0}`
   4. Parent stores the summaries under `research.<topic>.summaries`, then dispatches a final synthesis sub-agent that reads all summaries and produces the composed answer
   5. Hard depth cap = 3 (matches analyze-trajectory's pattern)
   6. On sub-agent failure or non-JSON response, fall back to direct `curl | head` and produce a low-confidence summary

   **Chunking for large sources** (>30KB): Split into chunks of ~25KB at paragraph boundaries, store each chunk as `research.<topic>.source-<N>.chunk-<M>`, dispatch one sub-agent per chunk.

3. **Add a `keywords:` field** to the frontmatter for skill-evolve scoring: `keywords: ["curl", "research", "synthesis", "shared_state", "sub_agent"]`

### Acceptance criteria (from issue #353):
- `skills/research/SKILL.md` `tools:` includes `sub_agent` and `shared_state`
- New "Multi-source synthesis" section with trigger conditions, step-by-step procedure
- JSON contract for sub-agent responses matches analyze-trajectory's pattern
- Chunking procedure for sources >30KB
- Fallback on sub-agent failure
- Hard depth cap = 3 documented
- No edits to other skills
- `cargo build && cargo test` still passes (skill files don't affect Rust compilation, but verify anyway)
