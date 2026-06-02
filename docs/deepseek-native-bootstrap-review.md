# DeepSeek Native Bootstrap Review Boundary

Date: 2026-06-02

This is the review boundary for the current `deepseek-native-bootstrap` worktree.
It turns the broad production-plan work into a reviewable bootstrap PR without
claiming that every later roadmap phase is finished.

## Intended PR Scope

Include these changes in the bootstrap PR:

- project identity and packaging for `yoyo-ds-harness`
- `yoyo-ds` binary alias while preserving `yoyo`
- DeepSeek-native profile, config loading, provider defaults, and prompt context
- DeepSeek protocol helpers for thinking, strict schemas, JSON mode, FIM, cache,
  streaming replay, and transport diagnostics
- local `yoagent-state` adapter/projection and state CLI
- local fixture eval runner, eval comparison/reporting, release gates, and replay
- constrained harness evolution lifecycle commands
- release/CI workflow updates that check out sibling `yoagent-state`
- docs for bootstrap baseline, audit, install caveat, and review boundary
- test hardening needed for deterministic bootstrap gates

Exclude from this bootstrap PR:

- upstream edits to `yoagent`
- upstream edits to `/Users/yuanhao/Dev/yoagent-state`
- dashboard/UI work
- generic-provider parity work
- crates.io publishing before `yoagent-state` is released
- new graph/reporting feature slices not required by the bootstrap audit

## Suggested Commit Grouping

1. Identity and packaging
   - `Cargo.toml`
   - README and docs identity updates
   - installer and release archive naming updates
   - `yoyo-ds` alias

2. DeepSeek-native runtime
   - DeepSeek model/config/profile behavior
   - prompt/context layout
   - protocol helpers and diagnostics
   - FIM routing and guarded local-edit surfaces

3. State, eval, and evolution
   - local `yoagent-state` adapter/projection
   - state CLI and graph/query reports
   - fixture eval runner and comparison/reporting
   - harness patch lifecycle, promotion, rollback, and release gates

4. Bootstrap docs and gate hardening
   - `docs/deepseek-native-baseline.md`
   - `docs/deepseek-native-bootstrap-audit.md`
   - this review-boundary document
   - test fixes for `/fim` help discovery, no-color assertions, current-dir
     serialization, and fixture-agent changed-file verification

## Gate Status

Latest local gate run: passed on 2026-06-02.

- `cargo fmt --check`
- `cargo test`
  - unit tests: 3741 passed, 1 ignored
  - integration tests: 89 passed, 1 ignored
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo run --quiet --bin yoyo -- eval fixtures validate --suite local-smoke`
  - suite: `local-smoke`
  - tasks: 368
- `test ! -e .yoyo`
- `git -C /Users/yuanhao/Dev/yoagent-state status --short`

Known waiver:

- Live DeepSeek end-to-end smoke was not run because `DEEPSEEK_API_KEY` is not
  present in this environment. Run it before release/tagging with:

```bash
DEEPSEEK_API_KEY=... cargo run --bin yoyo-ds -- --deepseek-native "<small task>"
```

## Review Checklist

- Confirm the PR keeps `yoyo` compatibility while adding `yoyo-ds`.
- Confirm `yoagent = "0.8.3"` is consumed as a package.
- Confirm `yoagent-state` is used only as a sibling path dependency until it is
  released.
- Confirm `src/state.rs` is a local adapter/projection, not a fork of
  `yoagent-state`.
- Confirm no upstream package sources are modified from this repo.
- Confirm the local gate set still passes on the exact PR tree.
- Confirm the live DeepSeek smoke is either run with credentials or explicitly
  waived for pre-release review only.

## Remaining Actions

Before opening or merging a PR:

1. Choose the commit strategy: grouped commits as above, or one bootstrap commit.
2. Stage only bootstrap-owned files.
3. Re-run the local gate set after staging or committing.
4. Run the live DeepSeek smoke when credentials are available.
