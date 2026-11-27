# Evaluate EasyPrivacy for article tracker blocking

## Summary

Determine whether Xaga can ship EasyPrivacy rules and use them to block known
tracking requests during remote article loading.

## Requirements

- Evaluate redistribution under Xaga's current proprietary terms before selecting
  a converter or delivery model. Assess the exact list revision, the converted
  rules, and any third-party converter code or resources separately.
- Use EasyPrivacy as the candidate list. Preserve ads and legitimate article
  resources that match no tracking rule, subject to existing loader controls.
  Exclude general ad blocking, cosmetic filtering, and scriptlet injection.
- Preserve the loader's existing network and isolation policies, extraction,
  and saved content. Report which request paths and rule types the proposed
  approach can cover.

## Remaining work

- Verify EasyPrivacy's CC BY-SA option for the exact upstream revision. Establish
  attribution, change notices, and share-alike terms for a separately shipped
  WebKit rules file. Check the terms of the converter, its dependencies, and any
  material it adds to the output. Record a viable packaging approach or the
  specific licensing issue that prevents one.
- Evaluate conversion to WebKit content rules and application before the first
  navigation. Use the existing loader image policy as the starting point for
  tracing rule installation and interaction with other rules. Measure conversion
  coverage, including unsupported rules and exceptions whose loss could block
  legitimate requests.
- Use controlled native fixtures to check that a known tracker receives no
  request, an ad that matches no tracking rule still loads, and a legitimate
  resource on a shared host remains available when its tracking URL is blocked.
  Check that article capture still succeeds. Trace redirects and native image
  and favicon recovery and identify any paths outside the WebKit rules.
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
