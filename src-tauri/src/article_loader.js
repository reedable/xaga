// Only fixed vocabulary crosses this boundary. Never report page text or URLs.
function __xagaDiagnostic(event, count) {
    try {
        return window.__TAURI_INTERNALS__.invoke('add_loader_diagnostic_event', { event: event, count: count || 1 }).catch(function() {});
    } catch (_) { return Promise.resolve(); }
}
__xagaDiagnostic('script_started');
var __xagaImageCounts = {};
function __xagaImageProblem(event) {
    __xagaImageCounts[event] = Math.min(100000, (__xagaImageCounts[event] || 0) + 1);
}
function __xagaReportImageCounts() {
    Object.keys(__xagaImageCounts).forEach(function(event) {
        __xagaDiagnostic(event, __xagaImageCounts[event]);
    });
    __xagaImageCounts = {};
}


window.open = function() { return null; };
window.focus = function() {};

// ---------------------------------------------------------------------------
// Fetch remote images as data: URLs before the HTML is captured, so the stored
// article carries its images locally and rendering it later makes no network
// request. An image whose host sends no `Access-Control-Allow-Origin` cannot be
// read (fetch() rejects); those keep their original remote `src` and are
// neutralised downstream (extract_images -> data-xaga-src). Set
// __XAGA_FETCH_IMAGES__ to false to disable.
// ---------------------------------------------------------------------------
var __XAGA_FETCH_IMAGES__ = window.__LOAD_REMOTE_IMAGE__ === true;
var __XAGA_IMAGE_MAX_BYTES__ = 2 * 1024 * 1024;
var __XAGA_IMAGE_TOTAL_BYTES__ = 24 * 1024 * 1024;
var __XAGA_IMAGE_CONCURRENCY__ = 6;

// The Rust side aborts the whole load after __LOADER_TIMEOUT_MS__
// (DEFAULT_LOADER_TIMEOUT_SECS = 10s), and that budget must also cover the page
// load itself. Fetching therefore gets a fraction of what remains, never a fixed
// figure — overrunning it does not merely skip images, it fails the article.
var __XAGA_LOADER_BUDGET_MS__ =
    typeof window.__LOADER_TIMEOUT_MS__ === 'number' && window.__LOADER_TIMEOUT_MS__ > 0
        ? window.__LOADER_TIMEOUT_MS__
        : 10000;
var __XAGA_IMAGE_OVERALL_MS__ = Math.max(1500, Math.floor(__XAGA_LOADER_BUDGET_MS__ * 0.45));
var __XAGA_IMAGE_PER_IMAGE_MS__ = Math.max(750, Math.floor(__XAGA_IMAGE_OVERALL_MS__ / 3));

function __xagaWithTimeout(promise, ms, onTimeout) {
    var timer;
    return Promise.race([
        promise,
        new Promise(function(resolve) { timer = setTimeout(function() {
            __xagaImageProblem('image_timeout'); resolve(onTimeout);
        }, ms); })
    ]).finally(function() { clearTimeout(timer); });
}

// Fetches an image and returns a data: URL, or null if it cannot be read
// (opaque/CORS-blocked response, HTTP error, non-image, or over the per-image
// cap). `budget` carries the running total of fetched bytes to enforce the
// per-load cap. Never throws — a rejection (typically a cross-origin image
// without permissive CORS headers) resolves to null.
function __xagaFetchAsDataUrl(url, budget) {
    return fetch(url, { credentials: 'omit', cache: 'force-cache' })
        .then(function(res) {
            if (res.type === 'opaque' || !res.ok) { __xagaImageProblem('image_http'); return null; }
            var len = res.headers.get('content-length');
            if (len && Number(len) > __XAGA_IMAGE_MAX_BYTES__) { __xagaImageProblem('image_size'); return null; }
            return res.blob();
        })
        .then(function(blob) {
            if (!blob) return null;
            if (blob.size > __XAGA_IMAGE_MAX_BYTES__) { __xagaImageProblem('image_size'); return null; }
            if (blob.type && blob.type.indexOf('image/') !== 0) { __xagaImageProblem('image_type'); return null; }
            if (budget.bytes + blob.size > __XAGA_IMAGE_TOTAL_BYTES__) { __xagaImageProblem('image_size'); return null; }
            return new Promise(function(resolve) {
                var reader = new FileReader();
                reader.onload = function() { budget.bytes += blob.size; resolve(reader.result); };
                reader.onerror = function() { __xagaImageProblem('image_unreadable'); resolve(null); };
                reader.readAsDataURL(blob);
            });
        })
        .catch(function() { __xagaImageProblem('image_unreadable'); return null; });
}

