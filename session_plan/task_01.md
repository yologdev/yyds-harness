Title: Relevance-ranked repo map for system prompt — prioritize recently-modified files
Files: src/commands_map.rs
Issue: none

## Goal

Make `generate_repo_map_for_prompt_with_limit` sort files by relevance before truncating,
so that when the map exceeds `max_chars`, the most useful files survive the cut.

## Current behavior

`build_repo_map` returns files in alphabetical order (from `list_project_files()`).
When the map is too large, `generate_repo_map_for_prompt_with_limit` includes files in that
order until hitting the limit, then truncates with `...`. This means files starting with
`a-c` always survive while files starting with `s-z` (like `src/tools.rs`, `src/watch.rs`)
may be cut.

## Desired behavior

Before the truncation loop, sort the `entries: Vec<FileSymbols>` by a relevance score:
1. **Recently modified** (from `git log --format='%H' --diff-filter=M -1 -- <path>` is too expensive for many files). Instead, use `git log --name-only --format= -n 50` to get the last ~50 modified paths, then rank files that appear in that list higher.
2. **Symbol density** — files with more symbols per line are likely more architecturally important.
3. **File size** — larger files are more likely to be important.

Implementation approach:
- Add a helper `fn recently_modified_files(n: usize) -> Vec<String>` that runs `git log --name-only --format= -n {n}` and deduplicates the output into an ordered list of recently-touched paths.
- In `generate_repo_map_for_prompt_with_limit`, after getting entries from `build_repo_map`, compute a score for each entry:
  - `recency_score`: position in recently_modified_files list (higher = modified more recently). Files not in the list get 0.
  - `density_score`: `symbols.len() * 100 / lines` (capped at 50)
  - `size_score`: `min(lines / 50, 20)` (larger files get a bump, capped)
  - Total = `recency_score + density_score + size_score`
- Sort entries by score descending, then truncate as before.
- The `/map` command output (handle_map) should NOT be reordered — it stays alphabetical for human readability. Only the prompt injection version gets relevance ranking.

## Tests to add

- `test_recently_modified_files_returns_deduped_paths` — verify the git helper returns expected format (may need to mock or use temp dir)
- `test_relevance_score_prefers_recent_files` — create FileSymbols with known attributes, verify sorting
- `test_generate_repo_map_with_limit_truncates_least_relevant` — with a small max_chars, verify that high-relevance files survive

## Constraints

- Only modify `src/commands_map.rs`
- The git command must be fast (single git invocation, no per-file lookups)
- If git fails (not a git repo), fall back to current alphabetical order
- Must pass `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
