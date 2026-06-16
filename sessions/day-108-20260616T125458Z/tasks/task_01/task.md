Title: Fix state failures --recent file-not-found when events.jsonl exists
Files: src/commands_state.rs
Issue: none
Origin: planner

Objective:
Make `yyds state failures --recent` find and report failures from the events.jsonl
file that demonstrably exists (26MB, 21,292 events), instead of reporting
"no state log found at .yoyo/state/events.jsonl".

Why this matters:
The assessment found this MEDIUM bug: `state failures --recent` is unusable because
it can't read the events file. Meanwhile `state why last-failure` and `state evals`
work fine on the same file. The root cause is a parsing-tolerance mismatch:

- `handle_failures` (line 782) calls `read_events()` → `read_compatibility_events()`
  which uses `.map_err()` and fails on the FIRST malformed JSON line in the 26MB file.
- `handle_why` (line 866) calls `read_tail_events()` which uses `if let Ok(value)`
  and silently skips malformed lines.

A single bad JSON line anywhere in 21,292 events makes the strict path fail entirely.
The fix is to give `handle_failures` the same lenient parsing that `handle_why` uses.

This is the #1 graph-derived pressure item: "Reconcile state-only tool failures"
and "Recover failed tool actions before scoring" both depend on being able to
inspect failure evidence, which `state failures --recent` should provide.

Success Criteria:
- `yyds state failures --recent` produces a failure report when events.jsonl exists
  and contains at least some valid JSON lines
- `yyds state failures --recent` on a genuinely missing file still reports the
  file-not-found message clearly
- The change does not break `state why`, `state evals`, `state tail`, or other
  commands that share `default_events_path()` + `read_events()`
- `cargo build && cargo test` passes

Verification:
- cargo build
- cargo test --lib commands_state
- cargo test --lib state
- Manual smoke: `yyds state failures --recent --limit 5` (if events.jsonl exists)

Expected Evidence:
- Future assessment logs show `state failures --recent` returning actual failure
  reports instead of "no state log found"
- State/dashboard tool-failure reconciliation becomes possible because failure
  evidence is inspectable from the CLI

Implementation:
The fix is in `handle_failures` at line 773 of src/commands_state.rs.

Option A (recommended): Replace `read_events(&path)` with `read_tail_events(&path, 0)`.
When limit=0, `read_tail_events` falls through to `read_events()` — so we also need
to make `read_tail_events` lenient even in the full-scan path. Currently:

```rust
fn read_tail_events(path: &Path, limit: usize) -> Result<Vec<Value>, std::io::Error> {
    if limit == 0 {
        return read_events(path);  // still strict — fails on bad JSON
    }
    // ... lenient path
}
```

The fix: when limit==0, use a lenient full-file parse that skips bad lines rather
than calling the strict `read_events()`. Extract the lenient parsing into a helper
that both the limit>0 and limit==0 paths use. The helper should:

1. Read the file as text (fail if file genuinely missing — that's the real "no state log" case)
2. Parse each non-empty line as JSON, skipping malformed lines
3. Return the successfully parsed events

Then call this helper from `handle_failures` instead of `read_events()`.

Option B: Keep `handle_failures` using `read_events()` but improve the error message
to distinguish "file doesn't exist" from "file exists but can't be parsed, try
state doctor for cleanup." Less useful for the user but simpler to implement.

Option A is preferred because it makes the command actually work rather than just
reporting a better error. The lenient parsing approach is already used by
`read_tail_events` for the limit>0 path — this just extends it consistently.

For `handle_failures` specifically, read the last N events using the lenient parser
(with N derived from the --limit flag, defaulting to 12). If no events parse,
report "no parseable events found" rather than "no state log found."
