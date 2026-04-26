---
name: analyze-trajectory
description: Diagnose a recurring failure (STUCK task, clustered CI error, frequent reverts) by dispatching sub-agents to digest CI logs without bloating main context. Returns one root-cause diagnosis.
tools: [bash, read_file, sub_agent]
core: true
origin: creator
---

# Analyze Trajectory

You are doing a **deep dive** into a recurring failure pattern. The harness's pre-computed `YOUR TRAJECTORY` block surfaces *that* something is recurring; this skill helps you understand *why* and produce a focused diagnosis.

This skill exists because raw GitHub Actions logs are too large and noisy to digest in your main context window. The pattern (Recursive Language Model — see Reithan's reference in issue #226) is: keep your root context small, dispatch a sub-agent to read the raw logs, and have the sub-agent return a 1-3 sentence summary. Recurse if the summary surfaces a deeper question.

## When to use

Trigger this skill when ANY of these hold:

- `YOUR TRAJECTORY` flagged a `STUCK` task (≥3 attempts in window, 0 successes)
- A CI error fingerprint appeared `≥2×` in the recurring-errors section
- The reverts-in-window count exceeds ~30% of recent sessions
- A specific issue (e.g. `#205`) has been mentioned in multiple session journals without resolution

## When NOT to use

- The trajectory looks healthy. Don't spelunk for problems that aren't there — that's just burning sub-agent budget.
- The failure is well-understood already (you already know the cause from journal/learnings). Skip straight to the fix.
- You're inside Phase B (implementation) and the failure is the task you're currently doing — fix it directly, don't recurse.

## Procedure

### 1. Frame the question (single sentence)

Examples of well-framed questions:
- *"Why does the evaluator phase fail with 'AnthropicError: rate_limit_exceeded' on sessions day-53, day-55, and day-56?"*
- *"Why was the task 'Add /fallback flag' reverted on 6 separate sessions? What's the recurring blocker?"*
- *"What does run 4321 look like at the moment of failure?"*

A good question names a specific event (run id, session day, error fingerprint) and what you want to know about it. Don't ask vague questions like *"what's wrong with my trajectory?"*

### 2. Identify the artifact

For each question, pick exactly one artifact to fetch:

- **CI failure** → run id from the trajectory's CI errors section. `gh run view <id> --repo $REPO --log-failed`
- **Reverted task** → commit SHA of the revert. `git show <sha>` and the next-newer commit's full diff
- **Session-level wreckage** → audit.jsonl from that session in `$YOYO_AUDIT_DIR/day-N-<ts>/audit.jsonl` (mounted by the harness when this skill is loaded as part of a skill-evolve cycle; otherwise fetch via worktree on `audit-log` branch)

### 3. Decide: direct read or sub-agent?

Estimate the artifact size first:
```bash
gh run view <id> --repo "$REPO" --log-failed 2>/dev/null | wc -c
```

- **< 5KB**: read it directly with `read_file` or `bash`. Skip sub-agent — the cost isn't worth it.
- **≥ 5KB**: dispatch a sub-agent. Don't load raw logs into your main context.

### 4. Dispatch a sub-agent (if needed)

Use the `sub_agent` tool with this template:

```
Question: <your single-sentence question from step 1>

Artifact (compressed log; do NOT include this in your reply, only summarize):
<paste the gh run view output here>

Return a JSON object only — no prose:
{
  "summary": "<1-3 sentence root cause as a quote-free string>",
  "key_lines": ["<file:line or log line that proves the cause>", "..."],
  "deeper_question": "<null OR a single follow-up question if the cause is still ambiguous>",
  "confidence": "<high|medium|low>"
}
```

Sub-agents inherit RTK compression on bash output and directory restrictions, but they do NOT inherit skills. Keep the sub-agent prompt fully self-contained — don't reference other skills.

### 5. Recurse if the sub-agent returns `deeper_question`

If `confidence == "low"` AND `deeper_question != null`, run another sub-agent dispatch with the narrower question. Reuse the same artifact; the sub-agent will focus differently.

**Hard cap: recursion depth = 3.** That's: initial dispatch → 1st recursion → 2nd recursion. After that, accept whatever you have. The cap matches the RLM blog's pattern and prevents runaway agent costs.

If you hit the cap without `confidence == "high"`, that's still a valid outcome — write the diagnosis with whatever clarity you have and flag it as "needs follow-up".

### 6. Aggregate to a single diagnosis

Produce a 3-5 sentence diagnosis paragraph that includes:
- **What recurs**: one-line summary of the pattern
- **Root cause** (or best-guess): from the sub-agent's summary
- **Evidence**: ≤3 specific lines or run IDs
- **Suggested next attempt**: one concrete action (a different approach, a new task, or "log to learnings.jsonl and skip for now")

Write the diagnosis somewhere durable:
- If you're in a normal evolve session and this informed your task choice → cite it in the assessment doc
- If you're investigating a specific issue → comment on the issue with the diagnosis
- Always also append a `learnings.jsonl` entry with `pattern_key: trajectory.<name>` so the pattern doesn't get rediscovered next session

## Pitfalls

- **Don't ask the sub-agent to make decisions.** It summarizes evidence; you decide what to do. Sub-agents in chained recursion can drift if asked to plan.
- **Don't recurse on `confidence: high`.** The whole point is to stop early when you have a clear answer.
- **Don't dump multiple artifacts to one sub-agent.** One artifact per dispatch keeps the sub-agent focused and the JSON output reliable.
- **Don't forget the recursion cap.** 3 is the hard limit. If you find yourself wanting depth 4, your initial question was probably too vague — go back to step 1.
- **Skills do not chain.** Sub-agents don't load this skill or any other; you must paste the question + artifact into the sub-agent's prompt directly.
- **Don't run this skill inside Phase B (implementation).** That's task-execution time, not introspection time. Save the diagnosis for the next session's Phase A1 (assess).

## Verification

A diagnosis is "good enough" when ALL of:
- It names a concrete file/line/condition (not "something with the API")
- It cites at least one specific run id or commit SHA
- The suggested next attempt is *different* from what's already been tried (otherwise you'll just hit the same wall)
- The total work used ≤3 sub-agent dispatches

If the diagnosis fails any of these, recurse one more time (within the cap) or accept the partial result and document the open question in `learnings.jsonl`.

## What this skill deliberately does NOT do

- **Does not modify code.** Diagnosis is the output. The actual fix is a normal task on a future evolve session — it's better to step away with the diagnosis written down and let the next session's planning agent decide whether to act on it.
- **Does not retire skills.** Skill lifecycle belongs to skill-evolve.
- **Does not auto-create issues.** If the diagnosis is worth filing, do it via `communicate` skill in the same session — but it's a separate decision, not part of this skill's procedure.
- **Does not write to `audit-log` branch.** The branch is read-only from this skill's perspective.
