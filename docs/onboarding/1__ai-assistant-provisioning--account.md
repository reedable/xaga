# AI assistant provisioning

>**NOTE:**
Start in the human operator's account.

**Step 1:** Create a new standard macOS account for the new AI assistant, e.g. `alpha`. **WARNING:** Do _not_ grant administrator access.

**Step 2:** Open Terminal. Log into the AI assistant account,

In the command below, substitute:

- `alpha`: the AI assistant account name.

```
su - alpha
```

**Step 3:** Create the development folder.

```
mkdir Development
cd Development
```

**Step 4:** Set up `~/.zshrc` in the AI assistant account home, for example,

```
cat > ~/.zshrc <<'EOF'
set -o vi
PS1='%n$ '
export PATH="/opt/homebrew/bin:${PATH}"
export GPG_TTY=$(tty)
EOF
```

Apply the settings:

```
source ~/.zshrc
```

**Step 5:** Generate an SSH key.

```
ssh-keygen -t ed25519
```

**Step 6:** Print the public key and copy it from Terminal.

```
cat ~/.ssh/id_ed25519.pub
```

**Step 7:** Add the public key at [GitHub SSH keys](https://github.com/settings/keys). Choose **New SSH key**, title it `alpha`, select **Authentication Key**, paste the key, and click **Add SSH key**.

**Step 8:** Clone the repository.

```
git clone git@github.com:reedable/xaga.git
```

**WARNING:** Before accepting GitHub's host key, verify the fingerprint against [GitHub's published fingerprints](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/githubs-ssh-key-fingerprints). The Ed25519 fingerprint is `SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU`. Enter `yes` only if it matches.
