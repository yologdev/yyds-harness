Title: Make healthy-codebase fallback rotate target files instead of always picking src/state.rs
Files: scripts/preseed_session_plan.py
Issue: none
Origin: planner

Evidence:
- Assessment Day 153 found the `_healthy_codebase_fallback()` function at line 1279 hardcodes `"files": "src/state.rs"` as its only target file. Every session where the assessment phase is missing or produces analysis-only output will try to modify state.rs, even when state.rs is healthy.
- This creates the self-referential "fix yourself" treadmill that Day 115's lesson warned about: "A fallback that responds to 'nothing is broken' by modifying the tool that looks for broken things is a self-referential cycle."
- `choose_task()` calls `_healthy_codebase_fallback()` from three code paths (lines 998, 1001, 1212) — each time it returns the same hardcoded task targeting `src/state.rs`.
- The trajectory shows `task_spec_warning_count=1` with graph pressure to "Tighten selected task specs" — rotating targets directly improves task spec quality by giving the implementation agent a fresh file to work on.
- The Day 153 10:40 simplification (commit 7fc97e27) was correct in spirit — stop diagnosing, start building — but incomplete in execution because it created a monoculture.

Edit Surface:
- scripts/preseed_session_plan.py

Verifier:
- python3 -c "
import sys; sys.path.insert(0, 'scripts')
import preseed_session_plan as p
# Call the fallback 6 times and collect target files
targets = set()
for _ in range(6):
    t = p._healthy_codebase_fallback()
    targets.add(t['files'])
# Should see at least 2 different target files if rotation works
assert len(targets) >= 2, f'Only saw targets: {targets} — rotation not working'
print(f'OK: rotation produces {len(targets)} distinct targets: {sorted(targets)}')
"

Fallback:
- If `_healthy_codebase_fallback` has already been updated to rotate (e.g., by a concurrent session), verify with the rotation check above, mark this task obsolete, and move on.

Objective:
Teach `_healthy_codebase_fallback()` to pick from a rotating list of high-value source files instead of always returning `src/state.rs`, so missing-assessment sessions don't loop on the same file.

Why this matters:
When `_healthy_codebase_fallback()` always targets `src/state.rs`, consecutive sessions with missing assessments all try to modify the same file. If state.rs is already healthy, the implementation agent either makes unnecessary edits (surface area for bugs) or marks the task as obsolete — wasting an evolution slot. A small rotation breaks the self-referential cycle. This directly addresses the Day 115 lesson about fallback self-reference and the Day 114 lesson about quiet sessions needing meaningful tasks.

Success Criteria:
- `_healthy_codebase_fallback()` returns a different target file on successive calls within the same session (using a simple round-robin or time-based rotation across a fixed list of high-value source files).
- The rotating list includes at least 4 files: `src/state.rs`, `src/deepseek.rs`, `src/tool_wrappers.rs`, `src/prompt.rs`.
- The fallback task's objective text is generic enough to work for any target file (e.g., "Add one focused unit test, doc comment, or micro-improvement to <file>").
- The task still passes strict verification (cargo build && cargo test) because all listed files are compiled Rust source.

Verification:
- python3 -c "import sys; sys.path.insert(0,'scripts'); import preseed_session_plan as p; targets={p._healthy_codebase_fallback()['files'] for _ in range(6)}; assert len(targets)>=2, f'no rotation: {targets}'; print(f'OK: {sorted(targets)}')"
- python3 scripts/preseed_session_plan.py --help (syntax check, no regressions)

Expected Evidence:
- Future trajectory shows healthy-codebase fallback tasks targeting different src/ files across sessions, not just src/state.rs.
- task_spec_warning_count stays low because rotated tasks give implementation agents fresh surface area.
- No self-referential treadmill: the fallback task no longer asks the agent to modify the planning script that produced it.

Implementation Notes:
- Add a module-level list `_HEALTHY_FALLBACK_FILES` above `_healthy_codebase_fallback()` with the rotation candidates. Use `itertools.cycle` or a simple `_fallback_index % len(files)` with a module-level counter.
- Keep the task template identical except for the `files` field and the corresponding file name in `objective`.
- The `verification` field should use `cargo test <module_name>` matching the selected file (e.g., `cargo test state` for state.rs, `cargo test deepseek` for deepseek.rs, etc.) — derive this from the file basename.
- Do NOT change any other function in the file. Keep the diff minimal (~15-25 lines).
- The existing `render_task()` function consumes the dict keys — ensure no key rename breaks it.
