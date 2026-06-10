Title: Fix recurring grep escape errors in evolve.sh
Files: scripts/evolve.sh
Issue: none
Origin: planner

Objective:
Fix the recurring "grep: unmatched ( or \\(" error that appears 3× in the CI trajectory feedback. This is a harness noise issue — the evolution pipeline's grep calls use literal parentheses in patterns without proper escaping, causing spurious error output that pollutes CI logs and may cause grep to silently fail on some pattern matches.

Why this matters:
The trajectory log feedback explicitly flags this as a repeated issue: "search error: grep: unmatched ( or \\( 3×". This is harness-level noise that:
- Pollutes CI logs with spurious error output
- May cause grep to return incorrect results when patterns with parens are used
- Adds noise to the feedback pipeline, making real errors harder to spot
- Is a simple fix (escape or use grep -F) that has been recurring without resolution

This is a classic "small duplicated unit survives longest" pattern — a trivial fix that keeps getting deferred because it never blocks a session.

Success Criteria:
- grep calls in evolve.sh that use literal parentheses in patterns are fixed (either escaped or switched to grep -F / fgrep)
- No new grep errors appear when the evolve pipeline runs
- All existing evolve.sh functionality is preserved
- The fix is minimal — don't restructure the script, just fix the grep calls

Verification:
- grep -n 'grep.*(' scripts/evolve.sh | grep -v 'grep -F' | grep -v 'grep -E' to find unescaped paren patterns
- Manual review that each fixed grep call is correct
- If possible, run a dry-run of the affected evolve.sh sections
- No cargo build needed (script-only change)

Expected Evidence:
- Next CI run shows zero "grep: unmatched (" errors in the trajectory feedback
- Log feedback score remains stable or improves (less noise)

Detailed plan:

The implementation agent should:
1. Search scripts/evolve.sh for grep calls that use literal parentheses without -F flag or proper escaping
2. For each occurrence, determine the intent:
   - If it's a literal search (looking for actual paren chars in text), switch to `grep -F` or `fgrep`
   - If it's a regex where parens are meant as grouping, switch to `grep -E` or escape them
   - If it's a basic regex where parens should be literal, escape them with backslash: `\(` and `\)`
3. The safest approach is to use `grep -F` for any pattern that contains literal parentheses — this avoids regex interpretation entirely
4. Verify with: `bash -n scripts/evolve.sh` (syntax check)
5. If feasible, run the specific grep commands in isolation to verify they work

The fix should be conservative: change only the grep invocations that are causing errors, don't restructure the script.
