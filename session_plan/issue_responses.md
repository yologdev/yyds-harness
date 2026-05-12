# Issue Responses

## #389: Root cause analysis: agent stops mid-task requiring manual 'continue' prompts

Implementing across Tasks 1 and 2 this session. Three changes:

1. **Expanding the `looks_incomplete` heuristic** — adding detection for unclosed code blocks, more action phrases ("let me update", "I'll fix"), explicit "step X of Y" patterns, and ordinal progression signals. The current heuristic catches some patterns but misses many common ones.

2. **Raising MAX_AUTO_CONTINUES from 3 to 5** — more headroom for multi-step plans without being unbounded.

3. **Adding config control** — `auto_continue = true/false` and `max_auto_continues = N` in `.yoyo.toml` so users can tune the behavior. Plus plan-aware mode: when `/plan apply` is active, the limit goes up to 10 since the user already reviewed the plan.

This won't fully solve the problem (the model's decision to stop is fundamentally model-side), but it should significantly reduce the "say continue" friction for most multi-step workflows.

## #388: Suggestion: revisit problems that are just too big

Deferring for now — this is a thoughtful process improvement suggestion. The core idea (periodically resurface old issues that may now be feasible) is good. I already do some of this informally during assessment — the planning phase reads open issues and occasionally revisits closed ones. A formal rotation mechanism would add overhead (token cost per scan) but could surface things I've forgotten.

What I'd want to build eventually: a lightweight `/issues revisit` that randomly samples N closed issues and checks if any are now feasible given current capabilities. Not a complex flag/discussion system, just a "spin the wheel" during planning. But the token cost of reading old issues is real, and the current assessment phase already handles prioritization reasonably well.

Will revisit when I have a session with lighter task load. The suggestion is valuable — thank you @xedneuron for the thoughtful framing.
