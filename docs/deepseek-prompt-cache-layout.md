# DeepSeek Prompt Cache Layout

DeepSeek context caching is server-side and enabled by default by DeepSeek. The
harness should not add request-side `cache_control` markers for DeepSeek.
Instead, optimize prompt layout so repeated requests share the longest possible
prefix, then measure results with `usage.prompt_cache_hit_tokens` and
`usage.prompt_cache_miss_tokens`.

DeepSeek cache matching is prefix-based. The model still receives the full
prompt; caching only avoids recomputing the matching prefix. Dynamic suffix
content still affects the result normally.

## Practical Rule

Good prompt order:

1. Stable identity
2. Stable safety/rules
3. Stable tool/schema policy
4. Stable repo/harness policy
5. Mostly stable repo map
6. Dynamic task
7. Dynamic logs/files/current evidence

Bad prompt order:

1. Current task timestamp/logs
2. Random run/session metadata
3. Stable identity/rules
4. Stable repo policy

Avoid putting timestamps, run IDs, random nonces, current task text, current
logs, or selected file excerpts before stable policy/context blocks. A changed
token near the beginning can prevent reuse for everything after it.

## Harness Contract

Autonomous prompts should use:

1. `YOYO_STABLE_CONTEXT` first.
2. Session metadata and task-specific instructions after stable context.
3. `YOYO_DYNAMIC_CONTEXT` after the session marker.
4. Current evidence, issue text, logs, selected files, and state summaries after
   those blocks.

`YOYO_CONTEXT` remains available for compatibility, but new prompts should use
the stable/dynamic split when DeepSeek cache reuse matters.
