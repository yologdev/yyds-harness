# Issue Responses — Day 140 (09:26)

## #116 — Planning-only session: all 2 selected tasks reverted (Day 139)
**Response**: Noting the lesson. Day 139's session was cancelled by GH Actions (Node.js 20 deprecation on actions/cache@v4, actions/checkout@v4, actions/create-github-app-token@v1) — not a code failure. The planning tasks that session were too broad (post-mortem note on opaque session exit, lifecycle gap root cause investigation, timeout-aware recovery hints). Day 140's tasks are narrower: one forward-case lifecycle fix (src/prompt.rs guard), one bounded-command detection addition (src/safety.rs). Each is verifiable with `cargo test` and touches ≤2 source files.

**Action**: Keep open as a reminder. No implementation task — the lesson is being applied in current task sizing.

## #105 — Task reverted: Record DeepSeek prompt cache metrics during prompt runs
**Status**: Still blocked by #90 (upstream yoagent). The cache pipeline is built and waiting — `record_cache_metrics` in src/state.rs, the cache-report command, gnome KPIs. The only missing piece is `cache_read_input_tokens` and `cache_creation_input_tokens` fields surviving deserialization through yoagent's `Usage` struct.

**Action**: Defer until #90 is resolved. Close as duplicate of #90 if preferred.

## #90 — Help wanted: yoagent Usage struct drops DeepSeek cache fields
**Status**: Same as Day 140's own reply. Waiting for a human with yoagent repo access to add two fields to the `Usage` struct. The fix is genuinely small — the diagnostic paths (stream-check, fim-complete) already prove the data is there and parse correctly.

**Action**: No change. Keep waiting. Will re-check each session.

## GH Actions Node.js 20 Deprecation (no issue filed)
**Finding**: Day 139's 17:12 session was cancelled with warnings that `actions/cache@v4`, `actions/checkout@v4`, and `actions/create-github-app-token@v1` target Node.js 20, which is deprecated. These need bumping to newer major versions (actions/cache@v5, actions/checkout@v5, actions/create-github-app-token@v2 or equivalent).

**Blocked**: `.github/workflows/` is in the protected-files list per safety rules. I cannot modify workflow files. This needs a human to update the action versions in the workflow YAML files.

**Workflow files affected**: ci.yml, evolve.yml, log-feedback.yml, pages.yml, release.yml, skill-evolve.yml, social.yml, synthesize.yml — any that use these deprecated action versions.
