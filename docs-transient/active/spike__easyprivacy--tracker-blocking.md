# Evaluate EasyPrivacy for article tracker blocking

## Summary

Determine how well EasyPrivacy rules block known tracking requests during remote
article loading.

## Requirements

- Use EasyPrivacy as the tracker list and its CC BY-SA option for a separately
  licensed rules file distributed alongside Xaga.
- Give the user a control to turn EasyPrivacy tracker blocking off. Existing
  loader protections apply in both states.
- Preserve legitimate article resources that match no tracking rule, subject to
  existing loader controls. Exclude cosmetic filtering and scriptlet injection.
- Preserve the loader's existing network and isolation policies, extraction,
  and saved content. Report which request paths and rule types the proposed
  approach can cover.

## Remaining work

- Pin and record the upstream EasyPrivacy revision and exact input artifact.
  Confirm any file-specific licensing terms
  and identify the attribution, license, and change notices to ship with the
  converted rules. Check the terms of any converter code or third-party material
  distributed with the rules. If evidence conflicts with the agreed licensing
  approach, report the evidence and its impact for the user's decision.
- Evaluate conversion to WebKit content rules and application before the first
  navigation. Use the existing loader image policy as the starting point for
  tracing rule installation and interaction with other rules. Measure conversion
  coverage, including unsupported rules and exceptions whose loss could block
  legitimate requests.
- Use controlled native fixtures to check that a known tracker receives no
  request with EasyPrivacy enabled and receives a request with it disabled
  when existing loader controls allow it. Verify that a legitimate resource on
  the same host remains available with EasyPrivacy enabled.
  Check that article capture still succeeds. Trace redirects, image recovery,
  and favicon acquisition to identify any paths outside the WebKit rules.
- Compare bundling a reviewed, pinned rules file with runtime updates, including
  freshness, reproducibility, publisher terms, and request privacy. Record
  observed coverage and breakage, remaining gaps, and the decisions needed on
  delivery, failure handling, and exceptions before scoping implementation.

## References

- [EasyPrivacy scope](https://github.com/easylist/easylist#easyprivacy)
- [EasyList repository licenses](https://easylist.to/pages/licence.html)
- [Xaga license](../../LICENSE)
- [WebKit content rules](https://developer.apple.com/documentation/webkit/wkcontentrulelist)
- [Loader image policy](../../src-tauri/src/loader_image_policy.rs)
- [Network architecture](../../docs/architecture/security/fetching_remote_content.md)
- [Resource persistence verification](../../docs/development/resource_verification.md)
