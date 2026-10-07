# Article loading diagnostics

An article operation has a diagnostic ID shared by the frontend and native load,
extraction, and persistence calls. The error notification's **Details** action
opens the recent record; **Copy diagnostics** copies its JSON. Technical detail
is copied as plain text, never rendered as HTML or substituted into the normal
user-facing error message.

## Collection and ownership

`src-tauri/src/article_diagnostics.rs` owns the native history and log sink.
Its command adapters live in `src-tauri/src/commands/articles.rs`.
`src/utils/ArticleLoadAttempt.ts` carries an attempt through frontend work;
shared diagnostic payloads live in `src/types.ts`, and the platform port exposes
`createArticleDiagnostic`, `getArticleDiagnostic`, and `addArticleDiagnosticEvent`.
`ArticleDiagnosticsDialog` owns presentation and copying under `components/dialogs`.

`useContentLoader`, `useArticles`, and `ArticleReader` identify lookup, fetch,
extraction, save/readback, and reader-preparation boundaries. `entryWorker.ts`
reports its parse, Readability, image restoration, sanitization, and Markdown
phases. Fresh retries receive new IDs.

Each record contains an ID, start time, operation mode, and an ordered sequence
of events: elapsed milliseconds at receipt, source, step, outcome, stable reason
code, and bounded technical detail. Outcomes distinguish starts, successes,
warnings, failures, skipped work, and cancellation. Frontend events are queued
in order; elapsed times measure receipt by the native collector, not precise
CPU time inside synchronous extraction.

The load command records native download validation and PDF persistence. The
loader records window construction/policy failures, navigation milestones,
page-reported capture progress, delivery, and image/favicon problems. Image
problems are aggregated by reason with counts. A favicon failure is a warning;
already captured article HTML is preserved if favicon navigation stalls or
returns an HTML error document. Cancellation ends the matching pending capture
and destroys its loader. Download callbacks do not expose HTTP response headers.

Native storage errors retain their typed code and available cause chain.
Frontmatter parser fragments and arbitrary JavaScript exception messages are
excluded; a failed Readability extraction has its own reason code. Unknown
frontend errors retain their phase and a generic code rather than copying
potential article content out of an exception.

The history holds the latest 32 attempts, with at most 128 events per attempt.
On overflow, the first event and newest events are retained and `droppedEvents`
counts omissions. Records are in memory and disappear on restart. Structured
`article_diagnostic` lines also go through the existing native release logger,
whose files are capped at 5 MB. On macOS the log directory is
`~/Library/Logs/xaga`; other platforms use the application data root's `logs`
directory. Event logging is capped per attempt after overflow, except terminal
failure/cancellation records.

## Network attribution

Each native article load gets a scoped loopback proxy listener built from the
same `ProxyConfig` as the shared proxy. Policy handles are shared, so current
HTTP/CONNECT settings and the existing resolve/validate/pin rules still apply.
The loader and its native downloads use this listener. No diagnostic ID or extra header is
sent to the publisher. Proxy events are attributed by listener ownership, never
by comparing hostnames or timestamps from concurrent loads.

The proxy reports invalid requests, local hostname refusal, blocked addresses,
DNS failure, disallowed CONNECT ports, disabled plaintext HTTP, connection
failure, and I/O errors. Its observer contains no orchestration or storage;
the command layer attaches the diagnostic collector. Dropping the scoped proxy
closes its listener and active tunnels. A visible loader retains
it until its window is destroyed.

A proxy event belongs to the attempt, but can concern a subresource. It is not
proof that the article's main document failed. HTTPS tunnels remain opaque: the
proxy cannot report their HTTP status, TLS diagnosis, or redirect chain.

## Trust and privacy

The normal trusted-window command policy protects history read/write commands.
The separate `add_loader_diagnostic_event` command accepts only fixed milestone/reason
vocabulary and a bounded count. Identity comes from the invoking loader window;
remote content cannot choose another attempt ID. At most 64 such events are
accepted for a pending loader. HTML/favicon capture callbacks also bind their
request ID to that window before consuming pending state.

