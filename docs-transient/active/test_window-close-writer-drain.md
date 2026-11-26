# Verify native shutdown with a retained writer handle

## Summary

Verify native Close, Quit, restart, and read-only Close in a logged-in macOS
desktop session. Use the extended native fixture at
`a96a5c6c7b6ec0487a03691bc116be4944ad637b` on the
`defect__window-close-writer-drain` branch.

## Requirements

- Run the fixture as the user logged into the macOS desktop. Use the repository's
  Rust and desktop build prerequisites. The fixture creates its own temporary
  libraries and blank webviews.
- Require Close and Quit to save the pending Interactive edit. Require the
  restarted successor to acquire the library lock in write mode and read the
  saved edit. Require read-only Close to leave its fixture directory empty.

## Remaining work

- Fetch the branch with `git fetch origin defect__window-close-writer-drain`
  and check out the fixture revision in a clean worktree. Run
  `cargo test -p xaga --test window_shutdown --locked`.
  Record the tested commit, macOS version, exit status, and the `PASS` lines
  for `close`, `quit`, `restart`, and `readonly`. If the test fails, record
  the failing mode and complete error output.

## References

- [Native shutdown verification](../../docs/development/verification.md#native-shutdown-verification)
- [Native fixture](../../src-tauri/tests/window_shutdown.rs)
- [Library lock contract](../../docs/architecture/single_writer_library_lock.md)
