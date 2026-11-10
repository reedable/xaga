# Automated testing

Test the behavior the code is responsible for. First identify what the test must
establish, then choose assertions that establish exactly that. Do not add
constraints unrelated to the behavior under test.

- Assert a specific value when that value is part of the contract. When the
  requirement is existence, change, equality, or ordering, assert that property
  or relationship directly. A test can capture a value before an action and
  compare it with the value afterward without hardcoding either value.
- Exercise the code through the interface appropriate to the behavior. Assert
  observable results rather than incidental implementation details. For example,
  an internal data attribute does not establish that accessible output exists.
- Keep assertions strong enough to catch the relevant regression. Ask whether
  the test would fail if the behavior it claims to cover were broken or absent.
  Passing after an assertion rewrite does not establish equivalent coverage.
- Keep setup and assertions focused on the scenario. Avoid unrelated exact
  values, markup, or snapshots that make harmless changes fail the test.

## Do not hardcode UI copy

Automated behavior tests must not select controls or assert results using
hardcoded product labels, messages, or sentence fragments. UI copy varies by
locale and changes independently of code behavior. Copy edits and translations
must not break these tests. UI copy review and testing are separate work.

Select controls through roles, stable form names, existing associations, or
scoped stable test identifiers where necessary. Assert behavior, state,
relationships, and data presentation. For accessible output, check the property
required by the scenario: for example, that a description exists or changes
after an action, without prescribing its wording. Do not substitute internal
attributes for the accessible output being tested.

Fixture data such as article titles, user-created names, and supplied error
payloads is distinct from product copy. Assert those values when their
presentation or exclusion is the behavior under test.

See [UI and UX testing](ui_testing.md) for workflow coverage and validation
requirements.
