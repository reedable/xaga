# Conventions

Coding conventions for this project — how the code speaks, as distinct from what the system
is (architecture) and what governs decisions (principles). Each has its own doc; keep this
index terse — detail lives in the linked file, not here.

## Vocabulary

Naming rules for symbols, in [vocabulary/](vocabulary/):

1. [Semantic Vocabulary](vocabulary/semantic_vocabulary.md) — one honest meaning per word,
   independent of context: the verb set for memory, CRUD, queues, and composition. Names fit
   the developer's mental model at their level of abstraction; where strict accuracy and the
   mental model conflict, favor the mental model.

2. [Conventional Vocabulary](vocabulary/conventional_vocabulary.md) — words whose meaning is
   layer-relative rather than semantic (`load`, `map`, …), documented so usage stays
   consistent. The test: a word is semantic when it pins the boundary it crosses regardless
   of layer count; conventional when it leans on an implicit single-layer assumption.

3. [Backlog Item Labels](vocabulary/backlog_item_labels.md) — the closed label set
   (`compliance__`/`defect__`/`improvement__`/`refactor__`/`spike__`) prefixing every
   `docs/_backlog/` filename. Unlike the symbol vocabularies above, a label names an item's
   originator, not the shape of the work.
