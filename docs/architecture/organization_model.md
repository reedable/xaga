# Organization Model

This is the agreed target domain design. Refactors implementing it may still be
outstanding; the definitions below do not claim every current type or storage
path already conforms.

## Collections and identity

Collection is the common abstraction for Feed, Topic, and Site. Membership may
be supplied or derived. ManagedCollection describes Feed and Topic: each has its
own identity and managed lifecycle independent of its current members. Managed
does not mean every member is selected manually.

Site is a Collection derived from article hostnames, without an independent
source-of-truth record or persisted membership list. Its normalization rules are
specified in [Sites](sites.md). UI `Site.id` holds that normalized hostname,
explained by a code comment and the platform mapping, without a duplicate UI
hostname field. Backend hostname naming remains unchanged. Do not fabricate
creation timestamps or loaded member lists to make Site satisfy a common type.

## Pins and Shortcuts

Pinning gives a navigation target access through Shortcuts. Library-scoped YAML
stores typed references centrally, rather than placing pin ownership in target
files. References contain an entity type and stable ID; Site references store
the normalized hostname, mapped to UI Site identity. Titles and counts are
resolved rather than copied into storage.

The root `pins` sequence contains target nodes. Group nodes extend the tree with
`type: group`, a stable `id`, `title`, and `children`. Groups organize navigation;
they do not change target content or membership.

Retain unresolved references and display that the target cannot be found, with a
manual removal action. Restoring a target makes the reference usable again.
A Site with zero members is still a valid navigation target, with zero counts.
Unpinning preserves the open list and reader context.

## Reading state, bookmarks, and color

Unread is Entry reading state, separate from organizational Tags. Its lifecycle
and storage are described in [the library upgrade guide](../library_upgrade.md).
Feed, Topic, and Site unread counts summarize member Entries rather than express
reading state on those collections.

Bookmarking an article assigns a color from the annotation swatch palette.
Color is optional; removal of the color removes the bookmark. There is no separate
stored or derived bookmark boolean. Article frontmatter stores optional `color`;
LanceDB carries the color, using an empty string for an unmarked Entry. Existing
bookmarks and imported bookmarks without colors receive Blue. Palette identifiers
are `red`, `orange`, `yellow`, `green`, `blue`, and `violet`. Article marking
commands carry optional color through the writer and cache; `bookmark:changed`
announces assignment, removal, and recoloring even when membership stays marked.

Every color-marked article is a bookmarked item eligible for bookmark export,
including formats unable to carry color. Saved includes marked articles and
annotations. Article marking uses swatch interaction without comment input;
whole-article comments remain a separate idea. Annotation color retains its
existing passage and comment context.

## Tags

Tags are optional user-defined organizational labels, with zero or more per
Entry. Initial functionality covers Entries/Articles only. Other entities need
further assignment and browsing UX design; theoretical applicability does not
imply implemented support.

A library YAML registry has a root `tags` sequence of definitions containing
`id` and display `name`. IDs are UUID v4 strings, independent of names and stable
across renames. Article frontmatter contains readable names; the indexing
boundary maps them to IDs stored in LanceDB assignments. Definitions own identity
and naming; article files own assignments.

Compare names case-insensitively using Unicode-aware comparison after trimming
surrounding whitespace. Reject only names empty after trimming; punctuation-only
names such as `---` are valid. Creation spelling determines display casing.
Later casing variants reuse the definition; explicit case-only renames are
supported. Atomic lookup/creation enforces one ID per normalized name.

Writable recovery registers unknown labels before indexing their assignments.
First-discovered spelling supplies the initial display name; read-only access
must not write definitions. Existing definitions and active transitions take
precedence over discovery.

### Assignment and browsing

The Inspector's Article Tags view presents searchable selectable chips. Clicking
a chip immediately assigns/removes that tag; removal preserves its definition.
An unmatched name offers Create “name”, which creates and assigns. No Save step
is required. The shared Inspector handles Articles and Annotations, shown as a
wide panel or a narrow drawer as described in [Inspector](article_inspector.md). Initial
tag assignment remains Article-only.

A Tags sidebar multi-select list browses Entries carrying every selected tag.
No selection means all Entries under existing text and Unread/Saved/All filters.
Responsive access extends the existing selectable ListView header. The combined
unread count counts unread Entries in the intersection once, not the sum of tag
counts. State or assignment changes preserve the current list until explicit
navigation/filter changes refresh membership.

### Rename

Rename retains identity and changes readable names in bounded file batches,
without rewriting unchanged indexed assignments. Persist progress and resolve
both names to the same ID until completion. Recovery loads transitions before
tag discovery, queues old-name files for rewriting, and retires the old mapping
only after completion. Subsequent old-name discovery can create a new identity.

Sequence transitions of the same tag; unrelated renames may proceed concurrently.
Serialize each entire file read–modify–write in the Entry writer lane and replace
atomically. Atomic replacement alone does not prevent lost updates.

Rename is available through hover/focus actions menus in sidebar and responsive
tag navigation. A prefilled dialog submits with Enter/Rename and cancels with
Escape/Cancel. Collisions are inline errors until explicit merging is available.
Show Renaming… and disable another rename for the affected tag during rewriting.

### Merge

A rename collision may offer explicit merge confirmation; cancellation changes
nothing. Use the destination name, but retain the ID with more indexed Entry
assignments; ties retain the destination ID. Persist that choice for retries.
Only losing-ID assignments change in LanceDB. Files whose names differ from the
destination still require rewriting, regardless of which ID survives. Deduplicate
articles carrying both tags.

Reserve both names and resolve them to the surviving ID while bounded file and
index conversion runs. New assignments use the destination name. External
non-destination labels join pending rewriting. Wait for active transitions on
participants before merging, then block conflicting rename/delete/merge work on
them while allowing unrelated operations. Retire the losing definition and old
mapping only after both conversions complete. The non-destination name may then
be reused for a new identity.

### Delete

Delete is available in the same actions menu and confirms removal of the tag
and its assignments without deleting articles. Assigned tags are deletable;
cleanup is bounded, checkpointed, and resumable. Block new assignments and
rename/merge involving a deleting tag. External occurrences join pending cleanup
until completion, after which the name may create a new identity.

Remove a deleted tag from active selection without clearing the current list.
The next explicit navigation/filter change refreshes membership. Removing an
assignment in Inspector chips never deletes the tag definition.

## Mutation and release contracts

Use the library write right and writer serialization for authoritative changes;
carry typed committed intent to index/UI consumers. Preserve read-only behavior
and make library-wide operations safe to interrupt and retry at the declared
[design scale](design_scale.md). Storage-model changes require a minor-version
increment during 0.x and an explicit CLI upgrade where conversion is needed.
