# Give users control over remote image loading

## Summary

The article loader is the only path for loading live articles. Selecting
**Reader** indicates that the user wants to use it, and the Reader choice
sticks across articles. The privacy controls govern what the loader can do.

## Requirements

- Keep **Reader** in the menu and preserve its sticky behavior. Use that
  choice in place of a separate **Enable article loader** setting.
- Provide a separate opt-in for remote image loading. With it off, block
  browser image loads and favicon requests through the loader using WebKit's
  supported controls. When publisher JavaScript is enabled, its general
  requests may still retrieve image bytes; this is an accepted limit. Do not
  invent additional image-blocking behavior beyond what WebKit supports.
  Explain this limit.
- Keep **Save article images for offline viewing** separate. Saving and
  recovering images can make additional requests that create a pattern
  specific to Xaga. Explain that image hosts can use live image requests
  to track visits and that some users want to read the text without images.
- When remote image loading is off, Xaga must not acquire or save new remote images,
  even if offline saving is enabled. Image capture and recovery require both
  remote image loading and offline saving to be enabled. Preserve images
  already saved in the library.
- Continue showing locally saved images. Explain in the Settings copy
  that reopening a saved article uses the images already in the library
  without requesting them again.
- Present the privacy controls together in order of increasing permission
  and explain what each allows.

## Remaining work

- Trace how the sticky Reader preference interacts with existing acquisition
  paths before specifying changes to individual actions.
- Apply WebKit image-request blocking before the loader's first navigation.
  WebKit content rule lists support blocking HTTP(S) resources classified as
  `image`; general script fetches are outside that classification and retain
  the accepted limit above. Verify native image-element and CSS image requests.
- Gate Xaga's explicit acquisition separately: body capture uses
  `__xagaFetchImages` in `article_loader.js`, hidden-loader favicon acquisition
  navigates to the favicon from `receive_html` in `commands/loader_capture.rs`,
  and saved-image recovery opens `LoaderPurpose::Image` windows from
  `recover_entry_images`. An image-only content rule is insufficient for these
  fetch and top-level navigation paths. Verify they make no image requests when
  remote image loading is off, while text capture and saved-image display work.
- Update Settings copy, the security documentation, the User Guide, and
  training to explain the shipped controls and their limits.

## References

- [Current privacy settings](../../src/components/settings/PrivacySettings.tsx)
- [Publisher JavaScript control](../../docs/architecture/security/handling_remote_content.md#publisher-javascript)
- [WebKit content rule lists](https://developer.apple.com/documentation/webkit/wkcontentrulelist)
- [WebKit resource classifications](https://webkit.org/blog/3476/content-blockers-first-look/)
