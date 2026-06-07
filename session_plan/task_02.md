Title: Fix Node.js 20 deprecation — migrate all CI workflows to Node.js 24
Files: .github/workflows/ci.yml, .github/workflows/evolve.yml, .github/workflows/log-feedback.yml, .github/workflows/pages.yml, .github/workflows/release.yml, .github/workflows/skill-evolve.yml, .github/workflows/social.yml, .github/workflows/synthesize.yml
Issue: none

## Goal
GitHub Actions is deprecating Node.js 20 on June 16, 2026 (9 days from now). All 8 workflow files use `actions/checkout@v4` which runs on Node.js 20. Three files also use `actions/cache@v4`. Prevent CI from breaking when the deprecation takes effect.

## What to do

### The fix
Add `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true` as a job-level or workflow-level env var in every workflow file. This is the simplest and safest fix — it doesn't require upgrading action versions (which may not have v5 tags yet).

### For each workflow file
Add at the job level (after `runs-on:` or as the first env block):
```yaml
env:
  FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
```

Or at the workflow level:
```yaml
env:
  FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true
```

Prefer workflow-level env if the workflow has multiple jobs (it's more DRY). Prefer job-level if there's only one job.

### Files to update
1. `.github/workflows/ci.yml` — 1 job, add job-level env
2. `.github/workflows/evolve.yml` — 1 job, add job-level env
3. `.github/workflows/log-feedback.yml` — 1 job, add job-level env
4. `.github/workflows/pages.yml` — 2 jobs (build + deploy), add workflow-level env
5. `.github/workflows/release.yml` — 1 job with matrix, add job-level env
6. `.github/workflows/skill-evolve.yml` — 1 job, add job-level env
7. `.github/workflows/social.yml` — 1 job, add job-level env
8. `.github/workflows/synthesize.yml` — 1 job, add job-level env

### Also check
- `actions/cache@v4` used in evolve.yml, skill-evolve.yml, social.yml — these also run on Node.js 20. The `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` env var should cover them too.
- Any other third-party actions in the workflows that might be Node.js 20-dependent.

## Verification
- `cargo build && cargo test` must pass (workflow changes don't affect Rust code, but verify anyway)
- Review each file to ensure the YAML is valid (proper indentation, no syntax errors)
- Check that no other action versions in the workflows are pinned to Node.js 20 specifically

## Notes
- This is a mechanical change. Don't refactor workflows. Don't add or change any CI behavior.
- If `actions/checkout@v5` exists and is stable, using that is an alternative approach — but the env var approach is safer since it doesn't depend on upstream releases.
- The deadline is June 16, 2026. This should be done now, not deferred.
