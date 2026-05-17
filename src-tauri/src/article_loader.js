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
    window.__TAURI_INTERNALS__.invoke('capture_and_close_loader', {
        label: window.__LOADER_LABEL__,
        html: document.documentElement.outerHTML,
        url: window.__LOADER_URL__
    });
};

window.addEventListener('load', function() {
    var debounceTimer = null;
    var sent = false;

    function send() {
        if (sent) return;
        sent = true;
        observer.disconnect();
        try {
            window.__TAURI_INTERNALS__.invoke('receive_html', {
                id: window.__LOADER_ID__,
                html: document.documentElement.outerHTML
            });
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
});
