# Library pin store

`<library>/metadata/pins.yaml` owns pin membership. Its path is derived by
`library::pins_path`; the Rust `Pins`, `PinNode`, and `PinTarget` types define the parsed format, and the platform exposes that model to the shared UI provider.

```yaml
pins:
  - type: entry
    id: abc123
  - type: feed
    id: feed456
  - type: topic
    id: topic789
```

Target identity is type plus ID, independent of group placement. Sequence order is preserved; pinning an existing
target is a no-op, and unpinning removes only that typed reference. Titles and
counts are resolved from current targets rather than duplicated in the file.
Missing stores mean empty membership. Malformed stores, duplicate targets,
anchors, unknown fields, and unsafe IDs are refused before mutation. Reads are
bounded to 12 MiB. IDs cannot contain path separators, parent traversal, or NUL.

Mutations serialize the parsed model; YAML formatting and comments are not retained.

All pin writes use one Interactive library-writer lane, under the existing
library write lock. The complete file is replaced through a sibling temporary
file and atomic rename. A pin edit does not modify the target entity. Library
read-only checks apply to generic and entity-specific pin commands; target
navigation remains available in read-only libraries.

After saving a membership change, the command emits a typed `pin:changed` event.
Organization commands emit `pins:organized` with the committed `PinEdit`. Both
notifications cause the UI to reread the current central tree. Request generations prevent earlier
responses from overwriting later snapshots; delayed notifications also read
current membership. Feed/Topic controls and the article toolbar show pin membership supplied by the
shared provider. Article shortcuts resolve source-file metadata without
waiting for cache recovery. The `is_pinned` cache column is a disposable mirror;
recovery derives it from the central store, never legacy tags or categories.

Unresolved references remain in storage and appear in both sidebar and responsive
navigation as compact “Shortcut unavailable” rows. Opening a row shows its type
and identity in a details dialog, with Retry and Remove shortcut actions. The
initial explanation says “We can’t find this topic/feed/article,” describing
resolution failure without claiming a cause: bulk lists can omit unreadable targets. Retry reports user-safe read errors where available, refreshes
the corresponding shared list, and remains available in read-only libraries.
Removal is disabled in read-only libraries and affects only the pin. Initial
Topic/Feed loading uses skeleton rows instead of unavailable-target errors.
Restoring a target resolves the same reference on the next target refresh.
Removing a pin preserves the current list and reading scope.
Article shortcuts suppress counts and contribute zero to unread sorting.
**All items** is library-wide navigation below the sidebar toolbar, independent
of shortcut expansion and collection filtering. Alphabetical/unread sorting is presentation state and never rewrites
the sequence.

Site pins use `type: site` and a normalized `hostname`, not an entity ID:

```yaml
pins:
  - type: site
    hostname: example.com
```

The persistence boundary validates bare DNS hostnames and applies the Site
identity rules for case, international domains, and terminal dots. Pin payloads
already carry Site identities: the article URL’s one-leading-`www.` alias rule
is not reapplied to them. For example, an article on `www.www.example.com`
derives to `www.example.com`, which remains unchanged across pin reads/writes.
URLs, paths, ports, credentials, and IP literals are refused. Hostnames never
become filesystem path components. Equivalent Site references cannot occur twice.
Site pin changes use the same writer lane and `pin:changed` event as other pins;
there is no Site entity row or cache pin column to update.

Site Settings shows a read-only hostname and an **Add to Shortcuts** checkbox.
Membership comes from the shared pin store each time the dialog opens. Read-only
libraries can navigate pinned Sites and inspect Settings, but cannot change pins.
Pinned Sites use current derived total/unread counts and existing Site navigation.
Once the Site list has loaded, a pinned hostname absent from that computed list is
still a valid empty Site with zero counts, not an unavailable target. Its pin
survives cache recovery and out-of-band changes. Initial Site loading uses the
same skeleton rows as other collection shortcuts. Alphabetical/unread sorting
includes Sites; **All items** is outside the sorted shortcuts.

