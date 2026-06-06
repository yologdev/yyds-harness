<p align="center">
  <img src="assets/banner.png" alt="Yoyo DeepSeek Harness" width="100%">
</p>

<p align="center">
  <a href="https://yologdev.github.io/yyds-harness/">Website</a> |
  <a href="https://yologdev.github.io/yyds-harness/evolution/">Evolution dashboard</a> |
  <a href="https://github.com/yologdev/yyds-harness/actions/workflows/evolve.yml">Evolution runs</a> |
  <a href="https://github.com/yologdev/yyds-harness/issues">Issues</a>
</p>

<p align="center">
  <a href="https://github.com/yologdev/yyds-harness/stargazers"><img src="https://img.shields.io/github/stars/yologdev/yyds-harness?style=flat" alt="stars"></a>
  <a href="https://github.com/yologdev/yyds-harness/actions"><img src="https://img.shields.io/github/actions/workflow/status/yologdev/yyds-harness/ci.yml?label=ci&logo=github" alt="ci"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="license MIT"></a>
  <a href="https://github.com/yologdev/yyds-harness/commits/main"><img src="https://img.shields.io/github/last-commit/yologdev/yyds-harness" alt="last commit"></a>
</p>

# Yoyo DeepSeek Harness

`yyds-harness` is a DeepSeek-native coding-agent harness that learns from its
own runs.

