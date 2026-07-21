window.open = function() { return null; };
window.focus = function() {};

// ---------------------------------------------------------------------------
// EXPERIMENT (measurement only): inline remote images as data: URLs before the
// HTML is captured, and record how often that succeeds.
//
// The open question is whether the page can *read* its own images: an image
// from a host that sends no `Access-Control-Allow-Origin` can be displayed but
// not read, so fetch() rejects and canvas becomes tainted. That is a WKWebView
// runtime behaviour — no unit test answers it. This measures the real hit rate
// against real articles.
//
// Failures leave the original remote `src` untouched, so rendering behaviour is
// unchanged from today. That is a property of the experiment (measure without
// changing what the user sees), not a decision about what the shipped feature
// should do on failure — that behaviour is still open.
//
// Stats ride out in a <meta> tag inside the captured HTML and are logged in
// load_via_webview. They are NOT sent via a new IPC command: LOADER_COMMANDS
// (src-tauri/src/commands/mod.rs) is a closed allow-list for untrusted loader
// webviews, and widening it for a throwaway measurement is not worth it.
// Set __XAGA_INLINE_IMAGES__ to false to disable.
// ---------------------------------------------------------------------------
var __XAGA_INLINE_IMAGES__ = true;
var __XAGA_INLINE_MAX_BYTES__ = 2 * 1024 * 1024;
var __XAGA_INLINE_TOTAL_BYTES__ = 24 * 1024 * 1024;
var __XAGA_INLINE_CONCURRENCY__ = 6;

// The Rust side aborts the whole load after __LOADER_TIMEOUT_MS__
// (DEFAULT_LOADER_TIMEOUT_SECS = 10s), and that budget must also cover the page
// load itself. Inlining therefore gets a fraction of what remains, never a fixed
// figure — overrunning it does not merely skip images, it fails the article.
var __XAGA_LOADER_BUDGET_MS__ =
    typeof window.__LOADER_TIMEOUT_MS__ === 'number' && window.__LOADER_TIMEOUT_MS__ > 0
        ? window.__LOADER_TIMEOUT_MS__
        : 10000;
var __XAGA_INLINE_OVERALL_MS__ = Math.max(1500, Math.floor(__XAGA_LOADER_BUDGET_MS__ * 0.45));
var __XAGA_INLINE_PER_IMAGE_MS__ = Math.max(750, Math.floor(__XAGA_INLINE_OVERALL_MS__ / 3));

function __xagaWithTimeout(promise, ms, onTimeout) {
    return Promise.race([
        promise,
        new Promise(function(resolve) { setTimeout(function() { resolve(onTimeout); }, ms); })
    ]);
}

function __xagaFetchAsDataUrl(url, stats) {
    return fetch(url, { credentials: 'omit', cache: 'force-cache' })
        .then(function(res) {
            // An opaque response (no-cors fallback) has unreadable bytes.
            if (res.type === 'opaque') { stats.failedOpaque++; return null; }
            if (!res.ok) { stats.failedHttp++; return null; }
            var len = res.headers.get('content-length');
            if (len && Number(len) > __XAGA_INLINE_MAX_BYTES__) { stats.failedTooBig++; return null; }
            return res.blob();
        })
        .then(function(blob) {
            if (!blob) return null;
            if (blob.size > __XAGA_INLINE_MAX_BYTES__) { stats.failedTooBig++; return null; }
            if (blob.type && blob.type.indexOf('image/') !== 0) { stats.failedNotImage++; return null; }
            if (stats.bytes + blob.size > __XAGA_INLINE_TOTAL_BYTES__) { stats.failedBudget++; return null; }
            return new Promise(function(resolve) {
                var reader = new FileReader();
                reader.onload = function() { stats.bytes += blob.size; resolve(reader.result); };
                reader.onerror = function() { stats.failedRead++; resolve(null); };
                reader.readAsDataURL(blob);
            });
        })
        .catch(function(e) {
            // A cross-origin image without permissive CORS headers lands here as
            // a TypeError. Network errors are indistinguishable from this side.
            stats.failedFetch++;
            stats.errors[e && e.name ? e.name : 'Error'] =
                (stats.errors[e && e.name ? e.name : 'Error'] || 0) + 1;
            return null;
        });
}

