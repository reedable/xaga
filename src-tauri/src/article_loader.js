window.open = function() { return null; };
window.focus = function() {};

window.addEventListener('beforeunload', function() {
    try {
        window.__TAURI_INTERNALS__.invoke('loader_will_unload', {
            url: window.location.href
        });
    } catch(e) {}
});

window.__captureAndClose = function() {
    var mimeType = document.contentType || 'text/html';
    window.__TAURI_INTERNALS__.invoke('capture_and_close_loader', {
        label: window.__LOADER_LABEL__,
        html: mimeType === 'application/pdf' ? '' : document.documentElement.outerHTML,
        url: window.__LOADER_URL__,
        mimeType: mimeType
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
        var html = mimeType === 'application/pdf' ? '' : document.documentElement.outerHTML;
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
