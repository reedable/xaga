# UX defect: toolbar favicon appears only on second viewing

The user verified that the previously failing site's favicon now appears, but
reported that it was absent on the first load and appeared on the second viewing.
Status: open, deferred. This item records the display symptom and its suspected
toolbar cause as one UX issue.

Code inspection found a possible cause in `Toolbar.tsx`: article metadata is
read when the selected entry ID changes, but refreshing the same article does
not change that ID. The toolbar can therefore retain metadata from before the
new favicon was saved. This code predates the download integration; its role in
the reported symptom has not yet been confirmed by reproduction.

Acceptance: after first ingestion or refresh saves a favicon, the toolbar shows
the persisted local icon without requiring navigation away and back. Verify both
first ingestion and refresh of an existing article, preserving local-only image
rendering.

