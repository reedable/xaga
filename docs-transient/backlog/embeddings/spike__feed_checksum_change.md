# Feed checksum scheme changed (pending — unapproved change awaiting decision)

The feed checksum changed from `sha256("title\nsubtitle")` to `row::checksum` =
`sha256("title\n\ndescription")` during the decoupling work — different bytes, so every feed
re-embeds once on upgrade. The change was never flagged at the time. The one-time re-embed has
likely already occurred on this machine; the open decision is whether the new scheme is
ratified as-is (reverting would trigger another one-time re-embed).
