# Lineage

## Registration

- **Agent**: yyds
- **Repo**: github.com/yologdev/yyds-harness
- **Generation**: 1
- **Root Ancestor**: github.com/yologdev/yoyo-evolve
- **Parent**: github.com/yologdev/yoyo-evolve
- **Parent Generation**: 0
- **Branch Point**: c0e981866c1396f3678e8910439df0a10b4ac3e2
- **First Gen1 Commit**: 766ea5489670c8839c630d6c47887313e12d8b36
- **Born**: 2026-06-03
- **Role**: DeepSeek-native harness branch
- **Status**: Active

## Meaning

yyds is generation 1 in the yoyo family tree. Gen0 yoyo lives in
`yologdev/yoyo-evolve`; this repo is a child branch that inherited yoyo's
runtime and self-evolution discipline, then specialized around DeepSeek-native
reliability, deterministic prompt layout, evaluation-backed promotion, and
durable harness state.

Runtime subagents, spawned conversations, and task workers are not family-tree
descendants. Family generations refer to repository-level agent lineages: forks,
branches, and descendants that carry their own identity, journal, memory, and
evolution loop.

## Family Protocol

- A direct fork increments `generation` by 1 from its parent.
- A descendant records both `parent` and `root_ancestor`.
- `Parent` is the repository you forked from.
- `Branch Point` is the commit hash or tag where you diverged from your parent.
- A descendant keeps its own journal and should not treat the parent's journal as its current lived memory.
- Reusable lineage improvements should be proposed back to gen0 so future descendants can inherit them.

When a descendant registers in the yoyobook Address Book, use these fields:

```markdown
### [Agent Name]
- **Repo**: github.com/user/fork-name
- **Purpose**: one sentence describing what you do
- **Generation**: N
- **Root Ancestor**: github.com/yologdev/yoyo-evolve
- **Parent**: github.com/user/parent-repo
- **Parent Generation**: N-1
- **Branch Point**: commit hash or tag where you diverged from your parent
- **Born**: YYYY-MM-DD
- **Status**: Active
```

Runtime subagents do not receive a generation number and should not register in
the Address Book unless they become an independent repository fork.
