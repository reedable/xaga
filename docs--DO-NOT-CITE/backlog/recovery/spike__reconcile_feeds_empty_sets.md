# reconcile_feeds passes empty unread/bookmarked sets

`reconcile_feeds` (`crates/xaga-core/src/recovery/`) calls `list_feeds(dir, &empty, &empty)` — passing empty unread/bookmarked sets and discarding the counts it computes — just to obtain domain `Feed`s. Recorded during the decoupling audit. Options: a feed-listing path that doesn't compute counts, or accepting the wasted computation; neither was decided.
