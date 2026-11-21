# AI assistant provisioning — GPG backup and transfer

> **NOTE:**
Start in the human operator's account.

**Step 1:** Back up the private keys to secure offline storage.

**WARNING:** Keep these backups and the primary private key out of the AI assistant's account.

In the command below, substitute:

- `/Volumes/KEY_BACKUP`: the backup storage path.
- `AI_ASSISTANT_PRIMARY_FINGERPRINT`: the AI assistant's primary key fingerprint.
- `AI_ASSISTANT_IDENTITY`: the AI assistant's identity label, e.g. `alpha@example.com`.

```
gpg --armor --export-secret-keys 'AI_ASSISTANT_PRIMARY_FINGERPRINT' > '/Volumes/KEY_BACKUP/secret_AI_ASSISTANT_IDENTITY-AI_ASSISTANT_PRIMARY_FINGERPRINT.asc'
```

**Step 2:** Back up the revocation certificate to the same offline storage.

In the command below, substitute:

- `/Volumes/KEY_BACKUP`: the backup storage path.
- `AI_ASSISTANT_PRIMARY_FINGERPRINT`: the AI assistant's primary key fingerprint.
- `AI_ASSISTANT_IDENTITY`: the AI assistant's identity label, e.g. `alpha@example.com`.

```
cp "${GNUPGHOME:-$HOME/.gnupg}/openpgp-revocs.d/AI_ASSISTANT_PRIMARY_FINGERPRINT.rev" '/Volumes/KEY_BACKUP/revocation_AI_ASSISTANT_IDENTITY-AI_ASSISTANT_PRIMARY_FINGERPRINT.rev'
```

**Step 3:** Export only the signing secret subkey to secure storage readable by the AI assistant account.

In the command below, substitute:

- `SIGNING_SUBKEY_FINGERPRINT`: the AI assistant's signing subkey fingerprint; keep the trailing `!` in the key selector.
- `/Volumes/KEY_TRANSFER`: the secure storage path.
- `AI_ASSISTANT_IDENTITY`: the AI assistant's identity label, e.g. `alpha@example.com`.

```
gpg --armor --export-secret-subkeys 'SIGNING_SUBKEY_FINGERPRINT!' > '/Volumes/KEY_TRANSFER/AI_ASSISTANT_IDENTITY.asc'
```

**Step 4:** Enter the AI assistant account.

In the command below, substitute:

- `alpha`: the AI assistant's account name.

```
su - alpha
```

**Step 5:** Configure GPG to prompt for passphrases directly. Run once.

```
mkdir -p -m 700 "${GNUPGHOME:-$HOME/.gnupg}"
printf '%s\n' 'pinentry-mode loopback' >> "${GNUPGHOME:-$HOME/.gnupg}/gpg.conf"
```

**Step 6:** Import the signing subkey file.

**WARNING:** Import the transfer file from Step 3, not the full private-key backup from Step 1.

In the command below, substitute:

- `/Volumes/KEY_TRANSFER`: the secure storage path used in Step 3.
- `AI_ASSISTANT_IDENTITY`: the AI assistant's identity label.

```
gpg --import '/Volumes/KEY_TRANSFER/AI_ASSISTANT_IDENTITY.asc'
```

**Step 7:** Check the imported keys.

```
gpg --list-secret-keys --with-subkey-fingerprint
```

**WARNING:** The primary key must show `sec#` (private key unavailable); the signing subkey must show `ssb`.

**Step 8:** Return to the human operator's shell and delete the transfer file.

```
exit
```

In the command below, substitute:

- `/Volumes/KEY_TRANSFER`: the secure storage path used in Step 3.
- `AI_ASSISTANT_IDENTITY`: the AI assistant's identity label.

```
rm '/Volumes/KEY_TRANSFER/AI_ASSISTANT_IDENTITY.asc'
```
