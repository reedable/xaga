# Invalid items silently dropped in list operations (pending)

In `src/platform/TauriPlatform.ts`, roughly ten list operations drop items that fail schema
validation without reporting anything. A malformed row disappears from the UI with no signal to
the user or the logs. Recorded fix direction: unified error reporting for validation drops.
