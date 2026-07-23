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
var __XAGA_FETCH_IMAGES__ = true;
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
    return Promise.race([
        promise,
        new Promise(function(resolve) { setTimeout(function() { resolve(onTimeout); }, ms); })
    ]);
}

// Fetches an image and returns a data: URL, or null if it cannot be read
// (opaque/CORS-blocked response, HTTP error, non-image, or over the per-image
// cap). `budget` carries the running total of fetched bytes to enforce the
// per-load cap. Never throws — a rejection (typically a cross-origin image
// without permissive CORS headers) resolves to null.
function __xagaFetchAsDataUrl(url, budget) {
    return fetch(url, { credentials: 'omit', cache: 'force-cache' })
        .then(function(res) {
            if (res.type === 'opaque' || !res.ok) return null;
            var len = res.headers.get('content-length');
            if (len && Number(len) > __XAGA_IMAGE_MAX_BYTES__) return null;
            return res.blob();
        })
        .then(function(blob) {
            if (!blob) return null;
            if (blob.size > __XAGA_IMAGE_MAX_BYTES__) return null;
            if (blob.type && blob.type.indexOf('image/') !== 0) return null;
            if (budget.bytes + blob.size > __XAGA_IMAGE_TOTAL_BYTES__) return null;
            return new Promise(function(resolve) {
                var reader = new FileReader();
                reader.onload = function() { budget.bytes += blob.size; resolve(reader.result); };
                reader.onerror = function() { resolve(null); };
                reader.readAsDataURL(blob);
            });
        })
        .catch(function() { return null; });
}

function __xagaFetchImages() {
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
        .catch(function() {});
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
    __xagaFetchImages().then(function() {
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
        // Fetch before serialising. The observer is already disconnected, so
        // rewriting src attributes cannot retrigger the debounce.
        __xagaFetchImages().then(function() {
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
