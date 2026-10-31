# Upgrading a library to 0.10

Xaga 0.10 stores Entry reading state in the `is_unread` article frontmatter field,
separately from tags. This is a library-format compatibility change: 0.9 code
cannot safely use a 0.10 library, and 0.10 code requires explicit conversion of a
0.9 library. The app does not run the upgrade or reject old libraries at startup.

1. Close Xaga and other processes using the library.
2. Back up the library before changing its format.
3. Run the 0.10 CLI against that library:

   ```sh
   xaga-cli --data-dir /path/to/library library upgrade
   ```

4. After the command reports completion, open the library with Xaga 0.10.

The command requires the exclusive library write lock. It preserves existing
boolean `is_unread` values; otherwise presence of the legacy `unread` tag means
unread, and absence means read. It removes the legacy marker. Unread articles
store `is_unread: true`; read articles omit the field. Explicit false is accepted
on reads and normalized to omission on writes. Manually supplied articles
without the field are read; newly fetched Entries explicitly start unread.

Conversion preserves article content, annotations, unrelated metadata and
timestamps using sequential atomic file replacement under the library lock. YAML
formatting and comments are not retained; unsupported `images` frontmatter is removed under the existing
writer rule. The stored bookmark marker and organizational tags retain their values.

The command reports conversion and cache errors and exits nonzero when work is
unfinished. Resolve the reported errors and run the same command again. Retries
preserve files already converted and retry cache reconciliation even when no
files need rewriting. No embedding-model download is required. Existing cache
rows are reconciled in batches; missing rows remain for normal recovery to rebuild.

Do not reopen a partially converted library with either app version. Finish the
upgrade, or restore the full backup to return to 0.9.