This repository,
[yologdev/yyds-harness](https://github.com/yologdev/yyds-harness), is
generation 1 in the yoyo family tree. Gen0 is
[`yologdev/yoyo-evolve`](https://github.com/yologdev/yoyo-evolve). This branch's
agent is named **yyds**. It keeps the inherited `yoyo` runtime compatibility
surface, then specializes the harness around DeepSeek models, state-backed
evidence, deterministic prompt layout, and evaluation-driven self-improvement.

The release artifact builds one binary:

```bash
yyds "fix the failing tests"
```

`yyds` is the gen1 product surface. It defaults to the DeepSeek-native harness
profile, so `--deepseek-native` is not needed.

## What This Repo Is For

The harness has two jobs:

1. Be a usable terminal coding agent.
2. Run autonomous evolution sessions that improve the harness itself.

The important gen1 focus is not just "make commits automatically." The goal is
to make each run leave enough evidence that the next run can make a better
choice.

That evidence lives in three places:

| Evidence | Where | Why it matters |
| --- | --- | --- |
| Journal | [`journals/JOURNAL.md`](journals/JOURNAL.md) | Human-readable history of what yyds tried. |
| State events | `audit-log` branch, per-session `state/events.jsonl` | Machine-readable run trace for replay and metrics. |
| Evolution dashboard | <https://yologdev.github.io/yyds-harness/evolution/> | Human-readable summary of sessions, metrics, and evidence. |

## How Evolution Works

On schedule or manual trigger, the `Evolution` workflow runs one harness session:

1. Load identity, lineage, active memory, and repo context.
2. Pick a small harness-improvement task.
3. Implement the change.
4. Run build/test/eval checks.
5. Commit only if the checks pass.
6. Save audit artifacts to the `audit-log` branch.
7. Run log feedback after GitHub Actions logs are complete.
8. Rebuild the evolution dashboard.

The important design point is the feedback loop:

```text
run -> state events -> gnome metrics -> dashboard/evidence -> next run prompt
```

The agent should not only remember that a session "passed." It should also see
where the run was slow, noisy, brittle, or under-instrumented.

## Gnome Metrics

Gnome metrics are the compact health signals that turn raw logs and state events
into useful evolution feedback. They are written into `PatchEvaluated` events and
summarized by `scripts/summarize_state_gnomes.py`.

They help yyds evolve the harness in four practical ways:

| Use | Example metrics | How it guides the next task |
| --- | --- | --- |
| Find weak spots | `distinct_failure_count`, `tool_error_count`, `json_error_count`, `state_failure_count`, `repair_loop_count` | Shows which friction happened during the run. |
| Score whether a change helped | `coding_log_score`, `workflow_success_rate`, `session_success_rate`, `task_success_rate` | Compares the latest run against the previous baseline. |
| Prioritize reusable fixes | `recurring_failure_count`, `max_failure_fingerprint_recurrence`, `closed_loop_fix_rate` | Promotes repeated friction into a harness-level fix. |
| Check the feedback loop itself | `coding_log_available`, `coding_log_confidence`, `state_capture_coverage`, `audit_capture_coverage` | Verifies that evidence was captured well enough to learn from it. |

The dashboard can therefore say something more useful than "CI passed." For
example:

```text
task_success_rate = 1.0
coding_log_score = 0.81
state_capture_coverage = 1.0
audit_capture_coverage = 1.0
failure fingerprints = timeouts and search errors
```

That means the session completed its tasks, but yyds should still consider a
future harness task like:

```text
Make state/search diagnostics timeout-safe and regex-safe.
```

This is the behavior we want: successful runs still produce learning pressure.

## What "0 Blockers / 1 Eval / 0 Patches" Means

The dashboard separates different event types:

| Count | Meaning |
| --- | --- |
| Blockers | Real policy blocks, failures, or rejected decisions that stopped progress. Allowed permission decisions are filtered out. |
| Evals | `PatchEvaluated` evidence, including log-feedback gnome metrics. |
| Patches | Explicit harness patch lifecycle events such as proposed/applied/promoted/rejected patches. |
| Refs | Code references emitted by state events, such as commits or patch artifacts. |

So `0 blockers / 1 eval / 0 patches / 0 refs` is not "nothing happened." It
means the run had no real blocker and no explicit patch-lifecycle record, but it
did emit one evaluation with numeric gnome metrics.

## DeepSeek Prompt Cache Rule

DeepSeek context caching is server-side. The harness should not add
request-side `cache_control` markers for DeepSeek.

What matters is stable prompt layout:

Good order:

1. Stable identity.
2. Stable safety/rules.
3. Stable tool/schema policy.
4. Stable repo/harness policy.
5. Mostly stable repo map.
6. Dynamic task.
7. Dynamic logs, selected files, and current evidence.

Bad order:

1. Current task timestamp/logs.
2. Random run/session metadata.
3. Stable identity/rules.
4. Stable repo policy.

The dynamic suffix can change without ruining the reusable stable prefix. See
[`docs/deepseek-prompt-cache-layout.md`](docs/deepseek-prompt-cache-layout.md).

## Quick Start

Install from source:

```bash
git clone https://github.com/yologdev/yyds-harness
cd yyds-harness
cargo install --path .
```

Or install the published crate:

```bash
cargo install yoyo-ds-harness
```

Run the DeepSeek-native surface:

```bash
DEEPSEEK_API_KEY=sk-... yyds
```

Run a one-shot prompt:

```bash
DEEPSEEK_API_KEY=sk-... yyds -p "summarize this repo"
```

## Configuration

General config can live in `.yoyo.toml`, `~/.yoyo.toml`, or
`~/.config/yoyo/config.toml`:

```toml
provider = "deepseek"
model = "deepseek-v4-pro"
thinking = "high"

[permissions]
allow = ["cargo *", "git status", "git diff *"]
deny = ["rm -rf *"]
```

DeepSeek-specific harness defaults can live in `.yoyo/deepseek.toml`:

```toml
[deepseek]
enabled = true
default_model = "deepseek-v4-pro"
fast_model = "deepseek-v4-flash"
base_url = "https://api.deepseek.com/v1"
thinking_default = "high"

[deepseek.cache]
stable_prefix = true
record_metrics = true
optimize_prompt_order = true

[deepseek.context]
recent_failure_limit = 5
changed_file_limit = 12
include_repo_map = true
include_instruction_files = ["YOYO.md", "AGENTS.md", "CLAUDE.md"]
```

## Useful Commands

Inside the REPL:

| Command | Purpose |
| --- | --- |
| `/help` | Show grouped command help. |
| `/status` | Show model, branch, modes, goal, changes, and context state. |
| `/health` | Run project build/test/lint checks. |
| `/fix` | Run checks and try to repair failures. |
| `/diff` | Show current git diff. |
| `/review` | Review current changes. |
| `/evolution` | Show local evolution/session summary. |
| `/cost` | Show model cost and token accounting. |
| `/tokens` | Show context usage. |

The full command reference lives in
[`docs/src/usage/commands.md`](docs/src/usage/commands.md).

## Grow Your Own Branch

To create another yoyo-family descendant:

1. Fork this repo.
2. Edit [`IDENTITY.md`](IDENTITY.md), [`LINEAGE.md`](LINEAGE.md), and
   [`PERSONALITY.md`](PERSONALITY.md).
3. Create a GitHub App for workflow commits.
4. Set secrets such as `DEEPSEEK_API_KEY`, `APP_ID`, `APP_PRIVATE_KEY`, and
   `APP_INSTALLATION_ID`.
5. Enable the `Evolution` workflow.

See [`docs/src/guides/fork.md`](docs/src/guides/fork.md) for the full guide.

## Repository Map

```text
src/                         Rust CLI, REPL, tools, commands, state, DeepSeek support
scripts/evolve.sh            Autonomous evolution session pipeline
scripts/log_feedback.py      GitHub Actions log feedback -> PatchEvaluated metrics
scripts/summarize_state_gnomes.py
                             State events -> gnome summary
scripts/build_evolution_dashboard.py
                             Static dashboard from audit-log sessions
.github/workflows/evolve.yml Evolution workflow
.github/workflows/log-feedback.yml
                             Post-completion log feedback workflow
.github/workflows/pages.yml  Website and dashboard deployment
docs/                        mdbook source
journals/                    Human-readable evolution journal
memory/                      Active and archived learnings
skills/                      Harness skills used by yyds
```

## Development

Run the main checks:

```bash
cargo fmt --check
cargo test
python3 scripts/log_feedback.py --test
python3 scripts/build_evolution_dashboard.py \
  --audit-sessions /tmp/yoyo-audit-log/sessions \
  --output-dir /tmp/yyds-dashboard
```

For release gates and fixture checks, see the CI workflow and
[`docs/deepseek-native-production-readiness-gap.md`](docs/deepseek-native-production-readiness-gap.md).

## Built On

[`yoagent`](https://github.com/yologdev/yoagent) provides the Rust agent loop.
[`yoagent-state`](https://crates.io/crates/yoagent-state) provides the state
event model used by this harness.

## Citation

```bibtex
@misc{yoyo2026yoyodsharness,
  title        = {Yoyo DeepSeek Harness: A DeepSeek-native coding agent harness that learns from its own failures},
  author       = {Yuanhao and {yyds}},
  year         = {2026},
  howpublished = {\url{https://github.com/yologdev/yyds-harness}},
  note         = {Open-source DeepSeek-native coding agent harness}
}
```

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=yologdev/yyds-harness&type=Date)](https://star-history.com/#yologdev/yyds-harness&Date)

## License

[MIT](LICENSE)
