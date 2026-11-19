# Prevent and respond to compromised Xaga releases

## Summary

Define a strategy for malicious code entering Xaga through frontend or Rust
dependencies, first-party source changes, build tooling, CI, or release
infrastructure. Bundled frontend code already has the app origin's trust and
can attempt permitted native commands. Bundled Rust code runs with the app
process's OS permissions. The browser CSP cannot establish the integrity of
either component.

## Requirements

- Cover prevention, detection, and response from source and dependency changes
  through building, signing, and distribution.
- Make it possible to trace a published artifact to its source revision,
  resolved inputs, and build run, and to identify other releases that share
  a suspect input. Distinguish artifact integrity and provenance from evidence
  that the inputs were safe.
- Define who can pause distribution and publish a replacement, how users learn
  whether their version is affected, and how to recover when the normal
  release workflow or its credentials may be compromised.

## Remaining work

- Map the current npm and Cargo inputs, including direct and transitive
  dependencies, release tarballs, and code that runs during installation or
  build. Map GitHub Actions, release credentials, signing, and distribution.
  Record current controls and where an attacker could introduce a change.
- Choose preventive controls proportionate to those paths. Evaluate dependency
  review and automated alerts, verification of pinned inputs and CI actions,
  least-privilege build and release jobs, credential separation, and release
  provenance. Decide which checks should block a release, who can approve
  exceptions, and what evidence each release should retain.
- Draft an incident runbook for a suspect dependency and for a compromised
  build or release system. Cover triage, evidence preservation, affected
  versions and platforms, pausing or withdrawing distribution, credential
  revocation and rotation, and rebuilding from independently verified inputs.
  Define how to publish an advisory and how users with an installed copy can
  verify and move to a clean release. Include how to assess possible exposure
  of local library data and credentials.
- Walk through both scenarios using a past release as a stand-in. Verify that
  the team can trace its inputs, identify affected artifacts, and explain how
  it would publish a trusted replacement. Use the gaps to scope follow-up
  implementation and keep the agreed strategy and runbook in durable docs.

## References

- [Development, build, and release cycle](../../docs/development/release_cycle.md)
- [Shared release pipeline](../../.github/workflows/release-pipeline.yml)
- [Tauri command boundary](../../docs/architecture/security/command_surface.md)
- [Article loader script policy](../active/compliance__article-loader-script-policy.md)
