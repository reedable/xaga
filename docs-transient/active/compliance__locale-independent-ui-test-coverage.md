# Test UI behavior independently of copy

## Summary

Refine the scope and approach for removing UI-copy dependencies from tests without
losing behavioral or accessibility coverage. The concern spans multiple test
surfaces and is separate from implementing in-app library upgrades.

## Requirements

- Tests must verify user-visible behavior and accessibility without depending on
  English UI copy. Internal attributes alone do not establish accessible output.
- Prefer roles, form names, existing associations, and semantic state. Use scoped,
  stable test identifiers only where controls otherwise need distinguishing;
  avoid blanket identifiers and fragile positional selectors.
- Distinguish product copy from fixture data such as article titles, user-created
  names, and supplied error payloads. Preserve tests of data presentation and
  private-detail exclusion.
- Establish bounded implementation work and acceptance checks before starting
  another repository-wide rewrite. Product internationalization itself is outside
  this story's scope.

## Remaining work

- Agree which test surfaces and workflows to include first, how to find remaining
  copy dependencies, and whether implementation should be split into smaller
  stories. Candidates include component, browser, native-menu, and training tests;
  their inclusion and order remain open.
- Define how tests will check accessible output and relationships independently
  of language, including meaningful state changes. Bookmark announcement tests
  currently assert `data-bookmark-color` and can pass if the screen-reader
  announcement disappears. Use assignment, recoloring, and removal as a concrete
  case when refining the approach.
- Assess replaced assertions for lost behavioral coverage rather than treating
  passing tests as proof of equivalence. Decide the bounded review coverage and
  which regressions need replacement assertions.
- Review identifier and helper usage within the agreed scope. Known examples
  include `inspector-topics` on an unrelated `SearchResultList` control and the
  shared browser `control` helper's support for multiple selector forms. Decide
  which identifiers are necessary and which should be removed or corrected.
- Define evidence that copy changes or a different UI language do not break the
  selected workflows, while broken behavior and accessible output still fail
  their tests. Agree the validation commands and remaining native/manual checks.

## References

- [UI testing convention](../../docs/conventions/ui_testing.md)
- [Browser test coverage and setup](../../docs/development/browser_tests.md)
- [Bookmark announcement component tests](../../src/components/list/EntityList.test.tsx)
- [Bookmark browser tests](../../e2e/bookmark-colors.spec.ts)
- [Browser control helper](../../e2e/fixtures.ts)
- [Search controls](../../src/components/search/SearchResultList.tsx)
- [Native menu tests](../../src-tauri/tests/sidebar_sort_menu.rs)
- [Training tests](../../docs/training/training.test.mjs)
