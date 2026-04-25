Title: Add system prompt section breakdown to /context tokens
Files: src/commands_project.rs
Issue: none

Currently `/context tokens` shows:
- system prompt total tokens
- conversation message count
- context used / max
- percentage and remaining

But it doesn't break down the system prompt into its component sections, which is
the most useful information for understanding where context budget goes. Users need
to know: how much is the base system prompt? How much is project context (CLAUDE.md)?
How much is memories? How much is the repo map?

**What to implement:**

In `show_context_tokens()` in `commands_project.rs`, replace the single "system prompt"
line with a breakdown of individual sections using `parse_prompt_sections()` (already
in the same file).

1. Call `parse_prompt_sections(system_prompt)` to get the sections.
2. For each section, estimate tokens using `estimate_tokens(&section_content)`.
3. Display each section name and its token count, indented under "system prompt":
   ```
     system prompt: ~4,200 tokens
       (preamble)         ~120
       Project context    ~2,800
       Git status         ~180
       Recently changed   ~400
       Project memories   ~700
     conversation:  12 messages
     context used:  45,000 / 200,000 tokens
     usage:         22%
     remaining:     ~155,000 tokens
   ```

4. Only show the section breakdown if there are more than 1 section (single-section
   prompts don't benefit from breakdown).

5. Add a test that verifies `show_context_tokens` doesn't panic with a multi-section
   system prompt, and optionally verify the output format includes section names.

**Note:** `parse_prompt_sections` is already in this file and returns `Vec<PromptSection>`
with `name`, `header_level`, and `lines` fields. `estimate_tokens` is also in this file.
No new dependencies needed.