function __xagaInlineImages() {
    var stats = {
        total: 0, inlined: 0, sameOrigin: 0, alreadyData: 0,
        failedFetch: 0, failedOpaque: 0, failedHttp: 0, failedTooBig: 0,
        failedNotImage: 0, failedRead: 0, failedBudget: 0, failedTimeout: 0,
        withSrcset: 0, cssBackgrounds: 0, bytes: 0, errors: {}, elapsedMs: 0
    };

    if (!__XAGA_INLINE_IMAGES__) return Promise.resolve(stats);

    var started = Date.now();
    var imgs = [];
    var all = document.images ? [].slice.call(document.images) : [];

    for (var i = 0; i < all.length; i++) {
        var img = all[i];
        var src = img.currentSrc || img.src || '';
        if (!src) continue;
        stats.total++;
        if (src.indexOf('data:') === 0) { stats.alreadyData++; continue; }
        if (!/^https?:/i.test(src)) continue;
        if (img.getAttribute('srcset') || img.getAttribute('sizes')) stats.withSrcset++;
        imgs.push(img);
    }

    // Count CSS background images too — a second copy of the same channel that
    // rewriting <img> does not touch. Counted only, not inlined.
    try {
        var els = document.querySelectorAll('*');
        for (var k = 0; k < els.length; k++) {
            var bg = els[k].style && els[k].style.backgroundImage;
            if (bg && bg.indexOf('url(') !== -1 && bg.indexOf('data:') === -1) stats.cssBackgrounds++;
        }
    } catch (e) {}

    var cursor = 0;
    function worker() {
        if (cursor >= imgs.length) return Promise.resolve();
        var img = imgs[cursor++];
        var src = img.currentSrc || img.src;
        return __xagaWithTimeout(
            __xagaFetchAsDataUrl(src, stats),
            __XAGA_INLINE_PER_IMAGE_MS__,
            '__timeout__'
        ).then(function(dataUrl) {
            if (dataUrl === '__timeout__') { stats.failedTimeout++; return; }
            if (!dataUrl) return;
            img.setAttribute('src', dataUrl);
            // srcset would let the renderer pick a remote candidate anyway.
            img.removeAttribute('srcset');
            img.removeAttribute('sizes');
            img.removeAttribute('loading');
            stats.inlined++;
        }).catch(function() {}).then(worker);
    }

    var pool = [];
    for (var w = 0; w < Math.min(__XAGA_INLINE_CONCURRENCY__, imgs.length); w++) pool.push(worker());

    return __xagaWithTimeout(Promise.all(pool), __XAGA_INLINE_OVERALL_MS__, null)
        .catch(function() {})
        .then(function() {
            stats.elapsedMs = Date.now() - started;
            try {
                var meta = document.createElement('meta');
                meta.setAttribute('name', 'xaga-inline-stats');
                meta.setAttribute('content', JSON.stringify(stats));
                (document.head || document.documentElement).appendChild(meta);
            } catch (e) {}
            return stats;
        });
}

window.addEventListener('beforeunload', function() {
    try {
        window.__TAURI_INTERNALS__.invoke('loader_will_unload', {
            url: window.location.href
        });
    } catch(e) {}
});

window.__captureAndClose = function() {
    var mimeType = document.contentType || 'text/html';
    if (mimeType === 'application/pdf') {
        window.__TAURI_INTERNALS__.invoke('capture_and_close_loader', {
            label: window.__LOADER_LABEL__,
            html: '',
            loaderUrl: window.__LOADER_URL__,
            pageUrl: window.location.href,
            mimeType: mimeType
        });
        return;
    }
    __xagaInlineImages().then(function() {
        window.__TAURI_INTERNALS__.invoke('capture_and_close_loader', {
            label: window.__LOADER_LABEL__,
            html: document.documentElement.outerHTML,
            loaderUrl: window.__LOADER_URL__,
            pageUrl: window.location.href,
            mimeType: mimeType
        });
    });
};

window.addEventListener('load', function() {
    var debounceTimer = null;
    var sent = false;

    function notifyPageLoaded(url, html, mimeType) {
        try {
            window.__TAURI_INTERNALS__.invoke('loader_page_loaded', {
                url: url,
                html: html,
                mimeType: mimeType
            });
        } catch(e) {}
    }

    function send() {
        if (sent) return;
        sent = true;
        observer.disconnect();
        var mimeType = document.contentType || 'text/html';
        if (mimeType === 'application/pdf') {
            try {
                window.__TAURI_INTERNALS__.invoke('receive_html', {
                    id: window.__LOADER_ID__,
                    html: '',
                    mimeType: mimeType
                }).catch(function() {});
            } catch(e) {}
            return;
        }
        // Inline before serialising. The observer is already disconnected, so
        // rewriting src attributes cannot retrigger the debounce.
        __xagaInlineImages().then(function() {
            var html = document.documentElement.outerHTML;
            try {
                window.__TAURI_INTERNALS__.invoke('receive_html', {
                    id: window.__LOADER_ID__,
                    html: html,
                    mimeType: mimeType
                }).then(function(consumed) {
                    // For subsequent page loads (consumed=false), the pending request
                    // was already resolved by a prior load, so notify the main window
                    // directly with the captured URL and HTML.
                    if (window.__LOADER_KEEP_OPEN__ && !consumed) {
                        notifyPageLoaded(window.location.href, html, mimeType);
                    }
                }).catch(function() {});
            } catch(e) {}
        });
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
