Title: Strengthen terminal marker visibility in implementation prompt template
Files: scripts/evolve.sh
Issue: none
Origin: planner

Evidence:
- Graph-derived next-task pressure row 3: "Emit terminal markers after verified commits (task_terminal_marker_missing_attempt_count=1): Implementation landed mechanical proof but omitted the exact TASK_TERMINAL_EVIDENCE marker."
- Trajectory: task_terminal_marker_missing_attempt_count=1 in the latest session (day-110)
- The harness-side strict matching landed Day 108 and is working — the check at scripts/evolve.sh line 388 enforces `grep -Eq '^[[:space:]]*TASK_TERMINAL_EVIDENCE:[[:space:]]*(changed|obsolete|blocked)[[:space:]]*$'`
- The prompt template at scripts/evolve.sh lines ~2160-2242 already mentions TASK_TERMINAL_EVIDENCE but 1 recent task still omitted it
- The template currently buries the marker format inside a paragraph — making it visually distinct (standalone block, bold markers) reduces omission risk

Edit Surface:
- scripts/evolve.sh (the implementation agent prompt template block around lines 2150-2250)

Verifier:
- bash -n scripts/evolve.sh (syntax check)
- grep -c 'TASK_TERMINAL_EVIDENCE' scripts/evolve.sh (should increase or format should change)

Fallback:
- If the marker format is already maximally prominent (standalone box with example), mark this task obsolete — the omission in day-110 was a one-off agent error, not a template weakness
- If the prompt template has been moved or refactored since this plan was written, search for `TASK_TERMINAL_EVIDENCE` in scripts/evolve.sh and apply the change to the current location

Objective:
Make the TASK_TERMINAL_EVIDENCE marker impossible to overlook in the implementation agent prompt by pulling it into a visually distinct block immediately before the final instruction, with an explicit example of the exact format required.

Why this matters:
The harness relies on TASK_TERMINAL_EVIDENCE markers as the contract between agent and harness — without them, the harness can't distinguish "task completed with changes" from "task abandoned mid-work." The strict harness-side check (Day 108) catches omissions, but catching them after the fact means the agent wasted a full implementation turn. Making the marker more prominent in the prompt prevents the omission before it happens.

Success Criteria:
- The TASK_TERMINAL_EVIDENCE format requirement appears as a visually distinct block (dedicated lines, not buried in a paragraph) near the end of the implementation prompt
- The exact required format is shown with an example: `TASK_TERMINAL_EVIDENCE: changed`
- The existing marker enforcement in the harness (line 388 grep) is not weakened or changed
- The prompt still includes the fallback/recovery instructions for when the marker was missed

Verification:
- bash -n scripts/evolve.sh
- grep -A5 'TASK_TERMINAL_EVIDENCE' scripts/evolve.sh | head -30 to confirm the marker block is visually prominent
- No changes to the grep enforcement at line 388

Expected Evidence:
- Future trajectory: task_terminal_marker_missing_attempt_count drops to 0
- The next session's implementation transcript shows the marker being emitted correctly on first attempt
- State events show fewer "missing terminal evidence" task rejections

Implementation Notes:
- The prompt template lives in scripts/evolve.sh, likely in a heredoc or python script block. Search for `TASK_TERMINAL_EVIDENCE` to find all occurrences.
- The change should extract the marker instruction into a dedicated block like:
  ```
  ═══════════════════════════════════════════════════════════════
  REQUIRED: Before your final message, emit exactly ONE of:
    TASK_TERMINAL_EVIDENCE: changed
    TASK_TERMINAL_EVIDENCE: obsolete
    TASK_TERMINAL_EVIDENCE: blocked
  This is a single line with no extra whitespace before or after.
  The harness reads this line to verify task completion.
  ═══════════════════════════════════════════════════════════════
  ```
- Keep the recovery instructions for when the harness rejects a missing marker (the "your first attempt was REJECTED" block)
- Do not change the harness-side grep pattern — this task only touches the agent-facing prompt template
