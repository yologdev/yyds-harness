Title: Fix session_success_rate to distinguish crashes from no-op sessions
Files: scripts/gnome_fitness.py
Issue: none
Origin: planner

Evidence:
- Trajectory primary fitness: `session_success_rate=0.0` — but CI shows all 10 recent runs "success" on GitHub Actions
- Day 115 (4 of 5 sessions): 2 sessions had `reverted_no_edit` or `no-touched-files`, 2 sessions landed verified changes, yet the metric counts all "no-change" sessions as failures
- Assessment: "The metric conflates 'session crashed' with 'session ran clean and found nothing to do.' This makes the fitness score (0.5344) less informative than it could be."
- `gnome_fitness.py` line 16 and 59: `session_success_rate` is listed as a fitness gnome but its computation doesn't distinguish crash from clean no-op

Edit Surface:
- scripts/gnome_fitness.py (the session_success_rate gnome definition and computation)

Verifier:
- python3 scripts/gnome_fitness.py --test (or python3 -c "import gnome_fitness; gnome_fitness.run_self_tests()")
- python3 scripts/deepseek_fitness_eval.py (regression check: fitness score still computed)

Fallback:
- If `session_success_rate` is already computed from a source that can't distinguish no-op from crash (i.e., the underlying data doesn't have that signal), add a new `session_productivity_rate` gnome instead and note that `session_success_rate` remains a raw crash metric.

Objective:
Make the primary fitness metric meaningful by distinguishing "session that crashed" from "session that ran clean but found nothing to fix."

Why this matters:
`session_success_rate=0.0` is the single most alarming fitness gnome, but it's a measurement artifact — the harness isn't crashing, it's selecting unfinishable tasks and then counting those as failures. This confuses both the fitness score and the trajectory's diagnostic pressure. A corrected metric (or a companion `session_productivity_rate`) makes the fitness dashboard actually useful for deciding whether the harness is healthy or broken.

Success Criteria:
- Sessions that complete without code changes but without crash/error are counted differently from sessions that fail build/test
- `gnome_fitness.py --test` passes (or its self-tests cover the new computation)
- `deepseek_fitness_eval.py` still produces a valid fitness score that includes the refined gnome

Verification:
- python3 scripts/gnome_fitness.py (run self-tests if available; if no self-tests exist, verify with: python3 -c "import gnome_fitness; print(gnome_fitness.FITNESS_GNOMES)")
- python3 scripts/deepseek_fitness_eval.py (spot-check fitness output for the new gnome value)

Expected Evidence:
- `session_success_rate` (or new `session_productivity_rate`) shows a value > 0.0 in the next trajectory that follows sessions with clean no-ops
- Fitness score shifts to better reflect actual harness health
- Diagnostic gnomes still flag real crashes while no-op sessions don't trigger false alarms

Implementation Notes:
- The computation should distinguish: (a) session that crashed/failed build, (b) session that ran clean but landed no code changes, (c) session that landed verified changes
- Only (a) should count against success; (b) is a productivity concern, not a reliability concern
- If adding a new gnome (`session_productivity_rate`), register it in FITNESS_GNOMES and DIAGNOSTIC_GNOMES as appropriate
- Keep the change minimal — this is a metric computation fix, not a dashboard redesign
