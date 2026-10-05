# UX defect: toolbar favicon appears only on second viewing

The favicon was absent on the first load and appeared on the second viewing.

Code inspection found a possible cause in `Toolbar.tsx`: article metadata is
read when the selected entry ID changes, but refreshing the same article does
not change that ID. The toolbar can therefore retain metadata from before the
new favicon was saved. Reproduce the symptom to confirm this cause.

Acceptance: after first ingestion or refresh saves a favicon, the toolbar shows
the persisted local icon without requiring navigation away and back. Verify both
first ingestion and refresh of an existing article, preserving local-only image
rendering.

