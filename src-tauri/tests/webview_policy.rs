//! Run on macOS with `cargo test -p xaga --test webview_policy` in a GUI session.
//! A custom harness keeps WebKit on the process's main thread. No remote URLs
//! are fetched; both documents and the subframe are supplied as HTML strings.

#[cfg(target_os = "macos")]
#[path = "../src/webview_policy.rs"]
mod webview_policy;

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("WebKit policy smoke test applies to macOS only");
}

#[cfg(target_os = "macos")]
fn main() {
    use block2::RcBlock;
    use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained, runtime::AnyObject};
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{NSDate, NSError, NSRect, NSRunLoop, NSString, NSURL};
    use objc2_web_kit::{WKWebView, WKWebViewConfiguration, WKWebsiteDataStore};
    use std::{
        cell::RefCell,
        rc::Rc,
        time::{Duration, Instant},
    };

    let mtm = MainThreadMarker::new().expect("main thread");
    let _app = NSApplication::sharedApplication(mtm);
    let run_loop = NSRunLoop::mainRunLoop();
    // A secure base URL without fetching it. Verify an unprotected control
    // exposes WebRTC, so an insecure/unsupported context cannot falsely pass.
    let base = NSURL::URLWithString(&NSString::from_str("https://example.invalid/")).unwrap();
    let control = unsafe {
        let config = WKWebViewConfiguration::new(mtm);
        config.setWebsiteDataStore(&WKWebsiteDataStore::nonPersistentDataStore(mtm));
        let _: () = objc2::msg_send![&*config.preferences(), _setPeerConnectionEnabled: objc2::runtime::Bool::YES];
        config
    };
    for (config, blocked) in [
        (control, false),
        (
            webview_policy::loader_configuration(None).expect("WebRTC disabled"),
            true,
        ),
    ] {
        assert!(!unsafe { config.websiteDataStore().isPersistent() });
        let view = unsafe {
            WKWebView::initWithFrame_configuration(WKWebView::alloc(mtm), NSRect::ZERO, &config)
        };

        // Check at document-start, in a subframe, and again after navigation.
        for _ in 0..2 {
            unsafe {
                view.loadHTMLString_baseURL(&NSString::from_str(r#"<!doctype html>
                <script>window.topBlocked = typeof RTCPeerConnection === 'undefined'
                    && typeof webkitRTCPeerConnection === 'undefined';</script>
                <iframe srcdoc="<script>window.blocked = typeof RTCPeerConnection === 'undefined'
                    &amp;&amp; typeof webkitRTCPeerConnection === 'undefined';</script>"></iframe>"#), Some(&base));
            }
            let deadline = Instant::now() + Duration::from_secs(20);
            loop {
                assert!(
                    Instant::now() < deadline,
                    "WebRTC policy/control mismatch or WebKit load failure (expected blocked={blocked})"
                );
                let result = Rc::new(RefCell::new(None));
                let output = result.clone();
                let callback = RcBlock::new(move |value: *mut AnyObject, error: *mut NSError| {
                    let text = if error.is_null() && !value.is_null() {
                        let description: Retained<NSString> =
                            unsafe { objc2::msg_send![value, description] };
                        description.to_string()
                    } else {
                        "pending".into()
                    };
                    *output.borrow_mut() = Some(text);
                });
                unsafe {
                    view.evaluateJavaScript_completionHandler(
                    &NSString::from_str(&format!("window.topBlocked === {blocked} && document.querySelector('iframe')?.contentWindow.blocked === {blocked} ? 'matched' : 'pending'")),
                    Some(&callback),
                );
                }
                while result.borrow().is_none() && Instant::now() < deadline {
                    run_loop.runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.05));
                }
                if result.borrow().as_deref() == Some("matched") {
                    break;
                }
                run_loop.runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.05));
            }
        }
    }
    println!(
        "PASS: WebRTC unavailable at document-start, in subframes, and after navigation; loader storage is ephemeral"
    );
}
