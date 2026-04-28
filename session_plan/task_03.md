Title: Analyze-trajectory JSON contract + token-aware chunking
Files: skills/analyze-trajectory/SKILL.md, scripts/extract_trajectory.py
Issue: #345

## What

Finish the remaining polish items from Issue #345. Day 58 Session 3 already improved fingerprint clustering. The two remaining items are:

1. **JSON contract for sub-agent dispatch** — When the skill dispatches a sub-agent to analyze a CI log, the sub-agent's response format is unstructured prose. If parsing fails or the sub-agent rambles, there's no retry. Add a clear JSON contract so the parent can validate the response and retry once if it's malformed.

2. **Token-aware chunking** — When a CI log is large (>50KB), the current skill just sends the whole thing to the sub-agent. Add chunking guidance: estimate tokens (~4 chars/token), and if the artifact exceeds the sub-agent's budget, split into chunks with overlap and dispatch multiple sub-agents, then merge their findings.

## Implementation

### In `skills/analyze-trajectory/SKILL.md`:

**JSON contract (Section 4 "Dispatch a sub-agent"):**
- Update the sub-agent prompt template to include an explicit output format:
  ```
  Respond with ONLY a JSON object:
  {"diagnosis": "one sentence root cause", "evidence": ["key line 1", "key line 2"], "confidence": "high|medium|low"}
  ```
- Add a validation step after the sub-agent returns: check if the response parses as JSON with the required fields
- If validation fails, retry ONCE with a prompt: "Your previous response was not valid JSON. Please respond with ONLY: {diagnosis, evidence, confidence}"
- If retry also fails, fall back to treating the raw text as the diagnosis (don't crash)

**Token-aware chunking (new Section 3.5 "Handle large artifacts"):**
- Before dispatching, estimate tokens: `artifact_bytes / 4`
- If estimated tokens > 30,000 (roughly half a sub-agent's context):
  - Split into chunks of ~20,000 tokens (~80KB) with 2,000-token overlap
  - Dispatch one sub-agent per chunk with the prompt: "Analyze this CHUNK of a CI log for the failure pattern. This is chunk N of M."
  - After all chunk sub-agents return, dispatch one final sub-agent: "Merge these chunk analyses into a single diagnosis" with all chunk results in shared_state
- If estimated tokens <= 30,000: proceed as before (single sub-agent)

### In `scripts/extract_trajectory.py`:
- No changes needed — the fingerprint clustering was already improved in Day 58

## Tests

The skill file is markdown (no unit tests), but verify:
- The JSON contract format is clearly specified
- The chunking threshold and sizes are consistent (30k token threshold, 20k chunk size, 2k overlap)
- The retry logic is described step-by-step
- The fallback behavior (raw text if JSON fails twice) is explicit

Run `cargo build && cargo test` to make sure nothing in the Rust codebase broke (the skill file is loaded at runtime, not compiled).

## Notes

- This closes the remaining items from Issue #345
- The fingerprint clustering sub-task was already done in Day 58 Session 3
- Keep the skill file readable — it's both documentation and instruction
- The JSON contract is important because it enables programmatic validation of sub-agent output, which is a prerequisite for reliable multi-agent workflows
