# Distinguish browsing selection from assignment

## Summary

Determine how controls should distinguish browsing selection from saved assignment
and consistently represent Topic and tag assignment. Sidebar Tags uses checkboxes
for browsing, Inspector Topics uses checkboxes for assignment, and Inspector Tags
uses chips for assignment.

## Requirements

- Review these workflows across the sidebar, responsive navigation, and Inspector.
  Identify which interactions change displayed results and which change saved data.
- Compare two or three control approaches with mockups showing selected/assigned,
  disabled/read-only, loading, and error states. Evaluate labels, visual distinction,
  keyboard operation, focus, and screen-reader feedback.
- Preserve existing browsing and assignment semantics. Explain each approach's
  trade-offs and recommend one with supporting evidence. The user chooses the
  design; do not change production controls during this spike.
- Complete when the user has approved an interaction design and its implementation
  is captured in bounded stories with affected workflows and verification criteria.

## Remaining work

- Inspect the current controls, shared components, and workflow tests; map the
  operations and states that the proposed design must support.
- Produce the alternatives and mockups for review, including responsive behavior.
- After the user's decision, record the approved design in durable documentation,
  create implementation stories, and delete this spike when no work remains.

## References

- [Tag browsing and list behavior](../../docs/architecture/list_view.md)
- [Tag assignment](../../docs/architecture/tags.md)
- [Inspector](../../docs/architecture/article_inspector.md)
- [Sidebar Tags](../../src/components/sidebar/TagList.tsx)
- [Responsive navigation](../../src/components/list/EntityList.tsx)
- [Topic assignment](../../src/components/content/TopicMembership.tsx)
- [Tag assignment picker](../../src/components/content/EntryTags.tsx)
- [UI testing](../../docs/conventions/ui_testing.md)
