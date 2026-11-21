# AI assistant provisioning — Git signing

> **NOTE:**
> Start in the AI assistant's shell (`su - alpha`, substituting the actual account name for `alpha`).

**Step 1:** Open [GitHub email settings](https://github.com/settings/emails) on the account receiving the key. Add the AI assistant's email and follow the verification link sent to that address.

**Step 2:** Export the public key.

In the command below, substitute:

- `AI_ASSISTANT_PRIMARY_FINGERPRINT`: the AI assistant's primary key fingerprint.

```
gpg --armor --export 'AI_ASSISTANT_PRIMARY_FINGERPRINT'
```

**Step 3:** Copy the output. Open [GitHub GPG keys](https://github.com/settings/keys), choose **New GPG key**, paste the public key, and click **Add GPG key**.

**Step 4:** Configure the Git identity.

In the command below, substitute:

- `Alpha`: the AI assistant's name.

```
git config --global user.name 'Alpha'
```

In the command below, substitute:

- `foo+alpha@example.com`: the AI assistant's verified GitHub email.

```
git config --global user.email 'foo+alpha@example.com'
```

**Step 5:** Configure Git signing.

```
gpg --list-secret-keys --with-subkey-fingerprint
git config --global gpg.format openpgp
```

In the command below, substitute:

- `SIGNING_SUBKEY_FINGERPRINT`: the AI assistant's signing subkey fingerprint (under `ssb`); keep the trailing `!`.

```
git config --global user.signingkey 'SIGNING_SUBKEY_FINGERPRINT!'
```

```
git config --global commit.gpgsign true
```

**Step 6:** Create and verify a signed test commit. Enter the passphrase when prompted.

**WARNING:** Any staged changes will be included in this commit.

```
cd ~/Development/xaga
git checkout -b develop
git commit --allow-empty -m 'Verify commit signing'
git verify-commit HEAD
```

**Step 7:** Push the commit to your development branch. Open the commit on GitHub and check that it shows **Verified**.
