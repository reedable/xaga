# AI assistant provisioning — Codex CLI

The human operator must have installed Codex CLI through Homebrew (`brew install --cask codex`), making it available to all accounts.

In [ChatGPT Security Settings](https://chatgpt.com/settings/security), enable **Enable device code sign-in for Codex, Excel, PowerPoint, and Word** for the ChatGPT account whose subscription will be used.

Start in the AI assistant's shell (`su - alpha`, substituting the actual account name for `alpha`).

**Step 1:** Start Codex from the repository.

WARNING: `codex` and `codex exec` use the directory you launch them from as their working directory. Login is saved for the AI assistant account regardless of directory; you do not need to sign in every session.

```
cd ~/Development/xaga
codex
```

**Step 2:** Select **Sign in with Device Code**.

**Step 3:** Open the displayed URL in the human operator's browser, sign in with the ChatGPT account, and enter the code. Complete sign-in.
