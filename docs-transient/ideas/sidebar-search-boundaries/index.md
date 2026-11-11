# Sidebar and Search boundary captures

Open [the boundary gallery](index.html). All screenshots are 800px high.

The prototype keeps the existing Sidebar width rules, then evaluates the settled
article reader: a visible Sidebar collapses below 320px. After that collapse,
Search becomes a right overlay drawer if inline Search still leaves the reader
below 320px. Exactly 320px passes. The 288px drawer uses the default MUI backdrop;
the article retains its underlying layout width while the drawer covers its right side.

Captures include one pixel below, exactly at, and one pixel above:

- The existing 980px Sidebar threshold without Search, for all four column combinations.
- The existing 1300px Sidebar threshold with Search, for all four column combinations.
- The Search drawer boundaries: 929px for EntityList 320px and 1089px for EntityList 480px.
- Additional Sidebar-collapse reader boundaries reachable under the existing rules:
  1001px (Sidebar 360 / EntityList 320), 1041px (240 / 480), and 1161px (360 / 480)
  without Search; 1329px (240 / 480) and 1449px (360 / 480) with Search.

Search at 840px, 900px, and 920px is included for both EntityList widths as a reference.
Both configured Sidebar widths are captured at the drawer boundaries, even though
Sidebar is already hidden there. No short-height captures are included.

This is a static screenshot prototype using the existing synthetic library and
an empty Search query. It evaluates each layout in stages after transitions settle;
it does not implement live resizing, restoration, result selection, or production
interaction behavior. No tests are changed. Temporary application source changes
are restored after capture and retained only in [prototype.patch](prototype.patch).

To recreate, apply the patch, start capture mode on port 1421, run
`node docs-transient/ideas/sidebar-search-boundaries/capture.mjs`, then reverse the patch.
[Measurements](measurements.json) record layout before the new conditions, after
Sidebar evaluation, and after Search evaluation.

Previous sets remain unchanged:

- [Original fixed-width rules](../sidebar-widths/index.html)
- [Additional Sidebar reader minimum](../sidebar-widths-reader-minimum/index.html)

## Quick comparison: Search presentation boundary

| EntityList | Below: drawer                        | Exactly: inline, reader 320px        | Above: inline, reader 321px          |
| ---------- | ------------------------------------ | ------------------------------------ | ------------------------------------ |
| 320px      | [928px](w928-s240-l320-search.png)   | [929px](w929-s240-l320-search.png)   | [930px](w930-s240-l320-search.png)   |
| 480px      | [1088px](w1088-s240-l480-search.png) | [1089px](w1089-s240-l480-search.png) | [1090px](w1090-s240-l480-search.png) |
