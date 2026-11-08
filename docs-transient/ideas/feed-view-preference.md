# Feed view preference

Allow a Feed to declare an explicit Summary or Reader view preference. When
viewing an article from that Feed, its explicit preference takes precedence over
the global view preference. A Feed without an explicit preference uses the global
preference.

Examples:

- xkcd: Summary, so the feed-provided comic is shown.
- BBC: Reader, so the article is shown in Reader view.

Users should be able to remove a Feed's explicit preference to use the global
preference again.
