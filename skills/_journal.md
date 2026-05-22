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

## 2026-05-21T09:00Z evt-0002 NO-OP
- ts: 2026-05-21T09:00Z
- type: NO-OP
- parent-event: evt-0001
- evidence-considered: 61 audit sessions mined across 6 eligible skills (explore-codebase, family, release, social, synthesis, x-research). No skill meets refine triggers (complaint_signals ≥ 2 or wins/uses < 0.5 with uses ≥ 3). No pattern_key reaches ≥3-session recurrence for create. All skills with true usage have 100% win rates. Score updates applied to 4 skills (explore-codebase 0.5→0.59, social 0.5→0.59, synthesis 0.5→0.59, x-research 0.0→0.24).
- keyword-noise-flagged: family (61/61 false positives — "yologdev/yoyo-evolve" matches every session, "fork" matches /fork CLI), synthesis (55/61 false positives — "sub_agent" and "research" are core agent tools). Wrote learning to memory/learnings.jsonl with pattern_key skill-evolve.keyword_noise for future cycle to act on once complaint threshold is met.
- note: release (last_evolved 2026-05-19) is within 3-session thrash guard and was skipped. Most skills have ≤3 true uses, all of which are creation-session or immediately-adjacent sessions — not enough signal to justify mutation.

## 2026-05-22T01:56Z evt-0003 create
- skill: blindspot
- ts: 2026-05-22T01:56Z
- type: create
- trigger: community issue #412 (@voku — "Blind-Spot Roasting Skill")
- origin: yoyo
- expected: skill is invoked during self-assessment or on-demand within the next 5 sessions; produces actionable findings that lead to at least one code fix. If unused after 10 sessions, keywords may need broadening.
- note: Created via skill-creator pattern during evolve session. Covers 7 analysis dimensions (error handling, security, architecture, scalability, testing, API design, dependencies). Supports roast levels (gentle/standard/brutal) and RLM dispatch for large targets.