Site targets extend the library format to **0.12**, implemented by **0.12.0-dev**.
Existing 0.11 libraries require the explicit
[CLI upgrade](../library_upgrade.md#site-pins-012). Flat Entry/Feed/Topic pins
retain their representation and order. Older applications cannot edit 0.12
libraries; their version mismatch selects read-only mode. No Site membership is
persisted or converted, and Site references never create cache entity rows.

## Sidebar sorting

The sidebar toolbar has one **Sort and filter collections** button immediately
left of Refresh. It opens a popup beside the sidebar containing a **Sort collections**
dropdown with **Sort by unread** and **Sort by name**, and a **Filter collections**
text input. The filter applies immediately through the existing shared view state;
closing the popup retains it. A dot on the toolbar button and its accessible name
indicate an active nonblank filter. The popup includes **Clear collection filter**,
supports keyboard dismissal, and returns focus to the trigger when closed. The
inline sidebar filter input is removed; All items stays available above the lists.

Matching checked sort commands in the macOS View menu are grouped with
**Show Sidebar**. The dropdown and native menu save through the same account-settings
command; only a successful write updates the choice and menu checkmarks. Failures
use the shared error presentation.

The optional `sidebar_sort` account preference stores `unread` or `alphabetical`.
Absence defaults to unread, without an eager write. Old IndexedDB
`sidebar.sort.<section>` preferences are ignored. One `SidebarSortProvider`
subscribes to the typed `settings:sidebar-sort` event before reading the initial
value, so a late startup read cannot overwrite a newer change.

The choice applies to Shortcuts, Topics, Feeds, and Sites, including each nested
shortcut container and its native-menu and responsive projections. Unread sorting
uses descending unread counts with title comparison for ties; name sorting uses
title comparison. Groups retain recursive count semantics, and article shortcuts
contribute zero. Sorting changes presentation only, preserving `pins.yaml` order.
Tags stay sorted by name, and All items is unaffected by collection filtering.
Sorting remains available in read-only libraries because it changes an account
preference rather than library data.

## Sidebar height

Let `A` be the sidebar height available below All items and `H` the combined
rendered height of the five section headers. When `A < 2 * H`, a **Collection list**
dropdown replaces the expandable headers. At equality and above, the expandable
sections remain. Header measurements retain their normal layout width and font
metrics in compact mode, so changing layouts does not change the threshold.
Resizing and text scaling re-evaluate it.

The selector offers Shortcuts, Topics, Feeds, Sites, and Tags. Its selected list
fills the remaining space and scrolls independently. Compact mode omits section
counts. The selected section's creation action remains beside the selector;
read-only libraries disable creation while retaining browsing. The toolbar
sorting/filtering popup and All items remain available. Nonselected content and measurement headers are inert
and excluded from accessibility navigation.

The selector uses the shared expanded-list view state. When all sections are
collapsed, compact mode displays Shortcuts without overwriting that state;
returning to the taller layout restores the collapsed sections. Choosing a list
updates the shared selection. Changing layouts or list sections preserves the
current article and browsing scope. Keyboard focus moves between the selector
and the section header when the focused control is replaced by resizing.

## Nested groups (0.15)

Library format **0.15**, implemented by **0.15.0-dev**, adds group nodes:

```yaml
pins:
  - type: group
    id: group123
    title: Research
    children:
      - type: site
        hostname: example.com
      - type: group
        id: group456
        title: Background
        children: []
```

Group IDs are unique across the tree and survive renames and moves. Titles are
trimmed and must be nonempty; duplicate titles are allowed. Typed targets remain
unique across the entire tree. Strict YAML parsing bounds structural depth to 64;
the typed model also refuses more than 24 nested groups, including edits received
through IPC. The existing 12 MiB bound and unknown-field/anchor rejection remain.

`PinEdit` operations create, rename, remove, or move nodes through the same
Interactive pin writer lane. A move removes and appends the node in one atomic
store replacement; destinations inside the moved subtree are refused. New pins
append at root; new groups and moved nodes append in their destination. Removing
a group replaces it with its children at the same stored position, preserving
child order and all target pins. Toolbar unpin finds its target at any depth.
Recovery, entity projections, and article resolution flatten target references
from this same tree; groups have no entity or cache rows.

In Shortcuts, the header plus button opens a group-title dialog at root. Each
group has a three-dot actions menu with **Add group**, **Rename group**,
**Move to…**, and **Remove group**. The menu button appears on hover and keyboard
focus and stays visible while its menu is open. During those states, the row’s
unread count is visually hidden with its layout space and screen-reader text
preserved. Move to… offers the root and eligible groups as a keyboard alternative
to drag and drop. Moving by drop into a collapsed group expands it. Individual
shortcuts use the same three-dot menu with **Move to…** and **Remove shortcut**,
including the same count visibility and keyboard behavior. Removing a shortcut
only unpins its target. Organization preserves
the open list and reading pane; read-only libraries can navigate and expand but
cannot change the tree.

Groups expand independently, defaulting to expanded. Collapsed descendants stay
mounted inside MUI Collapse; transitions run only when reduced motion is not
requested. Expansion is local view
state keyed by library path and group ID. Filtering reveals ancestor paths to
matching descendants; a matching group title reveals its full subtree. Filtered
expansion does not rewrite saved expansion. Each container sorts by the selected
Shortcuts mode. Group unread counts sum descendants recursively without
article deduplication across pinned collections; article pins contribute zero.
Groups display that unread count without a separate item count. Sorting never
rewrites stored sequence order. The selectable navigation header shows the
hierarchy when the sidebar is hidden; organization controls remain in the sidebar.

The explicit [CLI upgrade](../library_upgrade.md#nested-pin-groups-015) validates
0.14 pin stores and advances library metadata without rewriting flat pins or
scanning article files. Interrupted upgrades can be retried. Older libraries
still run their required conversions before completion. Version mismatch keeps
older applications from writing upgraded libraries.

### Sticky group headings

In the sidebar's Shortcuts scrolling area, group headings at levels one through
five remain visible while their descendants scroll beneath them. Root groups are
level one. Nested headings stack below ancestors and leave as their group ends;
sibling headings replace one another. Deeper headings scroll normally. This is a
display cap and does not change the supported 24-level stored hierarchy or edits.
The behavior applies in both expandable-section and compact-selector layouts.

The existing heading controls are positioned directly, without duplicate
navigation or accessible headings. A resize observer measures their actual
heights and updates ancestor-height offsets in rem after font changes,
expansion, and filtering. It reads all heading heights before writing offsets,
reuses those measurements for ancestors, and writes only changed offsets.
ResizeObserver ignores width-only changes because group labels cannot wrap;
height changes schedule one measurement per animation frame. Opaque theme backgrounds and ancestor stacking
order prevent scrolled descendants from showing through headings. Actions and
unread counts retain their existing visibility and accessibility contracts.

Group labels stay on one line and truncate with an ellipsis. The full name
remains in the accessible button name and is available on hover. Resizing the
Sidebar does not wrap group labels.

The five-level cap is fixed. There is no adaptive heading selection or reserved
space for shortcuts. A sufficiently short viewport or enlarged heading text can
therefore fill the list viewport and obscure shortcuts. This limitation is
available for hands-on review before choosing further overflow behavior. The
[browser workflow](../development/browser_tests.md#sticky-group-headings) includes
a reproduction and screenshot.

## Native menu

The native macOS **Shortcuts** menu sits immediately right of **History**.
It always starts with **All items**, followed by a separator when pins exist.
The main UI supplies the same resolved targets used by the sidebar, independent
of sidebar visibility or its text filter. The native menu uses the shared recursively sorted hierarchy, matching the
sidebar’s selected sort mode. Groups become nested submenus; empty groups contain
a disabled **Empty** item.
Pin, group, title, count, and sort changes refresh the native menu;
sorting never rewrites `pins.yaml`. Article shortcuts navigate into article
scope, and Site shortcuts use Site navigation, including empty pinned Sites.
Unavailable targets open the same Retry/Remove shortcut dialog as the sidebar;
initial collection loading produces disabled loading items instead.

Only the main window may replace these native menu items. Native menu actions
focus that window and deliver `menu:shortcut` through the shared typed app-event handler.
Menu item keys identify the version of the menu that created them, preventing
a delayed action from selecting a different target after replacement. Navigation and Retry remain available in read-only
libraries; Remove shortcut still requires write permission. The native menu does not read
library files or store pin membership separately.

The recovery walk constructs a membership set once, so its per-entity membership
lookup does not scan the ordered sequence. Article-shortcut resolution reads one
membership snapshot and each referenced article file, without repeatedly parsing
the store. Normal pin mutations and each newly transferred legacy pin still read
or serialize the complete pin sequence; the converter streams one source document
at a time. Those costs are governed by the number of pins and the 12 MiB store
bound, rather than a full-corpus pin discovery scan during normal navigation.
