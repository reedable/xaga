# Unprojected reads pull the vector column (pending)

`get_entry` (`crates/xaga-core/src/embeddings/db.rs`) loads every column including the 384-float vector. `list_entry_topics` reads an entry's `topics` list through it — an unprojected read paying vector width for a string list. Recorded fix direction: projected reads (select only the needed columns), as `state_ids` already does for the id-only sets. Other `get_entry` call sites likely share the pattern; enumerate them when picking this up.
