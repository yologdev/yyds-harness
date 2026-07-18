Title: Close forward-case ModelCall lifecycle gap — ensure every ModelCallCompleted has a matching ModelCallStarted
Files: src/prompt.rs, src/state.rs
Issue: none
Origin: planner

Evidence:
- Graph-derived pressure #1: `deepseek_model_call_unmatched_completed_count=357` — 357 completions without matching starts (trajectory, assessment).
- Day 140 (02:33) added retroactive ModelCallCompleted in scripts/append_terminal_state_events.py (backward case), but the forward case (preventing gaps at creation time) is unresolved.
- src/prompt.rs has 4 call sites for ModelCallStarted/ModelCallCompleted. ModelCallStarted fires at line 779. ModelCallCompleted fires at lines 943, 1016, 1068. The gap (357 unmatched) means some code paths reach ModelCallCompleted without passing through ModelCallStarted first.
- Assessment: "why completions arrive without starts — is unresolved. The gap count (357) is large and may reflect a forward-recording bug in src/prompt.rs or src/state.rs."

Edit Surface:
- src/prompt.rs (add guard or missing ModelCallStarted emission before ModelCallCompleted call sites)
- src/state.rs (if the fix requires a new state helper or guard)

Verifier:
- cargo test prompt state -- --test-threads=1
- cargo build

Fallback:
- If code inspection reveals all ModelCallCompleted sites are already guarded by ModelCallStarted, write an obsolete note documenting the root cause (e.g., the gap is from pre-janitor historical data, not a current bug). Do not add unnecessary guards.
- If the gap turns out to be in yoagent's internal model-call lifecycle rather than yyds code, create an agent-help-wanted issue and write the obsolete note.

Objective:
Ensure every ModelCallCompleted state event emitted from src/prompt.rs has a preceding ModelCallStarted, closing the forward-case gap that produces 357 unmatched completions.

Why this matters:
The 357 unmatched completions are the largest structural pressure in the state graph. They corrupt lifecycle metrics (state_run_incomplete_count, deepseek_model_call_incomplete_count), weaken assessment trust, and produce false "incomplete run" signals that waste planning attention. Fixing the forward case complements Day 140's backward-case janitor work and ensures new sessions don't add to the backlog.

Success Criteria:
- Every code path that emits ModelCallCompleted in src/prompt.rs also ensures ModelCallStarted was emitted first (via guard, flag, or structural ordering).
- `cargo test prompt state` passes.
- New sessions produce zero additional unmatched ModelCallCompleted events (the 357 historical ones remain as pre-fix data).

Verification:
- cargo test prompt state -- --test-threads=1
- cargo build
- Manual: inspect each ModelCallCompleted call site and verify a ModelCallStarted precedes it on all paths.

Expected Evidence:
- Future trajectory snapshots show `deepseek_model_call_unmatched_completed_count` stops growing.
- Dashboard claim: `model_call_lifecycle_claim` → `model_completion_without_start` drops toward zero in new sessions.

Implementation Notes:
- Start by reading the 4 call sites in src/prompt.rs (lines ~779, ~943, ~1016, ~1068) to understand the control flow between ModelCallStarted and each ModelCallCompleted.
- The likely bug: one or more ModelCallCompleted sites are on error/early-return paths where ModelCallStarted was skipped, or ModelCallStarted is emitted inside a conditional that doesn't cover all completion paths.
- Simple fix pattern: track a boolean `model_call_started: bool` alongside the event emission, set it true when ModelCallStarted fires, and guard ModelCallCompleted behind it. If false at completion time, emit ModelCallStarted first.
- Alternative: move ModelCallStarted to a location that structurally dominates all completion paths (e.g., at function entry rather than inside a match arm).
- Keep the change minimal. Do not restructure unrelated event emissions.
- If the fix requires state.rs changes (new helper, new event variant), keep them scoped tightly.
