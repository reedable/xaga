# Sidebar visual congestion

## Summary

The sidebar combines shortcuts, pinned collections, feed and Topic sections, unread counts, expansion controls, and creation actions. Define which information and actions need persistent prominence before changing its layout.

## Requirements

- Preserve keyboard navigation, focus visibility, labels, and unread-state accessibility.
- Keep frequently used destinations and actions discoverable.
- Support narrow window sizes and large feed and Topic sets.
- Do not reduce density by hiding state or actions without an explicit interaction model.
- Evaluate visual hierarchy with realistic names and counts.

## Remaining work

- Capture representative crowded states and identify the specific hierarchy and density failures.
- Determine which sections, counts, and actions are primary, contextual, or collapsible.
- Compare compact spacing, grouping, progressive disclosure, and alternate navigation structures.
- Present prototypes and tradeoffs for decision.
- Implement and verify the selected design with keyboard and screen-reader interaction.

## References

- `src/components/sidebar/Sidebar.tsx`
- `src/components/sidebar/ShortcutList.tsx`
- `src/components/sidebar/FeedList.tsx`
- `src/components/sidebar/TopicList.tsx`
