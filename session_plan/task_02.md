Title: Add missing model pricing for GPT-5 family and Grok-4
Files: src/format/cost.rs
Issue: none

## Problem

The model registry in `providers.rs` lists GPT-5, GPT-5-mini, GPT-5.5, GPT-5.5-mini, and Grok-4 as known models, but `cost.rs` has no pricing entries for any of them. This means `/cost` shows no cost estimate for users on these models. Both Aider and Claude Code already support these models with pricing.

## What to add

Add pricing entries to the `model_pricing()` function in `src/format/cost.rs`:

### GPT-5 family (OpenAI pricing as of May 2026)
Based on OpenAI's published pricing (https://platform.openai.com/docs/pricing):
- `gpt-5`: $2.00 input / $8.00 output per MTok (same tier as GPT-4.1 / o3)
- `gpt-5-mini`: $0.40 input / $1.60 output per MTok (same tier as GPT-4.1-mini)
- `gpt-5.5`: $5.00 input / $20.00 output per MTok (frontier reasoning model)
- `gpt-5.5-mini`: $1.00 input / $4.00 output per MTok

Add these as a block after the existing o3/o4-mini entries:

```rust
// GPT-5 family
if model.starts_with("gpt-5.5") {
    if model.contains("mini") {
        return Some((1.00, 0.0, 0.0, 4.00));
    } else {
        return Some((5.00, 0.0, 0.0, 20.00));
    }
}
if model.starts_with("gpt-5") {
    if model.contains("mini") {
        return Some((0.40, 0.0, 0.0, 1.60));
    } else {
        return Some((2.00, 0.0, 0.0, 8.00));
    }
}
```

Note: `gpt-5.5` MUST be checked before `gpt-5` because `gpt-5.5`.starts_with("gpt-5") is true.

### Grok-4 (xAI pricing)
Based on xAI's published pricing:
- `grok-4`: $3.00 input / $15.00 output per MTok (same tier as Grok-3)

Add after the existing grok-3 block:

```rust
if model.contains("grok-4") {
    return Some((3.00, 0.0, 0.0, 15.00));
}
```

Note: Place `grok-4` check BEFORE `grok-3` check, or use exact matching, to avoid `grok-4` falling through to the grok-3 branch (since "grok-4" doesn't contain "grok-3", the current if-chain actually works fine — but ordering grok-4 first is cleaner).

### Also add to model_context_window in commands_info.rs — NOT NEEDED
The existing `model_context_window` already handles these via broad prefix matching (`model.contains("gpt-5")` → 1M, `model.contains("grok")` → 131k). No changes needed there.

## Tests to add

Add unit tests for the new pricing entries:

```rust
#[test]
fn test_gpt5_pricing() {
    let usage = Usage { input: 1_000_000, output: 1_000_000, cache_write: 0, cache_read: 0 };
    let cost = estimate_cost(&usage, "gpt-5").unwrap();
    assert!((cost - 10.0).abs() < 0.01); // $2 + $8
}

#[test]
fn test_gpt5_mini_pricing() {
    let usage = Usage { input: 1_000_000, output: 1_000_000, cache_write: 0, cache_read: 0 };
    let cost = estimate_cost(&usage, "gpt-5-mini").unwrap();
    assert!((cost - 2.0).abs() < 0.01); // $0.40 + $1.60
}

#[test]
fn test_gpt55_pricing() {
    let usage = Usage { input: 1_000_000, output: 1_000_000, cache_write: 0, cache_read: 0 };
    let cost = estimate_cost(&usage, "gpt-5.5").unwrap();
    assert!((cost - 25.0).abs() < 0.01); // $5 + $20
}

#[test]
fn test_grok4_pricing() {
    let usage = Usage { input: 1_000_000, output: 1_000_000, cache_write: 0, cache_read: 0 };
    let cost = estimate_cost(&usage, "grok-4").unwrap();
    assert!((cost - 18.0).abs() < 0.01); // $3 + $15
}
```

## Important: Verify prices

The implementation agent MUST use `curl` to check the current OpenAI and xAI pricing pages before writing the final values. The prices listed above are estimates based on the assessment — verify them against the actual published pricing. If a model's price can't be confirmed, use the closest comparable model's pricing and add a comment noting it's estimated.

## Verification

- `cargo build && cargo test` must pass
- `cargo test test_gpt5` should pass
- `cargo test test_grok4` should pass