function __xagaFetchImages() {
    __xagaDiagnostic('images_started');
    if (!__XAGA_FETCH_IMAGES__) return Promise.resolve();

    var budget = { bytes: 0 };
    var imgs = [];
    var all = document.images ? [].slice.call(document.images) : [];

    for (var i = 0; i < all.length; i++) {
        var img = all[i];
        var src = img.currentSrc || img.src || '';
        if (!src) continue;
        if (src.indexOf('data:') === 0) continue;
        if (!/^https?:/i.test(src)) continue;
        imgs.push(img);
    }

    var cursor = 0;
    function worker() {
        if (cursor >= imgs.length) return Promise.resolve();
        var img = imgs[cursor++];
        var src = img.currentSrc || img.src;
        return __xagaWithTimeout(
            __xagaFetchAsDataUrl(src, budget),
            __XAGA_IMAGE_PER_IMAGE_MS__,
            null
        ).then(function(dataUrl) {
            if (!dataUrl) return;
            __xagaImageProblem('image_captured');
            // Stash the original remote URL so processEntry can pair it with the
            // data: bytes (read pre-Readability, then stripped there so it does
            // not trip Readability's lazy-load src rewrite).
            img.setAttribute('data-xaga-url', src);
            img.setAttribute('src', dataUrl);
            // srcset would let the renderer pick a remote candidate anyway.
            img.removeAttribute('srcset');
            img.removeAttribute('sizes');
            img.removeAttribute('loading');
        }).catch(function() {}).then(worker);
    }

    var pool = [];
    for (var w = 0; w < Math.min(__XAGA_IMAGE_CONCURRENCY__, imgs.length); w++) pool.push(worker());

    return __xagaWithTimeout(Promise.all(pool), __XAGA_IMAGE_OVERALL_MS__, null)
        .catch(function() { __xagaImageProblem('image_unreadable'); })
        .then(__xagaReportImageCounts);
}

// ---------------------------------------------------------------------------
// Favicon. Only the URL is resolved here — the bytes cannot be read from this
// document, because a favicon is usually served from a CDN host that sends no
// Access-Control-Allow-Origin and every in-page readback (fetch, XHR, canvas)
// requires it. Rust navigates this same webview to the URL instead; the
// resulting image document is same-origin with the image, so the branch at the
// bottom of this file can read it off a canvas. See the loader-navigation
// section of docs/architecture/security/fetching_remote_content.md.
// ---------------------------------------------------------------------------
function __xagaFaviconUrl() {
    // Same preference order as extractFavicon in entryWorker.ts.
    var links = [].slice.call(document.querySelectorAll('link[rel~="icon"]'));
    var preferred = null;
    for (var i = 0; i < links.length && !preferred; i++) {
        if (links[i].getAttribute('sizes') === '32x32') preferred = links[i];
    }
    for (var j = 0; j < links.length && !preferred; j++) {
        if (links[j].getAttribute('sizes') === '16x16') preferred = links[j];
    }
    if (!preferred && links.length) preferred = links[0];

    if (preferred) {
        var href = preferred.getAttribute('href');
        if (href) {
            try {
                return new URL(href, document.baseURI).href;
            } catch (e) {
                // Unparseable href: fall through to the conventional path.
            }
        }
    }
    // The page declares none: try the conventional location, which is the guess
    // the toolbar used to make from the main window.
    try {
        return new URL('/favicon.ico', document.baseURI).href;
    } catch (e) {
        return null;
    }
}

/** True when this document is the favicon itself, not an article. */
function __xagaIsImageDocument() {
    return (document.contentType || '').indexOf('image/') === 0;
}

window.addEventListener('beforeunload', function() {
    try {
        window.__TAURI_INTERNALS__.invoke('loader_will_unload', {});
    } catch(e) {}
});

// Native code invokes this at the overall deadline, even if load has not
// fired or the DOM never settles. Do not wait for images or favicon work.
var __xagaTimedOut = false;
window.__xagaCaptureOnTimeout = function() {
    if (__xagaTimedOut) return;
    __xagaTimedOut = true;
    var mimeType = document.contentType || 'text/html';
    try {
        window.__TAURI_INTERNALS__.invoke('receive_html', {
            id: window.__LOADER_ID__,
            html: mimeType === 'application/pdf' ? '' : document.documentElement.outerHTML,
            mimeType: mimeType,
            favicon: null,
            faviconData: null
        }).catch(function() { __xagaDiagnostic('ipc_failed'); });
    } catch (e) { __xagaDiagnostic('capture_failed'); }
};

