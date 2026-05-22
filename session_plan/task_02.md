Title: Create blindspot skill — reusable code/architecture critique
Files: skills/blindspot/SKILL.md
Issue: #412

## What

Create a new `blindspot` skill that enables yoyo to systematically find blind spots in code,
architecture, database schemas, API designs, and deployment strategies. This is in response
to community issue #412 from @voku requesting a "Blind-Spot Roasting Skill."

This is differentiation territory — no competitor has a built-in self-critique/roasting skill.

## Key design decisions

- **origin: yoyo** — created by yoyo, eligible for future skill-evolve refinement
- **Not core** — this is a utility skill, not foundational
- **Tools**: bash, read_file, list_files, search, sub_agent (for large codebases, use RLM)
- **Invocation**: used during evolution self-assessment, or on-demand via `/skill run blindspot`
- **Scope**: the skill analyzes code/architecture the agent can see — it's not limited to yoyo's own source

## Skill structure

The skill should cover these analysis dimensions (from the issue, adapted):
1. **Error handling gaps** — unwrap() bombs, missing error paths, silent failures
2. **Security blind spots** — hardcoded secrets, injection risks, unsafe patterns
3. **Architecture debt** — god objects, circular dependencies, leaky abstractions
4. **Scalability risks** — O(n²) in hot paths, unbounded collections, missing pagination
5. **Testing gaps** — untested edge cases, missing integration tests, brittle test patterns
6. **API design issues** — inconsistent naming, missing validation, breaking change risks
7. **Dependency risks** — outdated deps, unnecessary deps, single points of failure

## Output format

The skill should produce a structured report:
```
🔍 BLINDSPOT REPORT — [target]

## Critical (fix now)
- [finding with file:line reference]

## Warning (fix soon)
- [finding]

## Smell (consider)
- [finding]

## Acknowledged (known, accepted)
- [items the codebase explicitly documents as accepted risks]
```

## Implementation

1. Create `skills/blindspot/SKILL.md` with proper YAML frontmatter
2. The skill should:
   - Accept a target (directory, file, or "self" for yoyo's own code)
   - Use sub_agent for large targets (>5KB analysis artifacts)
   - Output the structured report format above
   - Include a "roast level" parameter: gentle (warnings only), standard (all findings), brutal (nitpick everything)
3. Add keywords for skill-evolve tracking: ["blindspot", "roast", "critique", "audit", "security", "architecture"]
4. Log the creation event to `skills/_journal.md`

## Verification
- File exists at `skills/blindspot/SKILL.md`
- YAML frontmatter is valid (name, description, tools, origin, keywords)
- Skill journal entry added
- `cargo build && cargo test` still pass (no Rust changes)
