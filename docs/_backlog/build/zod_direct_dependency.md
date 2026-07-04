# Declare zod as a direct dependency (pending)

`zod` is imported (`src/platform/TauriPlatform.ts`) but missing from `package.json`
dependencies — it resolves transitively today. Surfaced by knip as "unlisted". Declare it
directly so a transitive-dependency change cannot break the import.
