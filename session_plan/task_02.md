Title: Fuzzy memory search with relevance scoring
Files: src/memory.rs, src/commands_memory.rs
Issue: none

The `/memories <query>` command currently does case-insensitive substring matching via
`search_memories()`. This is a competitive gap — Copilot has persistent memory with semantic
search. We can't do semantic search locally, but we can dramatically improve retrieval with
fuzzy matching and relevance scoring.

## What to implement

### 1. Add fuzzy scoring to `search_memories()` in `src/memory.rs`

Replace the current substring-only filter with a scoring function that:
- Gives highest score to exact substring matches (current behavior preserved)
- Also matches when all query words appear in the note (word-level AND matching)
- Scores based on: word match ratio, position of match (earlier = better), recency of memory
- Returns results sorted by score (highest first) instead of insertion order
- Still returns `Vec<(usize, &'a MemoryEntry)>` — same signature, but sorted by relevance

The scoring function should be a separate `fn fuzzy_score_memory(note: &str, query: &str) -> Option<f64>`
that returns None for non-matches and Some(score) for matches.

Matching rules:
- If the query is a single word: substring match (case-insensitive)
- If the query has multiple words: each word must appear as a substring in the note (AND semantics, case-insensitive)
- Score boosters: consecutive word matches, match at word boundary, shorter note (more focused)

### 2. Update display in `src/commands_memory.rs`

When showing search results, show them in relevance order (already handled by sorted return).
No other display changes needed — the output format stays the same.

### 3. Add tests

Add tests for `fuzzy_score_memory()`:
- Single word exact match → high score
- Multi-word all present → match
- Multi-word partial (not all present) → no match
- Score ordering: exact > word-boundary > mid-word
- Empty query → match everything (or return all, with equal scores)
- Case insensitivity

Add tests for updated `search_memories()`:
- Results are sorted by relevance (exact match first)
- Multi-word query filters correctly
- Backward compatible: single-word substring still works

This is a self-driven competitive improvement: making memory queryable is a gap vs Copilot's
persistent Memory feature.
