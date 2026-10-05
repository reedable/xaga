# Feed checksum scheme

The feed checksum changed from `sha256("title\nsubtitle")` to
`row::checksum` = `sha256("title\n\ndescription")`. The different bytes cause
feeds with the earlier checksum to re-embed on upgrade.

Decide whether to retain the new scheme or restore the earlier one. Restoring
it would cause feeds with the new checksum to re-embed again.
