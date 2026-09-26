//! Native macOS policy, applied before constructing a WKWebView.
//!
//! An HTTP proxy does not contain WebRTC or all speculative DNS lookups.
//! Disable these in the engine, including subframes, before loading content.

use objc2::{MainThreadMarker, msg_send, rc::Retained, runtime::Bool, sel};
use objc2_foundation::{NSObjectProtocol, NSUUID};
use objc2_web_kit::{WKWebViewConfiguration, WKWebsiteDataStore};

pub fn configuration() -> Result<Retained<WKWebViewConfiguration>, String> {
    let mtm = MainThreadMarker::new().ok_or("WebView policy requires the main thread")?;
    // SAFETY: created on the main thread, before any WebView uses it.
    let config = unsafe { WKWebViewConfiguration::new(mtm) };
    // SAFETY: the configuration owns live WKPreferences; we are on the main
    // thread. These WebKit SPI selectors take/return Objective-C BOOL. Check
    // availability and read back the value; never load content unprotected.
    unsafe {
        let prefs = config.preferences();
        if !prefs.respondsToSelector(sel!(_setPeerConnectionEnabled:))
            || !prefs.respondsToSelector(sel!(_peerConnectionEnabled))
        {
            return Err("WebKit cannot enforce Xaga's WebRTC policy".into());
        }
        let _: () = msg_send![&*prefs, _setPeerConnectionEnabled: Bool::NO];
        let enabled: Bool = msg_send![&*prefs, _peerConnectionEnabled];
        if enabled.as_bool() {
            return Err("WebKit did not disable WebRTC".into());
        }
        // Speculative DNS can bypass an HTTP proxy on some WebKit versions.
        // Disable it in the engine too; a page's dns-prefetch hint must not
        // resolve .local before the proxy ever receives a request.
        if !prefs.respondsToSelector(sel!(_setDNSPrefetchingEnabled:))
            || !prefs.respondsToSelector(sel!(_dnsPrefetchingEnabled))
        {
            return Err("WebKit cannot enforce Xaga's DNS prefetch policy".into());
        }
        let _: () = msg_send![&*prefs, _setDNSPrefetchingEnabled: Bool::NO];
        let enabled: Bool = msg_send![&*prefs, _dnsPrefetchingEnabled];
        if enabled.as_bool() {
            return Err("WebKit did not disable DNS prefetch".into());
        }
    }
    Ok(config)
}

pub fn loader_configuration(
    identifier: Option<[u8; 16]>,
) -> Result<Retained<WKWebViewConfiguration>, String> {
    let config = configuration()?;
    let mtm = MainThreadMarker::new().ok_or("WebView policy requires the main thread")?;
    // Wry uses an explicitly supplied configuration's data store, ignoring its
    // incognito/data_store_identifier builder options. Preserve that isolation.
    // SAFETY: main thread; named data stores are available on our macOS 14+
    // deployment target. The configuration retains the supplied store.
    unsafe {
        let store = match identifier {
            Some(bytes) => {
                WKWebsiteDataStore::dataStoreForIdentifier(&NSUUID::from_bytes(bytes), mtm)
            }
            None => WKWebsiteDataStore::nonPersistentDataStore(mtm),
        };
        config.setWebsiteDataStore(&store);
    };
    Ok(config)
}
