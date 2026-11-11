# Sidebar captures with a 320px reader threshold

Open [the new gallery](index.html) or [the previous gallery](../sidebar-widths/index.html).
The new gallery links each screenshot to its matching previous screenshot.

The same 60 viewport, column-width, and Search combinations use the same synthetic
library and article. Existing 980px/1300px Sidebar thresholds and height-dependent
rendering remain. After layout settles, a visible Sidebar additionally collapses
when the article reader is less than 320px wide. The collection dropdown then appears.

This is a static screenshot prototype. Each scenario starts fresh and evaluates
the visible reader once. Live resizing and the rule for restoring a collapsed
Sidebar are not implemented. An already-hidden Sidebar cannot provide more space.
The inspector retains its existing responsive behavior. Search uses an empty query.

The temporary UI change is retained in [prototype.patch](prototype.patch); it is
not left applied to application source. To reproduce, apply that patch, start
capture mode on port 1421, run the following command, then reverse the patch:

```sh
node docs-transient/ideas/sidebar-widths-reader-minimum/capture.mjs
```

[Measurements](measurements.json) include reader width and Sidebar visibility before
and after the additional rule. The previous capture directory remains unchanged.

## States changed by the additional rule

| Viewport   | Sidebar / EntityList | State  | Reader before → after | Screenshots                                                                                  |
| ---------- | -------------------- | ------ | --------------------- | -------------------------------------------------------------------------------------------- |
| 1352 × 800 | 360 / 480            | search | 223 → 583             | [Before](../sidebar-widths/w1352-s360-l480-search.png) · [After](w1352-s360-l480-search.png) |
| 1147 × 800 | 360 / 480            | normal | 306 → 666             | [Before](../sidebar-widths/w1147-s360-l480-normal.png) · [After](w1147-s360-l480-normal.png) |
| 1147 × 400 | 360 / 480            | short  | 306 → 666             | [Before](../sidebar-widths/w1147-s360-l480-short.png) · [After](w1147-s360-l480-short.png)   |
| 1024 × 800 | 240 / 480            | normal | 303 → 543             | [Before](../sidebar-widths/w1024-s240-l480-normal.png) · [After](w1024-s240-l480-normal.png) |
| 1024 × 400 | 240 / 480            | short  | 303 → 543             | [Before](../sidebar-widths/w1024-s240-l480-short.png) · [After](w1024-s240-l480-short.png)   |
| 1024 × 800 | 360 / 480            | normal | 183 → 543             | [Before](../sidebar-widths/w1024-s360-l480-normal.png) · [After](w1024-s360-l480-normal.png) |
| 1024 × 400 | 360 / 480            | short  | 183 → 543             | [Before](../sidebar-widths/w1024-s360-l480-short.png) · [After](w1024-s360-l480-short.png)   |
