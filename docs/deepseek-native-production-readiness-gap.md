# DeepSeek Native Production Readiness Gap

Date: 2026-06-02

This ledger maps the broader production plan to the current private MVP branch.
It does not replace `docs/deepseek-native-bootstrap-audit.md`; it records what
is still unproved or intentionally outside the bootstrap PR.

## Current Review State

- Private repository: `yologdev/yyds-harness`.
- Rust package: `yoyo-ds-harness`.
- Bootstrap PR: <https://github.com/yologdev/yyds-harness/pull/1>.
- Branch: `deepseek-native-bootstrap`.
- Latest PR commit before this audit update: `b5c4831`.
- PR state: open and mergeable against `main`.
- GitHub checks: none reported in this environment.

## Proved By Current Evidence

- The fork keeps `yoyo` compatibility and adds the `yoyo-ds` binary.
- The project consumes released `yoagent = "0.8.3"` and sibling
  `yoagent-state = { path = "../yoagent-state" }`.
- `src/state.rs` is a local adapter/projection over `yoagent-state`, not an
  upstream fork.
- DeepSeek-native profile, v4 model defaults, protocol diagnostics, JSON/FIM
  helpers, cache metrics, strict schema probes, and thinking-protocol replay
  diagnostics exist and are covered by local tests.
- State shadow mode writes canonical JSONL through `yoagent_state`, projects
  into local query/report surfaces, and remains fail-soft by default.
- Local eval fixtures, comparison/reporting, release gates, harness patch
  lifecycle commands, promotion/rejection decisions, and rollback surfaces
  exist for bootstrap use.
- A local isolated harness lifecycle dogfood run completed on 2026-06-02:
  - patch id: `patch-1780408949061-80074`
  - proposed event: `event_667c161162114767a77e3dc7871885f7`
  - applied event: `event_441fe2af9a2d48ea8b270b859831ce33`
  - eval id: `eval-1780409081139-80229`
  - evaluated event: `event_0d888e0a85b74df7aace5c96a7197ec2`
  - rollback event: `event_00ba03ba9d1b46fa86dd9dd7b420ec78`
  - rejected event: `event_80b76381d43942439bf9573cfdb1cefd`
  - rejection decision event: `event_704920a65eed4a9189431ad0eae96f38`
  - `yoyo state lineage patch-1780408949061-80074` and
    `yoyo state why patch-1780408949061-80074` both reconstructed the
    lifecycle from the isolated worktree state log.
  - The candidate eval failed, so the patch was rolled back and rejected rather
    than promoted. This proves rejection/rollback lineage, not positive
    promotion.
- Local gate set passed on the PR tree:
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo run --quiet --bin yoyo -- eval fixtures validate --suite local-smoke`
  - `test ! -e .yoyo`
  - `git -C /Users/yuanhao/Dev/yoagent-state status --short`
- Local release packaging dry run passed on 2026-06-02 for
  `aarch64-apple-darwin`:
  - `cargo build --release --bins`
  - `target/release/yoyo --version` ->
    `yoyo v0.1.13 (b5c4831 2026-06-02) macos-aarch64`
  - `target/release/yoyo-ds --version` ->
    `yoyo v0.1.13 (b5c4831 2026-06-02) macos-aarch64`
  - archive:
    `/private/tmp/yyds-release-dry-run/yyds-harness-dry-run-aarch64-apple-darwin.tar.gz`
  - archive contents: `yoyo`, `yoyo-ds`
  - sha256:
    `5a8da88194ca120c775828dfebd9c6eb2e6dc9b3e6ace6eab0e714b3246d6d51`

## Not Yet Proved

- Live DeepSeek end-to-end agent run. `DEEPSEEK_API_KEY` is not present in this
  environment, so the required smoke remains manual:
  `DEEPSEEK_API_KEY=... cargo run --bin yoyo-ds -- --deepseek-native "<small task>"`.
- GitHub Actions execution on PR #1. `gh pr checks` reports no checks, and
  `gh workflow list` returned no workflows in this environment. Repo owner/admin
  settings should verify Actions visibility and default branch behavior.
- Production-grade streaming/tool-call reliability against live DeepSeek. The
  current evidence is mock/replay/local-test coverage plus one pending live
  smoke requirement.
- Real daily-use reliability of git commit/revert flows under DeepSeek-native
  operation. Local unit/integration tests pass, but sustained dogfooding evidence
  is still missing.
- Long-running yoyo evolution: scheduled harness evals, self-filed improvement
  issues, automated memory synthesis, state-backed journal generation, and
  regression replay before releases are represented by bootstrap surfaces and
  fixtures, not proven production loops.
- Positive harness improvement promotion. The isolated dogfood lifecycle proved
  proposed -> applied -> evaluated -> rolled back -> rejected, but did not prove
  promotion of an improving patch.
- Public release readiness. The repository is intentionally private for MVP; the
  local release packaging path has been exercised, but GitHub release workflows,
  public badges, crates.io publishing, and public docs should be rechecked when
  the repo is made public.

## Production Readiness Gate

Do not mark the full production plan complete until all of the following have
current evidence:

- Live DeepSeek smoke passes and records usable state lineage.
- PR checks or equivalent CI evidence pass on the exact merge candidate.
- Repo default branch and branch protection match the intended `main` production
  line.
- At least one positive harness patch is proposed, evaluated, compared, and
  promoted with state evidence from an isolated worktree.
- GitHub release workflow or public-release dry run passes when the repo is
  ready to publish.
- Secrets/redaction and source-provenance scans pass on the release candidate.
- A short dogfooding report confirms the agent can complete a small real coding
  task with DeepSeek-native mode.
