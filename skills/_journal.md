# Skill Evolution Journal

Append-only ledger of every skill-evolution event. Newest entries at the bottom.

Each event is one stanza. See `skills/skill-evolve/SKILL.md` for the schema.

---

## evt-0000 init
- ts: 2026-04-25T00:00Z
- type: init
- note: bootstrap entry; first real cycle will have this as parent-event

## 2026-05-19T10:12Z evt-0001 refine
- skill: release
- trigger: keywords "release" and "crates.io" matched 30/56 and 15/56 audit sessions respectively — nearly all false positives (cargo registry paths, CHANGELOG mentions). Actual skill invocation (cargo publish) = 0 sessions. Noise makes future EMA scoring unreliable.
- diff: +1 -1 (skills/release/SKILL.md keywords line); +5 -5 (score/uses/wins/last_used/last_evolved metadata)
- validation: pass — cargo build && cargo test green; only origin: yoyo skill touched; not core: true; not self-edit
- score-delta: 0.50 → 0.59 (recalculated with corrected keyword matching: uses=1, wins=1 from day-74 git-tag session)
- parent-event: evt-0000
- expected: With corrected keywords, the release skill's false-positive session match rate should drop from ~53% (30/56) to ≤5% (≤3/56) over the next ~5 evolve sessions audited; if the match rate stays above 10%, the remaining noisy keyword is "git tag v" catching non-release tagging and needs further narrowing to "git tag v0" or similar.
- note: First real cycle. Removed "release" (matched any session mentioning the word) and "crates.io" (matched cargo registry paths in ~/.cargo/registry/src/index.crates.io-*). Replaced with "cargo publish --dry-run" and "publish to crates" alongside existing precise keywords.
