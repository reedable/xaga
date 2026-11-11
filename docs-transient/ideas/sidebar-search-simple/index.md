# Search drawer with the simplified 608px rule

Open [the new screenshot gallery](index.html).

Sidebar visibility follows only the existing app-width thresholds: below 980px
without Search and below 1300px with Search. Resized column widths do not add
another reason to collapse Sidebar. After that decision, Search is inline when
article-plus-Search space is at least 608px, and an overlay drawer below 608px.
The Search panel is 288px, so the inline reader minimum is 320px.

All captures are 800px high, using the same synthetic library, article, light
theme, and empty Search query as previous sets. The drawer uses the standard MUI
backdrop and covers part of the reader without reducing its underlying width.
The existing horizontal clipping behavior remains visible in some inline states;
it has not been fixed or hidden for these captures.

This set includes the preceding boundary set's widths plus Search-open captures
from the original five window widths and four column-width combinations.
No short-height captures are included. Previous sets are unchanged:

- [Original Sidebar rules](../sidebar-widths/index.html)
- [Sidebar reader minimum](../sidebar-widths-reader-minimum/index.html)
- [Sidebar-first collapse and conditional Search drawer](../sidebar-search-boundaries/index.html)

This is a static screenshot prototype. It evaluates settled layout once per
capture; live resizing, restoration, and Search result interactions are not
implemented. Application source is restored after capture. The temporary change
is retained only in [prototype.patch](prototype.patch). No tests are changed.

To reproduce, apply the patch, start capture mode on port 1421, run
`node docs-transient/ideas/sidebar-search-simple/capture.mjs`, then reverse the patch.
[Measurements](measurements.json) contain before and final geometry.

## Selected comparisons

| Window / Sidebar / EntityList | Previous two-stage rule                                             | Simplified rule                   |
| ----------------------------- | ------------------------------------------------------------------- | --------------------------------- |
| w1300-s240-l480-search        | [Previous](../sidebar-search-boundaries/w1300-s240-l480-search.png) | [New](w1300-s240-l480-search.png) |
| w1328-s240-l480-search        | [Previous](../sidebar-search-boundaries/w1328-s240-l480-search.png) | [New](w1328-s240-l480-search.png) |
| w1448-s360-l480-search        | [Previous](../sidebar-search-boundaries/w1448-s360-l480-search.png) | [New](w1448-s360-l480-search.png) |
| w1089-s360-l480-search        | [Previous](../sidebar-search-boundaries/w1089-s360-l480-search.png) | [New](w1089-s360-l480-search.png) |

[1352px with Sidebar 360px, EntityList 480px, and Search as a drawer](w1352-s360-l480-search.png).
