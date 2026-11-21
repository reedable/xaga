# AI assistant provisioning — GPG

> **WARNING:**
Ed25519 is not quantum-resistant. A sufficiently capable quantum computer could recover the private keys from their public keys and forge signatures. Keeping the primary key offline does not prevent this. Plan to migrate to post-quantum signing.

>**NOTE:**
Start in the human operator's account. GPG must be installed and the human's certification key available.

**Step 1:** Generate the AI assistant's certification-only primary key. Protect it with a passphrase.

In the command below, substitute:

- `Alpha`: the AI assistant's name.
- `foo+alpha@example.com`: the AI assistant's email.

```
gpg --quick-generate-key 'Alpha <foo+alpha@example.com>' ed25519 cert 2y
```

**Step 2:** Generate the signing-only subkey.

In the command below, substitute:

- `AI_ASSISTANT_PRIMARY_FINGERPRINT`: the primary key fingerprint printed in Step 1.

```
gpg --quick-add-key 'AI_ASSISTANT_PRIMARY_FINGERPRINT' ed25519 sign 1y
```

**Step 3:** Read the signing subkey fingerprint (under `ssb`).

In the command below, substitute:

- `AI_ASSISTANT_PRIMARY_FINGERPRINT`: the primary key fingerprint printed in Step 1.

```
gpg --list-secret-keys --with-subkey-fingerprint 'AI_ASSISTANT_PRIMARY_FINGERPRINT'
```

**Step 4:** Certify the AI assistant's identity with the human's key.

In the command below, substitute:

- `HUMAN_KEY_FINGERPRINT`: the human's certification key fingerprint.
- `AI_ASSISTANT_PRIMARY_FINGERPRINT`: the AI assistant's primary key fingerprint.

```
gpg --local-user 'HUMAN_KEY_FINGERPRINT' --quick-sign-key 'AI_ASSISTANT_PRIMARY_FINGERPRINT'
```
