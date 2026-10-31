# Tagging Beyond Entries

Tags can theoretically organize Feeds, Topics, Sites, Notes, and annotations as
well as Entries. Initial implementation covers Entries/Articles only.

Broader support needs a UI/UX rethink. An Entry result opens the reader; a Feed,
Topic, or Site is itself a browsing target. Mixing those in a tag-filtered
article list introduces a second navigation level and makes selection behavior
less predictable. Annotation tagging also needs an explicit selected-annotation
context and assignment controls.

Explore separate browsing surfaces or grouping by target type, and how users
assign labels without confusing collection labels with labels on member
articles. Tagging a collection does not inherently tag its contents.

Site has no independent content file. Its assignments need an authoritative home
that preserves readable names and normalized-hostname identity, including when
the Site has no current articles. Account for external file edits, library
isolation, and rename/delete transitions across any additional assignment stores.

The Entry-only model should permit future coverage without claiming that a
single mixed-entity browser or Inspector interaction is already designed.
