Title: Wire SharedState into SubAgentTool (RLM Layer 2 code substrate)
Files: src/tools.rs, src/main.rs
Issue: #344

## Context

Issue #344 was blocked on #343 (yoagent 0.7→0.8 upgrade), which shipped earlier today.
yoagent 0.8 provides `SharedState`, `SharedStateTool`, and `SubAgentTool::with_shared_state()`.
This task wires that into yoyo's sub-agent infrastructure so any sub-agent (including
analyze-trajectory's recursive dispatch) can share state with its parent by reference
instead of pasting artifacts into prompts.

## What to do

### 1. Modify `build_sub_agent_tool` in `src/tools.rs` (~line 1279)

- Create a `SharedState::new()` (from `yoagent::SharedState`)
- Pass it to the SubAgentTool via `.with_shared_state(state.clone())`
- Return both the SubAgentTool and the SharedState handle (change return type to a tuple,
  or create a small struct) so the caller can store artifacts before dispatching

The key change: `build_sub_agent_tool` currently returns `SubAgentTool`. Change it to
return `(SubAgentTool, SharedState)` so the parent agent can pre-populate state.

```rust
use yoagent::SharedState;

pub(crate) fn build_sub_agent_tool(config: &AgentConfig) -> (SubAgentTool, SharedState) {
    let shared_state = SharedState::new();
    // ... existing provider/tools setup ...
    let tool = SubAgentTool::new("sub_agent", provider)
        .with_shared_state(shared_state.clone())
        // ... existing builder calls ...
        ;
    (tool, shared_state)
}
```

### 2. Update `BUILTIN_TOOL_NAMES` in `src/main.rs`

Add `"shared_state"` to the `BUILTIN_TOOL_NAMES` array — yoagent 0.8's SharedStateTool
registers as `"shared_state"` and will collide with any MCP server exposing the same name.

### 3. Update callers of `build_sub_agent_tool`

Search for all call sites in `src/main.rs` (and any other files) and update them to
destructure the tuple. The SharedState handle can be stored alongside the agent or
discarded if the caller doesn't need it yet — the sub-agent has it regardless.

### 4. Add/update tests

- Test that `build_sub_agent_tool` returns a valid SharedState
- Test that `BUILTIN_TOOL_NAMES` contains `"shared_state"`
- Ensure existing tests (including `tests/integration.rs`) still pass

### 5. Verify

```bash
cargo build && cargo test && cargo clippy --all-targets -- -D warnings
```

## Important notes

- Use `yoagent::SharedState` (re-exported from `yoagent::shared_state`)
- The default `MemoryBackend` is correct — we want in-process shared state
- Do NOT modify `scripts/evolve.sh` or any workflow files
- Do NOT touch the analyze-trajectory skill yet — that's task 2
