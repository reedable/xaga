# Sticky shortcut group headings: prototype evidence

The user selected sticky group headings as the starting direction. This local
Chromium investigation establishes feasibility, not a production implementation.

## Initial implementation direction

Start with sticky headings at group levels one through five, counting root
groups as level one; deeper headings scroll normally. This follows the user's
instruction to choose the simplest initial implementation. The current recursive
tree can determine eligibility by depth without tracking a changing set of
nearest ancestors during scrolling. Stored group depth remains unchanged.

The user has deferred adaptive overflow decisions until a working implementation
can be seen and used. The fixed cap does not establish that wrapped headings fit
every viewport. Preserve and report that limitation for hands-on review rather
than requiring a speculative overflow policy before implementation.

## Reproduction and artifacts

- [Capture scenario patch](capture-scenario.patch) adds an opt-in `sticky-spike`
  scenario to the existing capture platform, with 80 distinct Topic shortcuts,
  wrapped titles, sibling groups, and a 24-level branch.
- [Browser probe](probe.mjs) opens the real UI and applies experimental sticky
  styles to existing group rows. It measures each ancestor heading and uses their
  summed heights as nested offsets. It recalculates after filter and viewport
  changes explicitly; it does not implement production resize observation.
- [Raw browser measurements](results.json) contain computed browser coordinates.
- [Shallow hierarchy screenshot](nested.png) shows the parent and wrapped child
  heading remaining visible above scrolled shortcuts.
- [Deep hierarchy screenshot](deep.png) shows headings filling the available
  Shortcuts viewport while shortcuts are underneath them.

From the repository root, with the capture source matching the patch:

```sh
git apply --check docs-transient/ideas/shortcuts-sticky-headings/capture-scenario.patch
git apply docs-transient/ideas/shortcuts-sticky-headings/capture-scenario.patch
npm run ui:capture -- --port 1421
```

In another terminal:

```sh
node docs-transient/ideas/shortcuts-sticky-headings/probe.mjs
```

The probe writes new artifacts under `test-results/sticky-spike/`. Stop the capture
server and remove only the experimental fixture changes afterward:

```sh
git apply -R docs-transient/ideas/shortcuts-sticky-headings/capture-scenario.patch
```

## Observed behavior

The expanded section owns the scroll container
([section styles](../../../src/components/sidebar/ExpandableList.module.css#L53)).
Each group row and its descendant Collapse share a containing tree node
([tree](../../../src/components/sidebar/ShortcutTree.tsx#L254)). In the installed
MUI version, Collapse uses hidden overflow during transitions and visible overflow
after entering. The animated probe observed both states. Changing the Collapse
structure was unnecessary for the settled expanded-state prototype.

At the default root font size, the parent heading measured `1.8125rem` high and
the wrapped child heading `4.4375rem`. In the shallow scrolling probe, the parent
top matched the scroll viewport top and the child top matched the parent bottom.
The child released before the next sibling heading stuck below the parent. The
parent released when scrolling into the next root group.

The existing actions menu opened from a stuck heading. Collapse and re-expansion
worked, and filtering to one fixture shortcut retained its ancestor path. Two
programmatic focus checks reached unobscured shortcut controls. These
checks do not establish complete Tab traversal or focus behavior at every group
boundary. Collapse changed scroll position through browser scroll anchoring; the
prototype does not prescribe a different scroll-retention contract.

Unrestricted stacking failed in the short-window probe. The Shortcuts viewport
was approximately `8.75rem` high. Five single-line headings require `9.0625rem`,
already exceeding that space. At the first shortcut in the 24-level branch, the
viewport showed ancestor headings instead of the shortcut. Existing indentation
also causes extreme wrapping at that depth; the stack-height problem exists even
without that additional wrapping.

## Implementation implications

- CSS sticky positioning can retain the actual group controls without duplicate
  headings or another representation of the tree.
- A fixed offset per depth cannot preserve the current variable-height headings.
  One implementation option is to measure headings and propagate ancestor-height
  sums, updating when titles, font metrics, available width, or filtering change.
  A CSS-only alternative preserving variable-height titles was not established
  by this spike.
- Headings need opaque theme-aware backgrounds and ancestor/sibling stacking
  order. Production lengths must follow the rem convention.
- An overflow policy is required before unrestricted nesting can ship. Options
  below remain proposals for the user, not approved requirements.

| Overflow approach                                                                       | Benefit                                                   | Cost or limitation                                                                                 |
| --------------------------------------------------------------------------------------- | --------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Fall back to ordinary scrolling when the ancestor stack cannot leave room for shortcuts | Keeps the initial implementation close to the prototype   | Loses persistent hierarchy context in overflowing branches; the space threshold needs definition   |
| Keep the nearest containing heading sticky when the full stack cannot fit               | Retains local group context with less vertical space      | Hides outer ancestry; one very long heading still needs an overflow rule                           |
| Compress older ancestors into a compact path while retaining the nearest heading        | Retains broader hierarchy context and space for shortcuts | Adds path layout, truncation, and interaction decisions beyond the initial sticky-heading behavior |

The nearest-heading fallback is a candidate for preserving useful local context,
but none of these approaches has been prototyped or selected. A fixed depth cap
alone does not handle wrapped titles or short viewports.

## Verification limits

After restoring the application source, `npm run test:browser` passed all 50
tests. This verifies the restored application; the sticky prototype observations
come from the separate browser probe above.

The probe exercises Chromium against the capture platform. It does not verify
native WKWebView rendering, native IPC, saved group edits, drag-and-drop moves,
screen-reader announcements, unread-count overlap, theme variants, or font-size
scaling. The existing capture platform refuses group edits
([mock platform](../../../src/platform/capture/MockPlatform.ts#L928)); a production
implementation needs fixture support for organization workflows rather than a
second mock backend. The animated probe confirms overflow-state transitions but
does not establish visual continuity throughout animation.

Implementation verification should cover sticky geometry, group boundaries,
wrapped titles and resizing, the selected overflow behavior, keyboard traversal,
actions and counts, filtering, collapse with both motion preferences, and drag
and drop. Native WebKit needs a rendering check in addition to Chromium coverage.