window.__captureAndClose = function() {
    var mimeType = document.contentType || 'text/html';
    if (mimeType === 'application/pdf') {
        window.__TAURI_INTERNALS__.invoke('capture_and_close_loader', {
            html: '',
            mimeType: mimeType
        });
        return;
    }
    __xagaFetchImages().then(function() {
        window.__TAURI_INTERNALS__.invoke('capture_and_close_loader', {
            html: document.documentElement.outerHTML,
            mimeType: mimeType,
            // URL only: this window is visible and the user may be browsing it,
            // so it is never navigated away to read the favicon's bytes.
            favicon: __xagaFaviconUrl(),
            faviconData: null
        });
    });
};

// ---------------------------------------------------------------------------
// Favicon capture mode. Rust navigates this window here after the article HTML
// is captured, so the document IS the favicon: same-origin with the image, and
// therefore readable off a canvas without the CORS grant an in-page fetch of a
// cross-origin favicon would have needed.
//
// The re-encode to PNG is deliberate — it means the stored bytes are always an
// allowlisted media type, so `.ico` favicons need no special handling in
// ext_for_media_type on the Rust side.
//
// This must always invoke, including on failure: Rust is holding the loader
// window open waiting for it, and only the overall load timeout would otherwise
// end that wait.
// ---------------------------------------------------------------------------
function __xagaCaptureFavicon() {
    __xagaDiagnostic('favicon_started');
    var dataUri = null;
    // Why the read produced nothing, reported so a silent failure here is
    // diagnosable from the app log rather than only visible as a missing icon.
    var reason = null;
    try {
        var img = document.images && document.images[0];
        if (!img) {
            reason = 'no-image';
        } else if (!img.complete) {
            reason = 'incomplete';
        } else if (!img.naturalWidth) {
            reason = 'decode-failed';
        } else {
            var size = Math.min(img.naturalWidth, img.naturalHeight, 64) || 16;
            var canvas = document.createElement('canvas');
            canvas.width = size;
            canvas.height = size;
            canvas.getContext('2d').drawImage(img, 0, 0, size, size);
            dataUri = canvas.toDataURL('image/png');
            if (!dataUri || dataUri.indexOf('data:image/png') !== 0) {
                reason = 'encode-failed';
                dataUri = null;
            }
        }
    } catch (e) {
        dataUri = null;
        reason = 'canvas-unreadable';
    }
    try {
        window.__TAURI_INTERNALS__.invoke('receive_favicon', {
            id: window.__LOADER_ID__,
            dataUri: dataUri,
            reason: reason
        }).catch(function() { __xagaDiagnostic('ipc_failed'); });
    } catch (e) { __xagaDiagnostic('ipc_failed'); }
}

if (__xagaIsImageDocument() && !window.__LOADER_IMAGE_ONLY__) {
    if (document.readyState === 'complete') __xagaCaptureFavicon();
    else window.addEventListener('load', __xagaCaptureFavicon);
}