Page-reported milestones are explicitly marked `source: loader`: the remote
page can forge them, and they are not security evidence. Native/proxy observations
remain separate. General events exclude HTML, PDF bytes, image data, cookies, headers,
and raw URLs. Native download evidence is the explicit exception described below. URL-bearing tokens in native exception text are removed; strings
are length-limited and control characters stripped. Frontend diagnostics accept
codes rather than raw exception text, and stored frontmatter parse errors are
classified without including their source fragments.

## Observation limits

The installed Tauri/Wry integration exposes navigation and page-load milestones,
but no application callback carrying WebKit provisional-navigation errors.
Diagnostics do not replace WebKit's navigation delegate. A timeout records the
wait that expired and the last observed milestone; the underlying browser cause
is explicitly unknown unless independent evidence was collected. Navigation
callbacks are not treated as a verified redirect chain.

A successful `render` step means reader HTML/blob preparation succeeded. It does
not establish successful painting or decoding inside WebKit's built-in PDF
viewer. Subsequent visible-browser HTML extraction starts a separate attempt;
page-script milestones are collected only while the initial request is pending.
Native navigation observations remain associated with that browser's original
attempt. Frontend cancellation cancels observation of the command result; native
work still follows its existing timeout/cleanup lifecycle.

## Verification

Frontend tests cover extraction failure, code-only reporting, cancellation,
concurrent-attempt isolation, unavailable diagnostic transport, and image-timeout
reporting. Native tests cover retention/redaction and preserving captured HTML
on favicon timeout. Proxy tests exercise independent observers with real policy
refusals and scoped-listener/tunnel cleanup. Socket tests need permission to bind
local test listeners.

Native browser behavior must be smoke-tested in a bundled build, following
[fetching remote content](security/fetching_remote_content.md): test an invalid
URL, blocked local target, unavailable server, stalled page, favicon failure,
PDF download, and a visible loader. Verify the notification's copied record and
the release log agree. A development-server run does not establish bundled
WebKit behavior.

## Download evidence

Article loaders override Wry's default acceptance with native pending-phase
checks. The first download during the pending favicon phase is accepted into private
temporary storage and must pass image-byte validation before entering existing
persistence. Downloads outside that phase and additional downloads are rejected.
The callback URL may be redirected and does not establish the initiating frame.
Diagnostic collection makes no extra
network requests. `article:download-accepted` records transport acceptance, not
validation or persistence success; `download_validation` and `favicon` record
processing. Download failure or validation rejection completes the pending
capture with the original article HTML and a favicon warning.

`download_requested` events include a structured `download` object: native loader
label, original article URL, current webview URL (if available), download URL,
and proposed local destination. `download_finished` records any runtime completion
callback, including its success flag and optional path. A completion callback is
not guaranteed after rejection; on macOS its path can be absent. The current
webview URL is context, not proof of the initiating frame or redirect chain.
Tauri does not expose response headers, MIME type, HTTP status, the initiator,
or a cancellation error through this callback. These must not be inferred.

These records appear in Copy diagnostics and the bounded native release log
(`~/Library/Logs/xaga` on macOS), including when article loading succeeds and no
error dialog appears. Search the log for `download_requested` or
`article:download-rejected`. Each line carries the existing attempt ID.
The event count and log size limits above still apply. Individual evidence fields
are limited to 4096 characters and control characters removed. HTTP(S) URL
credentials and fragments are removed; other schemes are omitted entirely.
Paths and query parameters are retained to identify the resource, so reports
and logs can contain sensitive tokens or filenames. Review before sharing.
The diagnostics dialog displays a reminder when download evidence is present.

## Favicon notifications

Observed acquisition failures in the native favicon phase show a warning:
“The site icon couldn’t be loaded.” The warning retains the attempt's Details
link. Favicon timeouts do not use the generic page-timeout notification.

When a saved icon reference is remote or its local bytes cannot be read, the
warning is: “The site icon couldn’t be loaded. Reloading the article will also
retry loading its icon.” No reference alone is not treated as a load failure.
Remote references remain excluded from toolbar image sources. Notifications
are deduplicated by article URL for the lifetime of the error-sink provider
(the app session); the first warning is retained for that URL. These warnings
make no claim about article availability or the validity of its content.

Download diagnostics leave `currentUrl` null. A navigation can become a download
before WebKit commits a document URL, and Wry 0.57's `url()` getter unwraps that
absent value. The callback records the native download URL and original source
URL without querying the webview's current URL.
