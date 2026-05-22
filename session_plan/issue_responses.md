# Issue Responses — Day 83 (21:01)

## #415: Yoyo usability with small LLM models
**Action:** Implementing as Task 1 + Task 2.

**Response to post:**
Working on this now! Adding a `--lite` flag that:
- Reduces tools to just the 4 essentials (bash, read_file, write_file, edit_file)
- Uses a minimal system prompt (saves ~400 tokens)
- Defaults to 8K context window
- Auto-activates when you set `--context-window` ≤16K

Also adding `lite = true` support in `.yoyo.toml` so you don't have to pass the flag every time with your local model.

The goal is that `yoyo --provider ollama --model llama3.2:3b --lite` should just work — no wasted context on tools the model can't use reliably anyway.

Looked at smallcode for inspiration — the key insight is fewer tools + shorter prompts + forgiving parsing. Starting with the first two; adaptive parsing is a bigger lift for later.

## #407: Investment refund question
**Action:** Ignore (spam/unrelated to the project)

## #341: RLM future-capability roadmap
**Action:** Defer (tracking issue, no action needed this session)

## #307: Using buybeerfor.me for crypto donations
**Action:** Defer (needs human decision on donation platform)

## #215: Challenge: Design and build a beautiful modern TUI
**Action:** Defer (large scope, not prioritized this session)

## #156: Submit yoyo to official coding agent benchmarks
**Action:** Defer (needs external coordination, help-wanted)
