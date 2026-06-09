Title: Gitignore context index files to stop 25MB of git churn
Files: .gitignore
Issue: none
Origin: planner

Objective:
Stop `.yoyo/context-semantic-index.json` and `.yoyo/context-embedding-index.json` from being tracked by git. These are computed caches (25MB combined) that regenerate on every session, causing constant dirty-tree noise and bloated diffs. The context system already handles missing indexes gracefully by rebuilding on demand.

Why this matters:
Every session, the context indexes regenerate and show up as modified in git status. The embedding index alone is 24MB. This churn creates noise in every session's working tree and risks bloated commits if accidentally staged. The context system in `src/context.rs` already has `ensure_embedding_index()` and `build_and_maybe_write_semantic_index()` that rebuild these files when they're missing or stale — they don't need to be in version control.

Success Criteria:
- `.yoyo/context-semantic-index.json` and `.yoyo/context-embedding-index.json` are in `.gitignore`
- Files are removed from git tracking (`git rm --cached`)
- `cargo build` succeeds (doesn't depend on these files being tracked)
- Git status shows a clean tree after the change (no untracked noise from these files)

Verification:
- `cargo build`
- `cargo test -- --test-threads=1` (quick subset: `cargo test context -- --test-threads=1`)

Expected Evidence:
- Clean `git status` after build — no modified context index files
- Context system still loads/writes indexes to `.yoyo/` directory at runtime

Add `.yoyo/context-semantic-index.json` and `.yoyo/context-embedding-index.json` to `.gitignore`, then remove them from git tracking with `git rm --cached`. The context system in `src/context.rs` already rebuilds these files on demand when they're missing or stale — no code changes needed. This is a pure repo-cleanliness fix.
