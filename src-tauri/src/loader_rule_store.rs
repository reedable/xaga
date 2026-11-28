use block2::RcBlock;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_foundation::{NSError, NSString};
use objc2_web_kit::{WKContentRuleList, WKContentRuleListStore};
use std::cell::RefCell;
use std::collections::HashMap;

type RuleResult = Result<Retained<WKContentRuleList>, String>;
type Completion = Box<dyn FnOnce(RuleResult)>;

thread_local! {
    // WebKit persists compiled lists. This map only coalesces concurrent lookups
    // and compilations for the same identifier on the main thread.
    static PENDING: RefCell<HashMap<String, Vec<Completion>>> = RefCell::default();
}

fn finish(identifier: &str, result: RuleResult) {
    let waiting = PENDING.with(|pending| pending.borrow_mut().remove(identifier));
    for complete in waiting.into_iter().flatten() {
        complete(result.clone());
    }
}

pub(super) fn load(
    mtm: MainThreadMarker,
    identifier: String,
    rules: impl FnOnce() -> Result<String, String> + 'static,
    complete: impl FnOnce(RuleResult) + 'static,
) {
    let waiting = PENDING.with(|pending| {
        let mut pending = pending.borrow_mut();
        let waiting = pending.entry(identifier.clone()).or_default();
        let already_loading = !waiting.is_empty();
        waiting.push(Box::new(complete));
        already_loading
    });
    if waiting {
        return;
    }

    // SAFETY: the store and callbacks are used on the main thread.
    let Some(store) = (unsafe { WKContentRuleListStore::defaultStore(mtm) }) else {
        return finish(
            &identifier,
            Err("WebKit content rule store is unavailable".into()),
        );
    };
    let name = NSString::from_str(&identifier);
    let store_for_lookup = store.clone();
    let name_for_compile = name.clone();
    let rules = RefCell::new(Some(rules));
    let lookup = RcBlock::new(move |rule: *mut WKContentRuleList, _: *mut NSError| {
        // SAFETY: WebKit supplies a live object for the duration of the callback.
        if let Some(rule) = unsafe { Retained::retain(rule) } {
            return finish(&identifier, Ok(rule));
        }
        let Some(rules) = rules.take() else {
            return;
        };
        let rules = match rules() {
            Ok(rules) => NSString::from_str(&rules),
            Err(error) => return finish(&identifier, Err(error)),
        };
        let identifier = identifier.clone();
        let compiled = RcBlock::new(move |rule: *mut WKContentRuleList, error: *mut NSError| {
            // SAFETY: the callback arguments are valid for the callback duration.
            let result = unsafe { Retained::retain(rule) }.ok_or_else(|| {
                unsafe { error.as_ref() }
                    .map(|error| error.localizedDescription().to_string())
                    .unwrap_or_else(|| "no rule list returned".into())
            });
            finish(&identifier, result);
        });
        // SAFETY: arguments are nonnull and WebKit retains the completion block.
        unsafe {
            store_for_lookup
                .compileContentRuleListForIdentifier_encodedContentRuleList_completionHandler(
                    Some(&name_for_compile),
                    Some(&rules),
                    Some(&compiled),
                );
        }
    });
    // SAFETY: arguments are nonnull and WebKit retains the completion block.
    unsafe {
        store.lookUpContentRuleListForIdentifier_completionHandler(Some(&name), Some(&lookup))
    };
}