window.addEventListener('load', function() {
    __xagaDiagnostic('page_loaded');
    // Favicon documents are handled above; none of the article capture applies.
    if (window.__LOADER_IMAGE_ONLY__) { __xagaCaptureImage(); return; }
    if (__xagaIsImageDocument()) return;

    var debounceTimer = null;
    var sent = false;

    function notifyPageLoaded(url, html, mimeType) {
        try {
            window.__TAURI_INTERNALS__.invoke('loader_page_loaded', {
                html: html,
                mimeType: mimeType,
                // URL only, for the same reason as capture_and_close_loader.
                favicon: __xagaFaviconUrl(),
                faviconData: null
            });
        } catch(e) {}
    }

    function send() {
        if (sent || __xagaTimedOut) return;
        sent = true;
        __xagaDiagnostic('dom_settled');
        observer.disconnect();
        var mimeType = document.contentType || 'text/html';
        if (mimeType === 'application/pdf') {
            try {
                __xagaDiagnostic('capture_sent');
                window.__TAURI_INTERNALS__.invoke('receive_html', {
                    id: window.__LOADER_ID__,
                    html: '',
                    mimeType: mimeType
                }).catch(function() { __xagaDiagnostic('ipc_failed'); });
            } catch(e) {}
            return;
        }
        // Fetch before serialising. The observer is already disconnected, so
        // rewriting src attributes cannot retrigger the debounce.
        __xagaFetchImages().then(function() {
            if (__xagaTimedOut) return;
            __xagaDiagnostic('images_finished');
            __xagaDiagnostic('capture_started');
            var html = document.documentElement.outerHTML;
            var favicon = __xagaFaviconUrl();
            try {
                __xagaDiagnostic('capture_sent');
                window.__TAURI_INTERNALS__.invoke('receive_html', {
                    id: window.__LOADER_ID__,
                    html: html,
                    mimeType: mimeType,
                    // URL only. On the hidden single-shot path Rust navigates
                    // this window to it next and the bytes come back through
                    // receive_favicon; other paths keep the URL and no bytes.
                    favicon: favicon,
                    faviconData: null
                }).then(function(consumed) {
                    // For subsequent page loads (consumed=false), the pending request
                    // was already resolved by a prior load, so notify the main window
                    // directly with the captured URL and HTML.
                    if (window.__LOADER_KEEP_OPEN__ && !consumed) {
                        notifyPageLoaded(window.location.href, html, mimeType);
                    }
                }).catch(function() { __xagaDiagnostic('ipc_failed'); });
            } catch(e) { __xagaDiagnostic('capture_failed'); }
        }).catch(function() { __xagaDiagnostic('capture_failed'); });
    }

    var observer = new MutationObserver(function() {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(send, 300);
    });

    observer.observe(document.documentElement, {
        childList: true,
        subtree: true,
        characterData: true
    });

    debounceTimer = setTimeout(send, 300);

    // Intercept SPA navigation (pushState/replaceState/hashchange) so that
    // in-page URL changes without a full reload also notify the main window.
    if (window.__LOADER_KEEP_OPEN__) {
        var spaTimer = null;
        function notifySpaNavigation() {
            clearTimeout(spaTimer);
            spaTimer = setTimeout(function() {
                var spaMimeType = document.contentType || 'text/html';
                notifyPageLoaded(window.location.href, spaMimeType === 'application/pdf' ? '' : document.documentElement.outerHTML, spaMimeType);
            }, 300);
        }

        var origPush = history.pushState.bind(history);
        var origReplace = history.replaceState.bind(history);
        history.pushState = function() {
            origPush.apply(history, arguments);
            notifySpaNavigation();
        };
        history.replaceState = function() {
            origReplace.apply(history, arguments);
            notifySpaNavigation();
        };
        window.addEventListener('hashchange', notifySpaNavigation);
    }
});

// Direct resource mode reads original bytes from the image's own origin.
// No canvas conversion: content dimensions, encoding and animation survive.
// Both this path and native downloads cross the Rust image validator.
var __xagaImageSent = false;
async function __xagaCaptureImage() {
    if (__xagaImageSent) return;
    __xagaImageSent = true;
    var dataUri = null;
    try {
        if (!__xagaIsImageDocument()) throw new Error('not an image document');
        var response = await fetch(window.location.href, { credentials: 'omit', cache: 'force-cache' });
        if (!response.ok || !response.body) throw new Error('image response unavailable');
        var reader = response.body.getReader();
        var chunks = [];
        var size = 0;
        while (true) {
            var next = await reader.read();
            if (next.done) break;
            size += next.value.byteLength;
            if (size > 8 * 1024 * 1024) { await reader.cancel(); throw new Error('image too large'); }
            chunks.push(next.value);
        }
        var blob = new Blob(chunks, { type: response.headers.get('content-type') || 'application/octet-stream' });
        dataUri = await new Promise(function(resolve, reject) {
            var fileReader = new FileReader();
            fileReader.onload = function() { resolve(fileReader.result); };
            fileReader.onerror = reject;
            fileReader.readAsDataURL(blob);
        });
    } catch (_) { /* A failed resource remains unavailable, never a canvas substitute. */ }
    try {
        await window.__TAURI_INTERNALS__.invoke('receive_image', { id: window.__LOADER_ID__, dataUri: dataUri });
    } catch (_) { __xagaDiagnostic('ipc_failed'); }
}
if (window.__LOADER_IMAGE_ONLY__ && document.readyState === 'complete') __xagaCaptureImage();
