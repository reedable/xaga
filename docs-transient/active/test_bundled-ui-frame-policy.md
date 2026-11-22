# Verify the bundled UI frame policy on macOS

## Summary

Verify the frame policy in a bundled macOS debug build of commit
`c4c7778a317491e798f9d07c9ef99258d36af096` from
`origin/compliance__bundled-ui-frame-policy`. The smoke check requires a
logged-in desktop session with a main display.

## Requirements

- Observe a `frame-src` enforcement event for the attempted HTTPS frame in
  both the main and Settings windows.
- Confirm the interface loads, IPC returns the current theme in both windows,
  and the reading pane renders saved article and PDF content.

## Remaining work

- Run the bundled smoke check below. Record the macOS version, tested commit,
  inspector results from both windows, the article Find result, and whether
  the PDF page rendered. Include error details for any failure and identify
  any check that could not run.

## Bundled macOS smoke check

Check out the specified commit on a macOS 14 or later desktop. Follow the
[local build prerequisites](../../docs/development/release_cycle.md#development-and-local-builds),
run `npm ci`, then run `npm run tauri -- build --debug --bundles app`. Open the
`Xaga.app` at the path printed by the build. Use a disposable library with a saved
article containing a known phrase in its body and a saved PDF with visible content
on its first page. In the main window, right-click the app surface and choose
**Inspect Element**, then select **Console** in Web Inspector. Run the following
in the top-frame console. Open **Settings...** with Command-comma and repeat in
that window's inspector.

```js
(async () => {
  const url = new URL("https://xaga-frame-policy.invalid/probe");
  const frame = document.createElement("iframe");
  let timeout;
  let onViolation;
  try {
    const violation = new Promise((resolve, reject) => {
      onViolation = (event) => {
        if (
          event.effectiveDirective === "frame-src" &&
          event.disposition === "enforce" &&
          [url.origin, url.href].includes(event.blockedURI)
        ) {
          resolve(event);
        }
      };
      document.addEventListener("securitypolicyviolation", onViolation);
      timeout = setTimeout(
        () => reject(new Error("No frame-src violation")),
        5000
      );
    });
    frame.src = url.href;
    document.body.append(frame);
    const [frameCheck, ipcCheck] = await Promise.allSettled([
      violation.then((event) => ({
        directive: event.effectiveDirective,
        blockedURI: event.blockedURI,
      })),
      window.__TAURI_INTERNALS__.invoke("get_theme"),
    ]);
    return {
      frame:
        frameCheck.status === "fulfilled"
          ? frameCheck.value
          : String(frameCheck.reason),
      theme:
        ipcCheck.status === "fulfilled"
          ? ipcCheck.value
          : String(ipcCheck.reason),
    };
  } finally {
    clearTimeout(timeout);
    document.removeEventListener("securitypolicyviolation", onViolation);
    frame.remove();
  }
})();
```

Confirm that both windows report a `frame-src` violation and the current
theme. Open the saved article in Reader and use Find to locate a phrase in
its rendered body. Enable **Show PDFs in the reading pane** in Settings and
confirm the saved PDF page appears inside Xaga.

## References

- [Bundled app CSP](../../src-tauri/tauri.conf.json)
- [Reader isolation and bundled CSP](../../docs/architecture/security/handling_persisted_content.md)
- [Article loader investigation](compliance__article-loader-script-policy.md)
- [Build supply-chain strategy](../backlog/compliance__software-supply-chain.md)
- [Loopback proxy bypass remediation](../backlog/compliance__loopback-proxy-bypass.md)
