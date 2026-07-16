# Audit: decisions Claude logged in docs/\* without owner ruling

Raised 2026-07-11. The assistant wrote decision-flavored claims into durable docs that the owner never made. Each item below needs an owner ruling: ratify as written, reword to plain fact, or reverse.

## docs/architecture/security/embedding_model_trust.md

1. **"Deliberate, recorded" header (Limits section).** Deliberateness asserted by the assistant; no owner decision moment exists for it.
2. **Single-network anchoring recorded as a deliberate limit.** The anchoring procedure ran from one machine/network by the assistant's choice; never discussed as a decision point.
3. **Qdrant trust described as "accepted on reputation."** The reproduce-from-upstream alternative was mentioned in conversation and not pursued; the owner never ruled to accept the reputational link as a recorded limit.
4. **Provenance placed in the manifest comment rather than the doc.** Assistant's placement choice, undisclosed as a choice.
5. **Chain-link classifications (cryptographic / procedural / infrastructural / reputational).** Assistant's analysis, unreviewed by the owner.

## docs/architecture/security/fetching_remote_content.md

6. **CLI download proxy runs "settings at their secure defaults."** The fixed false/false settings were the assistant's implementation choice (disclosed in a code comment, not decided by the owner) and are described in the doc as design.
7. **Consumer-list intro reworded** from "Everything that touches attacker-influenced URLs" to "Every outbound consumer" — assistant's edit to the doc's framing, made as a "consistency fix."
8. **Model-download entry rationale wording** (constant URL, server-controlled redirects) — assistant's wording, unreviewed.

## docs/architecture/index.md

9. **Trust-doc summary sentence** (entry 7) — assistant's wording, unreviewed.

## Full tree scan (all 26 files under docs/\*_/_.md, 2026-07-11)

Every decision-claim about the system, regardless of author. Git cannot attribute authorship (all commits are the owner's); the owner recognizes which rulings are genuinely theirs. Definitional lines — the principles and conventions docs describing how decisions are made — are excluded; the criterion is "asserts that a design choice was made," not "mentions the word decision."

### Carries explicit owner-ruling provenance (1)

10. fetching_remote_content.md:271-278 — `max_redirects` default 3, a recorded Secure-by-Default exception: "the project owner reviewed that conflict and granted the exception on 2 July 2026." The only decision-claim in the tree with a named ruling moment.

### No provenance recorded — owner to recognize or rule (pre-existing)

11. layer_ownership.md:6 — "ratified as architectural decisions during the file-watcher retirement."
12. performance_tradeoff.md:12 — "the decision not to implement" conditional GET; :67 — teardown "deliberately," network cost "a fair price."
13. architecture/index.md:55-56 — mirrors item 12's claims ("rebuilt deliberately," "validators rejected").
14. fetching_remote_content.md:135 — clients "deliberately do not set `.https_only()`."
15. fetching_remote_content.md:184-185 — conditional-GET validators "rejected by decision, not missing."
16. handling_user_supplied_content.md:96 — `import_max_bytes` default 64 MiB "chosen from unmeasured" assumptions (choice flagged as unmeasured, chooser unnamed).
17. single_source_of_truth.md:40 — "rejected alternative" (topic membership in member frontmatter).
18. dry.md:8 — DRY "adopted here explicitly."
19. backlog_item_labels.md:29,32 — label vocabulary choices ("deliberately avoid `bugfix`/`feature`"; `improvement__` "chosen over `enhancement__`").

### Excluded as definitional (not decision-claims)

principles/decision_ownership.md, definition_of_priority.md, secure_by_default.md:42, least_visibility_principle.md, dry.md:22-43 (rule text), conventions/index.md, semantic_vocabulary.md, backlog_item_labels.md rule text, design_scale.md:29, docs/index.md, architecture/index.md:32, command_surface.md:29 and handling_user_supplied_content.md:49,85 ("attacker-chosen file" — not a design choice), fetching_remote_content.md:262,307 (mechanism descriptions).

## Scope note

Items 1-9: this conversation's changes (diffable against `dad6059`). Items 10-19: pre-existing text, authorship unattributable from git. Code comments asserting rationale or decisions ("deliberately," "per decision," "owner directive") across this week's code changes are NOT audited; auditing them is part of this item.
