use objc2::{MainThreadMarker, rc::Retained};
use objc2_web_kit::WKWebViewConfiguration;

use crate::webview_policy::loader_configuration;

/// Prepare the image policy before constructing or navigating the loader.
pub(crate) fn configure_article_loader(
    identifier: Option<[u8; 16]>,
    remote_images_allowed: bool,
    complete: impl FnOnce(Result<Retained<WKWebViewConfiguration>, String>) + 'static,
) {
    use block2::RcBlock;
    use objc2_foundation::{NSError, NSString};
    use objc2_web_kit::{WKContentRuleList, WKContentRuleListStore};
    use std::cell::RefCell;

    let config = match loader_configuration(identifier) {
        Ok(config) => config,
        Err(error) => return complete(Err(error)),
    };
    if remote_images_allowed {
        return complete(Ok(config));
    }
    let Some(mtm) = MainThreadMarker::new() else {
        return complete(Err("WebView policy requires the main thread".into()));
    };
    // SAFETY: the WebKit store and configuration are used on the main thread.
    let Some(store) = (unsafe { WKContentRuleListStore::defaultStore(mtm) }) else {
        return complete(Err("WebKit content rule store is unavailable".into()));
    };
    let complete = RefCell::new(Some(complete));
    let callback = RcBlock::new(move |rule: *mut WKContentRuleList, error: *mut NSError| {
        let Some(complete) = complete.take() else {
            return;
        };
        // SAFETY: WebKit passes live objects for the duration of its callback.
        if let Some(rule) = unsafe { rule.as_ref() } {
            unsafe { config.userContentController().addContentRuleList(rule) };
            complete(Ok(config.clone()));
        } else {
            let detail = unsafe { error.as_ref() }
                .map(|error| error.localizedDescription().to_string())
                .unwrap_or_else(|| "no rule returned".to_owned());
            complete(Err(format!(
                "WebKit could not block remote images: {detail}"
            )));
        }
    });
    let name = NSString::from_str("xaga-block-remote-images-v1");
    let rules = NSString::from_str(
        r#"[{"trigger":{"url-filter":"^https?://","resource-type":["image"]},"action":{"type":"block"}}]"#,
    );
    // SAFETY: the arguments are nonnull and WebKit retains the completion block.
    unsafe {
        store.compileContentRuleListForIdentifier_encodedContentRuleList_completionHandler(
            Some(&name),
            Some(&rules),
            Some(&callback),
        )
    };
}
