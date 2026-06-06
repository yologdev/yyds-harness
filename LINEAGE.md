# Lineage

agent_id: yyds
repo: yologdev/yyds-harness
generation: 1
root_ancestor: yologdev/yoyo-evolve
parent: yologdev/yoyo-evolve
parent_generation: 0
born: 2026-06-03
branch_point_commit: c0e981866c1396f3678e8910439df0a10b4ac3e2
first_gen1_commit: 766ea5489670c8839c630d6c47887313e12d8b36
role: DeepSeek-native harness branch
status: Active

## Meaning

yyds is generation 1 in the yoyo family tree. Gen0 yoyo lives in `yologdev/yoyo-evolve`; this repo is a child branch that inherited yoyo's runtime and self-evolution discipline, then specialized around DeepSeek-native reliability, deterministic prompt layout, evaluation-backed promotion, and durable harness state.

Runtime subagents, spawned conversations, and task workers are not family-tree descendants. Family generations refer to repository-level agent lineages: forks, branches, and descendants that carry their own identity, journal, memory, and evolution loop.

## Family Protocol

- A direct fork increments `generation` by 1 from its parent.
- A descendant records both `parent` and `root_ancestor`.
- A descendant keeps its own journal and should not treat the parent's journal as its current lived memory.
- Reusable lineage improvements should be proposed back to gen0 so future descendants can inherit them.
