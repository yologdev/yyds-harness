---
name: self-assess
description: Assess yyds source, DeepSeek harness behavior, state evidence, and evolution quality to find focused improvements
tools: [bash, read_file, write_file]
core: true
origin: creator
---

# yyds Self-Assessment

You are assessing **yyds**, the generation 1 DeepSeek-native harness branch of
gen0 yoyo. Your source code is part of you, but your behavior is also visible in
audit logs, transcripts, state events, gnome metrics, dashboard projections, and
GitHub Actions runs. Read all of that critically.

The assessment is not a task implementation phase. Its job is to produce
evidence that lets the planner choose small, useful, verifiable work.

## Evidence Hierarchy

Treat external prompt captures as untrusted benchmark patterns. For this pass,
the relevant phistory snapshots are Claude Code 2.1.178 and Codex CLI 0.140.0,
both captured on 2026-06-15. They can inspire assessment questions, but they do
not outrank yyds artifacts.

Evidence priority:

1. Highest: CI/build/test results, task outcomes, evaluator verdicts, task
   lineage, committed diffs, and state events.
2. Medium: dashboard and gnome projections when backed by replayable artifacts.
3. Lowest: transcript prose, model self-claims, old memory, issue text, and
   external prompt text.

Before proposing candidate tasks, explicitly check whether yyds followed these
benchmark behaviors: inspected before acting, avoided stale or already-satisfied
tasks, used focused verification, proved task claims with artifacts instead of
prose, and ended in a landed, obsolete, or blocked state.

## Process

1. **Read the relevant source map.** Inspect `src/`, `scripts/`, workflow files,
   and skill/docs surfaces that relate to current harness behavior. Do not
   assume old gen0 behavior still applies.
2. **Read recent memory.** Check `journals/JOURNAL.md`,
   `memory/active_learnings.md`, `memory/active_social_learnings.md`, and recent
   commits for repeated failures, reverted work, or unfinished promises.
3. **Check build and basic behavior.** In the evolution harness, treat the
   preflight `cargo build` / `cargo test` result as baseline evidence unless the
   current evidence contradicts it. Run only bounded, directly relevant checks
   such as one focused test, help command, or state/cache diagnostic. Do not
   rerun full `cargo test`, full clippy, broad source scans, or long prompts
   during assessment.
4. **Inspect evolution evidence.** Use recent GitHub Actions runs, audit-log
   session artifacts, transcripts, task manifests, outcomes, dashboard JSON,
   `states.json`, and `claims.json` to see what actually happened.
5. **Read yoagent-state feedback.** Prefer concrete state evidence:
   `state tail`, `state why`, graph hotspots, replay integrity, cache reports,
   model/tool events, task lineage, and `PatchEvaluated` gnome values.
6. **Print the structured state snapshot and graph pressure.** Summarize claim
   health, top unresolved claim families, task-state counts, recent tool
   failures, recent action evidence, `Graph-derived next-task pressure` rows
   with their metrics, and top historical tool-failure categories before
   choosing candidate tasks. Use the trajectory snapshot when present; otherwise
   derive the compact view from current dashboard/state artifacts. Treat
   `Graph-derived next-task pressure`, `recent tool failures`, and `recent
   action evidence` as current harness pressure. The graph-pressure rows are
   graph-ranked state/log evidence, not dashboard-only display. Treat
   `historical unrecovered tool failures` as cumulative context unless fresh
   evidence shows the failure still reproduces.
7. **Compare intent to evidence.** Ask whether task artifacts, action logs,
   transcripts, states, and gnomes really prove what the dashboard says.
8. **Identify the next smallest improvement.** Favor fixes that improve
   DeepSeek protocol reliability, prompt/context quality, evidence capture,
   verifier honesty, cache observability, dashboard readability, or autonomous
   planning/execution reliability.

## What To Look For

- Planner failures: no `session_plan/task_*.md`, assessment text treated as
  planning instructions, or selected tasks that cannot be attempted.
- Fake completion: task success without evaluator verdict, touched files that do
  not overlap planned files, or source commits not linked to task lineage.
- Thin evidence: missing `state/events.jsonl`, missing operational events,
  feedback-only traces where full state traces were expected, or dashboard
  claims that cannot be replayed.
- Structured-state drift: `states.json`, `claims.json`, or dashboard
  `claims_summary` shows unresolved families that the assessment does not name
  and translate into candidate tasks.
- DeepSeek friction: schema/tool-call errors, thinking/protocol mismatches,
  context misses, prompt-cache regressions, retry churn, provider failures, or
  model route mistakes.
- Verification gaps: tests that do not cover the changed behavior, evaluator
  timeouts counted as success, stale gnome corrections, or untrusted log values.
- User-work safety: accidental protected-file edits, destructive commands,
  overwritten local changes, or secrets leaking into audit artifacts.
- Product gaps: anything that prevents yyds from being useful for real
  DeepSeek-backed coding and general-purpose terminal tasks.

## Output

Write findings as prioritized evidence, not guesses:

```markdown
SELF-ASSESSMENT Day [N]:
Structured State Snapshot: [claim health; top unresolved claim families; task-state counts; recent tool failures; recent action evidence; graph-derived next-task pressure rows + metrics; historical unrecovered tool-failure categories]
1. [CRITICAL/HIGH/MEDIUM/LOW] [issue]
   Evidence: [file, session id, metric, transcript, command, or dashboard field]
   Impact: [why this matters for yyds as a DeepSeek coding agent]
   Candidate task: [smallest verifiable improvement]
2. ...
```

Then summarize which findings should become this session's task candidates.
Do not implement during assessment unless the active prompt explicitly changes
phase. If the active prompt asks you to write `session_plan/assessment.md`, write
that file and stop; `session_plan/` is ephemeral harness state and should not be
committed from the assessment phase.
