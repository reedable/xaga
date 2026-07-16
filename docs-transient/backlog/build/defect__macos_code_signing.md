# macOS code signing: stable identity to stop the WebCrypto keychain prompt (pending)

## Symptom

Visiting bbc.com (and a few other sites from time to time) in Xaga pops a macOS keychain dialog:

> Xaga wants to use your confidential information stored in "Xaga WebCrypto Master Key" in your keychain.

Nothing in the app's code or config asks for keychain access, and the dialog reappears across rebuilds even after clicking "Always Allow".

## What we found

The keychain item is created and used by WebKit itself, not by Xaga. The item in the login keychain (created 2026-06-17):

- account: `com.apple.WebKit.WebCrypto.master+app.xaga`
- comment: "Used to encrypt WebCrypto keys in persistent storage, such as IndexedDB"

When a website calls the Web Crypto API (`crypto.subtle`) and persists a `CryptoKey` into IndexedDB, WKWebView encrypts that key at rest using a per-app master key it keeps in the keychain. Sites that do this (bbc.com among them) trigger a keychain read on load. A grep of the repo for keychain/WebCrypto references hits only `Cargo.lock` — no app code is involved.

The prompt recurs because keychain ACLs are tied to the app's code signature, and both installed copies of Xaga.app (`~/Applications` and `target/release/bundle/macos`) are ad-hoc signed (`Signature=adhoc`, `TeamIdentifier=not set`, verified via `codesign -dv`). Every rebuild produces a different ad-hoc identity, so "Always Allow" grants access to a signature that no longer exists after the next build, and macOS prompts again.

State at time of writing: `security find-identity -v -p codesigning` reports 0 valid identities on the machine, and `src-tauri/tauri.conf.json` has no `signingIdentity` under `bundle.macOS`.

## Fix: sign builds with a stable identity

Two ways to obtain the identity; the Tauri wiring afterward is the same.

### Option A — Apple Developer ID certificate

Requires Apple Developer Program membership ($99/year).

1. Enroll at developer.apple.com.
2. Xcode → Settings → Accounts → Apple ID → Manage Certificates → "+" → **Developer ID Application** (or via the developer portal with a CSR generated in Keychain Access).
3. Verify: `security find-identity -v -p codesigning` lists `Developer ID Application: Name (TEAMID)`.

Pros:

- Valid for distributing to other Macs; Gatekeeper-trusted (with notarization).
- Stable for years.

Cons:

- Costs money; enrollment takes time.
- More than is needed if the app never leaves this machine.

### Option B — self-signed code-signing certificate

Free, local-only.

1. Keychain Access → Certificate Assistant → **Create a Certificate…**
2. Name e.g. `Xaga Dev`; Identity Type: Self-Signed Root; Certificate Type: **Code Signing**.
3. Verify: `security find-identity -v -p codesigning` lists `Xaga Dev`.

Pros:

- Free, immediate, fully solves the keychain-prompt problem on this machine.

Cons:

- Gatekeeper won't trust it on any other Mac — useless for distribution.
- Signing setup gets redone if the app is later distributed.

### Then, in both cases: point Tauri at the identity

Either in `src-tauri/tauri.conf.json`:

```json
"macOS": {
  "minimumSystemVersion": "14.0",
  "signingIdentity": "Xaga Dev"
}
```

(exact identity name from `security find-identity`), or via environment variable at build time, which keeps the identity out of the committed config:

```sh
APPLE_SIGNING_IDENTITY="Xaga Dev" npm run tauri build
```

## What to expect afterward

- The identity applies when `tauri build` produces the bundle. Replace the copy in `~/Applications` with the newly signed build.
- On first launch of the signed build, the keychain prompt appears **once more** — click **Always Allow**. From then on, every rebuild signed with the same identity matches the keychain ACL and the prompt stops.
- Bundled builds only: a binary run via `tauri dev` (`target/debug`) remains ad-hoc linker-signed and would still prompt. Webview behavior is exercised via bundled builds anyway (CSP is likewise only enforced in the bundle).

## Open question

Whether Xaga will ever be distributed to other machines — that is the fact that separates Option A from Option B.
