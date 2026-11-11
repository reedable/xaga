# Sidebar column width captures

Open [the screenshot gallery](index.html) in a browser to review the full-size captures.

These captures use the existing browser capture platform, its crowded Sidebar fixture,
and an open article. Topics is expanded where the Sidebar is available. Width overrides
apply only to the rendered page; application sources, tests, and responsive rules are unchanged.
Search is open with an empty query because the capture platform does not provide search results.
Browser viewports exclude native window chrome. All dimensions below are CSS pixels at the default root font size.

The matrix retains the existing Sidebar thresholds: below 980 without Search,
below 1300 with Search. Short captures retain the measured compact-height rule.
The article inspector also follows its existing responsive behavior.

## Window width 1512

| Sidebar | EntityList | Height 800                           | Search, height 800                   | Height 400                         |
| ------- | ---------- | ------------------------------------ | ------------------------------------ | ---------------------------------- |
| 240     | 320        | [normal](w1512-s240-l320-normal.png) | [search](w1512-s240-l320-search.png) | [short](w1512-s240-l320-short.png) |
| 360     | 320        | [normal](w1512-s360-l320-normal.png) | [search](w1512-s360-l320-search.png) | [short](w1512-s360-l320-short.png) |
| 240     | 480        | [normal](w1512-s240-l480-normal.png) | [search](w1512-s240-l480-search.png) | [short](w1512-s240-l480-short.png) |
| 360     | 480        | [normal](w1512-s360-l480-normal.png) | [search](w1512-s360-l480-search.png) | [short](w1512-s360-l480-short.png) |

## Window width 1352

| Sidebar | EntityList | Height 800                           | Search, height 800                   | Height 400                         |
| ------- | ---------- | ------------------------------------ | ------------------------------------ | ---------------------------------- |
| 240     | 320        | [normal](w1352-s240-l320-normal.png) | [search](w1352-s240-l320-search.png) | [short](w1352-s240-l320-short.png) |
| 360     | 320        | [normal](w1352-s360-l320-normal.png) | [search](w1352-s360-l320-search.png) | [short](w1352-s360-l320-short.png) |
| 240     | 480        | [normal](w1352-s240-l480-normal.png) | [search](w1352-s240-l480-search.png) | [short](w1352-s240-l480-short.png) |
| 360     | 480        | [normal](w1352-s360-l480-normal.png) | [search](w1352-s360-l480-search.png) | [short](w1352-s360-l480-short.png) |

## Window width 1147

| Sidebar | EntityList | Height 800                           | Search, height 800                   | Height 400                         |
| ------- | ---------- | ------------------------------------ | ------------------------------------ | ---------------------------------- |
| 240     | 320        | [normal](w1147-s240-l320-normal.png) | [search](w1147-s240-l320-search.png) | [short](w1147-s240-l320-short.png) |
| 360     | 320        | [normal](w1147-s360-l320-normal.png) | [search](w1147-s360-l320-search.png) | [short](w1147-s360-l320-short.png) |
| 240     | 480        | [normal](w1147-s240-l480-normal.png) | [search](w1147-s240-l480-search.png) | [short](w1147-s240-l480-short.png) |
| 360     | 480        | [normal](w1147-s360-l480-normal.png) | [search](w1147-s360-l480-search.png) | [short](w1147-s360-l480-short.png) |

## Window width 1024

| Sidebar | EntityList | Height 800                           | Search, height 800                   | Height 400                         |
| ------- | ---------- | ------------------------------------ | ------------------------------------ | ---------------------------------- |
| 240     | 320        | [normal](w1024-s240-l320-normal.png) | [search](w1024-s240-l320-search.png) | [short](w1024-s240-l320-short.png) |
| 360     | 320        | [normal](w1024-s360-l320-normal.png) | [search](w1024-s360-l320-search.png) | [short](w1024-s360-l320-short.png) |
| 240     | 480        | [normal](w1024-s240-l480-normal.png) | [search](w1024-s240-l480-search.png) | [short](w1024-s240-l480-short.png) |
| 360     | 480        | [normal](w1024-s360-l480-normal.png) | [search](w1024-s360-l480-search.png) | [short](w1024-s360-l480-short.png) |

## Window width 840

| Sidebar | EntityList | Height 800                          | Search, height 800                  | Height 400                        |
| ------- | ---------- | ----------------------------------- | ----------------------------------- | --------------------------------- |
| 240     | 320        | [normal](w840-s240-l320-normal.png) | [search](w840-s240-l320-search.png) | [short](w840-s240-l320-short.png) |
| 360     | 320        | [normal](w840-s360-l320-normal.png) | [search](w840-s360-l320-search.png) | [short](w840-s360-l320-short.png) |
| 240     | 480        | [normal](w840-s240-l480-normal.png) | [search](w840-s240-l480-search.png) | [short](w840-s240-l480-short.png) |
| 360     | 480        | [normal](w840-s360-l480-normal.png) | [search](w840-s360-l480-search.png) | [short](w840-s360-l480-short.png) |

Recreate with capture mode running on port 1421:

```sh
node docs-transient/ideas/sidebar-widths/capture.mjs
```
