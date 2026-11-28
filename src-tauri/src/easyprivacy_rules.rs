//! Conservative translation of unqualified EasyPrivacy host rules.
use std::collections::BTreeSet;

pub(super) fn compile(list: &str) -> Result<String, String> {
    let mut hosts = BTreeSet::new();
    let mut exceptions = BTreeSet::new();
    let resource_types = vec![
        "document",
        "image",
        "style-sheet",
        "font",
        "media",
        "svg-document",
        "raw",
    ];
    let mut top_frame_only = false;
    let mut conditional_depth = 0usize;

    for line in list.lines().map(str::trim) {
        if line.starts_with("!#if ") {
            conditional_depth += 1;
        } else if line == "!#endif" {
            conditional_depth = conditional_depth.saturating_sub(1);
        }
        if let Some(disabled_host) = line
            .strip_suffix("$badfilter")
            .and_then(exception_host_prefix)
        {
            exceptions.insert(disabled_host);
        } else if let Some(exception) = line.strip_prefix("@@") {
            if let Some(prefix) = exception_host_prefix(exception) {
                exceptions.insert(prefix);
            } else {
                let options = exception
                    .split_once('$')
                    .map(|(_, options)| options)
                    .unwrap_or("");
                let mut scoped = false;
                for option in options.split(',') {
                    match option {
                        "script" => scoped = true,
                        "subdocument" => {
                            scoped = true;
                            top_frame_only = true;
                        }
                        "third-party" | "~third-party" => {}
                        option if option.starts_with("domain=") => {}
                        _ => {
                            return Err(
                                "EasyPrivacy contains an exception with unsupported scope".into()
                            );
                        }
                    }
                }
                if !scoped {
                    return Err("EasyPrivacy contains an exception with unsupported scope".into());
                }
            }
        } else if let Some(host) = line
            .strip_prefix("||")
            .and_then(|rule| rule.strip_suffix('^'))
        {
            if conditional_depth == 0
                && host.contains('.')
                && host.split('.').all(|label| {
                    !label.is_empty()
                        && label
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                })
            {
                hosts.insert(host.to_ascii_lowercase());
            }
        }
    }

    // Script requests are outside this subset, including exceptions whose
    // publisher or frame conditions would need a separate translation.
    let rules: Vec<_> = hosts
        .into_iter()
        .filter(|host| !overlaps_exception(host, &exceptions))
        .map(|host| {
            let mut trigger = serde_json::json!({
                "url-filter": format!("^https?://{}[:/]", host.replace('.', "\\.")),
                "resource-type": resource_types,
            });
            if top_frame_only {
                trigger["load-context"] = serde_json::json!(["top-frame"]);
            }
            serde_json::json!({ "trigger": trigger, "action": { "type": "block" } })
        })
        .collect();
    if rules.is_empty() {
        return Err("EasyPrivacy has no safely supported blocking rules".into());
    }
    serde_json::to_string(&rules).map_err(|error| error.to_string())
}

fn exception_host_prefix(exception: &str) -> Option<String> {
    let start = exception
        .strip_prefix("||")
        .or_else(|| exception.strip_prefix("|https://"))
        .or_else(|| exception.strip_prefix("|http://"))?;
    let prefix = start
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
        .next()
        .filter(|prefix| !prefix.is_empty())?;
    Some(prefix.to_ascii_lowercase())
}

fn overlaps_exception(host: &str, exceptions: &BTreeSet<String>) -> bool {
    std::iter::once(host)
        .chain(host.match_indices('.').map(|(index, _)| &host[index + 1..]))
        .any(|suffix| {
            // An exception may continue with a wildcard or path after this
            // prefix. Omit the exact-host block without guessing its options.
            (1..=suffix.len()).any(|length| exceptions.contains(&suffix[..length]))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_host_rules_with_potential_exceptions() {
        let rules = compile("||tracker.example^\n||other.example^\n||sub.wild.example^\n||disabled.example^\n||disabled.example^$badfilter\n!#if env_mobile\n||conditional.example^\n!#endif\n@@||tracker.example/allowed\n@@||wild.*$third-party\n@@/frame$subdocument").unwrap();
        let rules: Vec<serde_json::Value> = serde_json::from_str(&rules).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(
            rules[0]["trigger"]["url-filter"],
            r"^https?://other\.example[:/]"
        );
        assert_eq!(rules[0]["trigger"]["load-context"][0], "top-frame");
        assert!(
            !rules[0]["trigger"]["resource-type"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "script")
        );
    }

    #[test]
    fn pinned_snapshot_produces_supported_rules() {
        let rules = compile(include_str!("../resources/easyprivacy/easyprivacy.txt")).unwrap();
        let rules: Vec<serde_json::Value> = serde_json::from_str(&rules).unwrap();
        assert!(!rules.is_empty());
    }
}
