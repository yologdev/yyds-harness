Title: Add `is_token_backed()` method to DeepSeekUsage for cache ratio verification
Files: src/deepseek.rs
Issue: none
Origin: planner

Evidence:
- Trajectory graph pressure (Day 110): `deepseek_cache_ratio_unverified_count=1` — a DeepSeek cache hit ratio was reported in prose without token-backed cache metrics to independently verify it.
- `scripts/log_feedback.py` line 2548 self-tests this: `check("cache prose ratio marked unverified", cache_prose["deepseek_cache_ratio_unverified_count"] == 1, cache_prose)` — the test confirms the unverified detection exists but the underlying struct (`DeepSeekUsage`) has no method to expose token-backed status.
- `src/deepseek.rs` line 66-84: `DeepSeekUsage` has `cache_hit_tokens: Option<u64>` and `cache_miss_tokens: Option<u64>`. The `cache_hit_ratio()` method already returns `None` when either is missing. But there is no method to query *whether* the ratio is token-backed (both fields present) vs prose-derived (one or both fields absent).
- The cache report (`yyds deepseek cache-report`) works correctly (95.73% hit ratio from 209 events) — this task adds a verification building block, not a fix to broken behavior.

Edit Surface:
- src/deepseek.rs — `DeepSeekUsage` impl block (around line 73)

Verifier:
- cargo test deepseek
- cargo test -- deepseek_usage

Fallback:
- If `DeepSeekUsage` already has a method that exposes token-backed status (unlikely per current HEAD grep), close as already-done.

Objective:
Add `pub fn is_token_backed(&self) -> bool` to `DeepSeekUsage` that returns `true` when both `cache_hit_tokens` and `cache_miss_tokens` are `Some`, enabling downstream code (log_feedback.py, dashboard, cache-report) to distinguish prose-only cache claims from independently verifiable token-backed ratios.

Why this matters:
The trajectory's `deepseek_cache_ratio_unverified_count=1` means at least one session had a cache ratio claim that couldn't be verified against actual token counts. Without `is_token_backed()`, every consumer of cache ratios must independently check whether the ratio is backed by token data. This method provides a single source of truth. It's a prerequisite for future work that flags unverified ratios in scoring and cache reports.

Success Criteria:
- `DeepSeekUsage::default().is_token_backed()` returns `false`
- A `DeepSeekUsage` with both `cache_hit_tokens` and `cache_miss_tokens` set to `Some(n)` returns `true`
- A `DeepSeekUsage` with only one of the two fields set returns `false`
- The method appears in the public API of `src/deepseek.rs`
- All existing tests pass

Verification:
- cargo build
- cargo test deepseek
- cargo test -- deepseek_usage

Expected Evidence:
- A new `is_token_backed()` method on `DeepSeekUsage` with unit tests covering all three cases (both Some, one Some, both None)
- Future log_feedback.py or dashboard code can call this method (via state events that record DeepSeekUsage) to flag unverified ratios
- The `deepseek_cache_ratio_unverified_count` metric will become more reliable when downstream code consumes this method

Implementation Notes:
- Add method to the `impl DeepSeekUsage` block at line 73:
  ```rust
  pub fn is_token_backed(&self) -> bool {
      self.cache_hit_tokens.is_some() && self.cache_miss_tokens.is_some()
  }
  ```
- Add 3-4 focused tests in the existing `#[cfg(test)] mod tests` block (around line 2600+):
  1. Default DeepSeekUsage (cache_hit_tokens=None, cache_miss_tokens=None) → is_token_backed() == false
  2. Only hit tokens set → false
  3. Only miss tokens set → false
  4. Both set → true
- Do not modify any other code or add new public API beyond this one method.
