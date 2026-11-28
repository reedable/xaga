use objc2::{MainThreadMarker, rc::Retained};
use objc2_web_kit::WKWebViewConfiguration;

use crate::webview_policy::loader_configuration;

fn configure_remote_images(
    mtm: MainThreadMarker,
    config: Retained<WKWebViewConfiguration>,
    remote_images_allowed: bool,
    complete: impl FnOnce(Result<Retained<WKWebViewConfiguration>, String>) + 'static,
) {
    if remote_images_allowed {
        return complete(Ok(config));
    }
    crate::loader_rule_store::load(
        mtm,
        "xaga-block-remote-images-v1".into(),
        || {
            Ok(r#"[{"trigger":{"url-filter":"^https?://","resource-type":["image"]},"action":{"type":"block"}}]"#.into())
        },
        move |result| {
            complete(
                result
                    .map(|rule| {
                        // SAFETY: the controller and rule are used on the main thread.
                        unsafe { config.userContentController().addContentRuleList(&rule) };
                        config
                    })
                    .map_err(|error| format!("WebKit could not block remote images: {error}")),
            );
        },
    );
}

/// Attach request policies before constructing or navigating the loader.
pub(crate) fn configure_article_loader(
    identifier: Option<[u8; 16]>,
    remote_images_allowed: bool,
    easyprivacy: Option<crate::article_loader::RuleList>,
    complete: impl FnOnce(Result<Retained<WKWebViewConfiguration>, String>) + 'static,
) {
    let config = match loader_configuration(identifier) {
        Ok(config) => config,
        Err(error) => return complete(Err(error)),
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return complete(Err("WebView policy requires the main thread".into()));
    };
    configure_remote_images(mtm, config, remote_images_allowed, move |result| {
        let config = match result {
            Ok(config) => config,
            Err(error) => return complete(Err(error)),
        };
        let Some((identifier, rules)) = easyprivacy else {
            return complete(Ok(config));
        };
        crate::loader_rule_store::load(mtm, identifier, rules, move |result| {
            complete(
                result
                    .map(|rule| {
                        // SAFETY: the controller and rule are used on the main thread.
                        unsafe { config.userContentController().addContentRuleList(&rule) };
                        config
                    })
                    .map_err(|error| format!("WebKit could not prepare EasyPrivacy: {error}")),
            );
        });
    });
}
