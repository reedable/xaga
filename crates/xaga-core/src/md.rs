//! Intersection of Markdown and domain
//! File system concerns should be managed by `persistence` layer

use std::sync::LazyLock;

use noyalib::ParserConfig;
use noyalib::policy::DenyAnchors;

use crate::types::{Annotation, Article, CollectionItem, Entry, Topic};

/// The frontmatter parser config. `DenyAnchors` rejects any `&anchor`,
/// `*alias`, or `<<:` merge key at parse time; `strict()` applies noyalib's
/// stricter parse preset on top. Anchors in yaml are not supported.
/// Read docs/architecture/security/handling_persisted_content.md
static FM_PARSER: LazyLock<ParserConfig> =
    LazyLock::new(|| ParserConfig::strict().with_policy(DenyAnchors));

/// A `.md` document that could not be read as frontmatter + body.
#[derive(Debug, thiserror::Error)]
pub enum MdError {
    #[error("missing `---` frontmatter delimiters")]
    Delimiter,
    #[error("invalid frontmatter: {0}")]
    Frontmatter(String),
}

/// Represends the .md storage format
pub(crate) struct Markdown {
    pub id: String,
    pub fm: Frontmatter,
    pub body: String,
}

/// Parses both the header fields and the markdown body from a `---`-delimited document.
pub(crate) fn parse_document(raw: &str) -> Result<(Frontmatter, String), MdError> {
    split_document(raw)
}

/// Presence differs from an explicitly stored null or empty string.
pub(crate) fn has_summary_field(raw: &str) -> Result<bool, MdError> {
    parse_document(raw)?;
    let rest = raw
        .trim_start()
        .strip_prefix("---\n")
        .ok_or(MdError::Delimiter)?;
    let end = rest.find("\n---\n").ok_or(MdError::Delimiter)?;
    let fields: std::collections::BTreeMap<String, noyalib::Value> =
        noyalib::from_str_with_config(&rest[..end], &FM_PARSER)
            .map_err(|e| MdError::Frontmatter(e.to_string()))?;
    Ok(fields.contains_key("summary"))
}

/// Merge supplied fields into parsed frontmatter and serialize the mapping.
/// YAML quoting, indentation and collection layout belong to the serializer.
/// Omitted fields retain their values; an omitted body retains its exact bytes.
pub(crate) fn patch_document(
    raw: &str,
    updates: &[(&str, serde_json::Value)],
    body: Option<&str>,
) -> Result<String, MdError> {
    parse_document(raw)?;
    let trimmed = raw.trim_start();
    let prefix = &raw[..raw.len() - trimmed.len()];
    let rest = trimmed.strip_prefix("---\n").ok_or(MdError::Delimiter)?;
    let end = rest.find("\n---\n").ok_or(MdError::Delimiter)?;
    let mut fields: std::collections::BTreeMap<String, noyalib::Value> =
        noyalib::from_str_with_config(&rest[..end], &FM_PARSER)
            .map_err(|e| MdError::Frontmatter(e.to_string()))?;
    for (key, value) in updates {
        fields.insert(
            (*key).to_owned(),
            noyalib::to_value(value).map_err(|e| MdError::Frontmatter(e.to_string()))?,
        );
    }
    // Unsupported image-map storage is omitted on every save (crash_consistency.md).
    fields.remove("images");
    // Block scalars in noyalib 0.0.15 do not preserve embedded carriage returns.
    // Let the library escape them in quoted strings instead.
    let config = noyalib::SerializerConfig::new().block_scalars(false);
    let yaml = noyalib::to_string_with_config(&fields, &config)
        .map_err(|e| MdError::Frontmatter(e.to_string()))?;
    let suffix = match body {
        Some(body) => format!("\n{body}\n"),
        None => rest[end + 5..].to_string(),
    };
    let rendered = format!(
        "{prefix}---\n{}\n---\n{suffix}",
        yaml.trim_end_matches('\n')
    );
    parse_document(&rendered)?;
    Ok(rendered)
}

#[cfg(test)]
pub(crate) fn test_frontmatter_values(
    raw: &str,
) -> std::collections::BTreeMap<String, noyalib::Value> {
    let yaml = raw
        .trim_start()
        .strip_prefix("---\n")
        .unwrap()
        .split_once("\n---\n")
        .unwrap()
        .0;
    noyalib::from_str_with_config(yaml, &FM_PARSER).unwrap()
}

/// Converts .md storage format to Topic domain object
pub(crate) fn md_to_topic(md: Markdown) -> Topic {
    let is_pinned = crate::tags::is_pinned(&md.fm.tags);

    Topic {
        id: md.id.to_string(),
        title: md.fm.title,
        created_at: md.fm.created_at,
        updated_at: md.fm.updated_at,
        description: md.fm.summary,
        threshold: md.fm.threshold,
        is_pinned,
        entry_count: 0,
        unread_count: 0,
        members: md
            .fm
            .items
            .iter()
            .map(crate::types::Reference::from)
            .collect(),
    }
}

/// Converts .md storage format to Article domain object
pub(crate) fn md_to_article(md: Markdown) -> Article {
    let article_id = md.id.clone();
    let annotations = md.fm.annotations.clone();
    let content = md.body.clone();
    let favicon = md.fm.favicon.clone();

    Article {
        meta: md_to_entry(md),
        annotations: with_reference(annotations, &article_id),
        content,
        favicon,
    }
}

/// Enriches annotations vector with their reference id (article_id)
fn with_reference(mut annotations: Vec<Annotation>, article_id: &str) -> Vec<Annotation> {
    for a in &mut annotations {
        a.reference = article_id.to_string();
    }

    annotations
}

/// Converts .md storage format to Entry domain object
pub(crate) fn md_to_entry(md: Markdown) -> Entry {
    let is_unread = crate::tags::is_unread(&md.fm.tags);
    let is_bookmarked = crate::tags::is_bookmarked(&md.fm.tags);

    Entry {
        id: md.id,
        url: md.fm.source,
        title: md.fm.title,
        created_at: md.fm.created_at,
        published_at: md.fm.published_at,
        updated_at: md.fm.updated_at,
        description: md.fm.summary,
        is_unread,
        is_bookmarked,
        color: None,
    }
}

// ── Frontmatter: the `---`-delimited header parsing (the `.md` inbound boundary) ──

/// The parsed header fields of a `.md` document.
pub struct Frontmatter {
    pub source: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub summary: Option<String>,
    pub annotations: Vec<Annotation>,
    pub threshold: Option<f32>,
    pub tags: Vec<String>,
    pub favicon: Option<String>,
    pub items: Vec<CollectionItem>,
}

/// Parses only the header fields from a `---`-delimited document.
pub(crate) fn parse_frontmatter(raw: &str) -> Result<Frontmatter, MdError> {
    Ok(split_document(raw)?.0)
}

/// The `.md` frontmatter storage shape for an annotation. Storage keeps `note`; the domain
/// `Annotation`'s `title`/`content` are derived here at the boundary (title = highlighted text,
/// content = note). Mapped to/from the domain type by the `From` impls below.
#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct FrontmatterAnnotation {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reference: String,
    pub text: Vec<String>,
    pub occurrence: Vec<u32>,
    pub color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub created_at: String,
}

impl From<FrontmatterAnnotation> for Annotation {
    fn from(a: FrontmatterAnnotation) -> Self {
        Annotation {
            title: a.text.join(" "),
            content: a.note.unwrap_or_default(),
            id: a.id,
            reference: a.reference,
            text: a.text,
            occurrence: a.occurrence,
            color: a.color,
            created_at: a.created_at,
        }
    }
}

impl From<&Annotation> for FrontmatterAnnotation {
    fn from(a: &Annotation) -> Self {
        FrontmatterAnnotation {
            id: a.id.clone(),
            reference: a.reference.clone(),
            text: a.text.clone(),
            occurrence: a.occurrence.clone(),
            color: a.color.clone(),
            note: (!a.content.is_empty()).then(|| a.content.clone()),
            created_at: a.created_at.clone(),
        }
    }
}

#[derive(serde::Deserialize, Default)]
struct FrontmatterRaw {
    #[serde(default)]
    source: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    created: String,
    #[serde(default)]
    published: Option<String>,
    #[serde(default)]
    updated: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    annotations: Vec<FrontmatterAnnotation>,
    #[serde(default)]
    threshold: Option<f32>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    favicon: Option<String>,
    #[serde(default)]
    items: Vec<CollectionItem>,
}

pub(crate) fn split_document(raw: &str) -> Result<(Frontmatter, String), MdError> {
    let raw = raw.trim_start();
    let rest = raw.strip_prefix("---\n").ok_or(MdError::Delimiter)?;
    let end = rest.find("\n---\n").ok_or(MdError::Delimiter)?;
    let yaml = &rest[..end];
    let body = rest[end + 5..].trim_matches('\n').to_string();

    let fm: FrontmatterRaw = noyalib::from_str_with_config(yaml, &FM_PARSER)
        .map_err(|e| MdError::Frontmatter(e.to_string()))?;

    Ok((
        Frontmatter {
            source: fm.source,
            title: fm.title,
            created_at: fm.created,
            published_at: fm.published,
            updated_at: fm.updated,
            summary: fm.summary,
            annotations: fm.annotations.into_iter().map(Into::into).collect(),
            threshold: fm.threshold,
            tags: fm.tags,
            favicon: fm.favicon,
            items: fm.items,
        },
        body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_edits_preserve_siblings_and_escape_multiline_values() {
        let raw = "---\nsource: https://example.com/a\nsummary: |\n  Old summary\n# Keep this comment\ntags: [research]\ncustom: {nested: [one, two]}\n---\n\nBody\n\n";
        let summary = "New summary\n---\ntags: [injected]\n";
        let patched =
            patch_document(raw, &[("summary", serde_json::json!(summary))], None).unwrap();
        let (fm, _) = parse_document(&patched).unwrap();
        assert_eq!(fm.summary.as_deref(), Some(summary));
        assert_eq!(fm.tags, vec!["research"]);
        assert_eq!(
            test_frontmatter_values(&patched)["custom"],
            test_frontmatter_values(raw)["custom"]
        );
        assert!(patched.ends_with("---\n\nBody\n\n"));
    }

    #[test]
    fn summary_carriage_returns_roundtrip_without_changing_other_fields() {
        let raw =
            "---\nsource: https://example.org/a\ntitle: Saved\ntags: [research]\n---\n\nBody\n\n";
        for summary in [
            "First\r<br/>\nSecond",
            "First\r\nSecond\r\n",
            "First\rSecond",
            "Quote: \"text\"\r<br/>\n\r<br/>\nLast\n\n",
        ] {
            let saved =
                patch_document(raw, &[("summary", serde_json::json!(summary))], None).unwrap();
            let mut expected = test_frontmatter_values(raw);
            expected.insert("summary".into(), noyalib::Value::String(summary.into()));
            assert_eq!(test_frontmatter_values(&saved), expected);
            assert!(saved.ends_with("---\n\nBody\n\n"));
        }
    }

    #[test]
    fn frontmatter_edits_keep_the_strict_anchor_policy() {
        let raw = "---\nsource: https://example.com/a\ncustom: &secret [one, two]\nother: *secret\n---\nBody\n";
        assert!(patch_document(raw, &[("title", serde_json::json!("New"))], None).is_err());
    }

    fn doc(fm: &str) -> String {
        format!("---\n{fm}\n---\n\nbody\n")
    }

    #[test]
    fn parses_frontmatter_ending_in_a_block_sequence() {
        let raw = doc(
            "source: https://example.com/x\ntitle: T\ncreated: '2026-01-01'\n\
             annotations:\n  - id: a1\n    text:\n      - hi\n    \
             occurrence:\n      - 0\n    color: red\n    created_at: '2026-01-01'",
        );
        let (fm, body) = split_document(&raw).expect("valid frontmatter parses");
        assert_eq!(fm.title, "T");
        assert_eq!(fm.annotations.len(), 1);
        assert_eq!(body, "body");
    }

    #[test]
    fn rejects_anchors_and_aliases() {
        let raw = doc("title: &a hello\nsource: *a");
        assert!(matches!(split_document(&raw), Err(MdError::Frontmatter(_))));
    }

    #[test]
    fn rejects_alias_expansion_bomb() {
        // Nested-alias ladder (billion-laughs class): must be rejected.
        let mut fm = String::from("a0: &a0 x\n");
        for i in 1..=8 {
            let p = format!("*a{}", i - 1);
            fm.push_str(&format!(
                "a{i}: &a{i} [{p},{p},{p},{p},{p},{p},{p},{p},{p},{p}]\n"
            ));
        }
        fm.push_str("title: last");
        assert!(split_document(&doc(&fm)).is_err());
    }

    #[test]
    fn rejects_deeply_nested_frontmatter() {
        // Recursion-depth guard (the non-alias half of the billion-laughs class):
        // a pathologically deep structure must be rejected by the parser's depth
        // limit, not recursed into. Depth far exceeds any sane limit, so a bounded
        // parser errors early; an unbounded one would recurse (and could overflow).
        let depth = 50_000;
        let nested = format!("title: {}{}", "[".repeat(depth), "]".repeat(depth));
        assert!(split_document(&doc(&nested)).is_err());
    }

    #[test]
    fn parses_a_real_serde_yaml_written_file() {
        // A file as serde_yaml 0.9 wrote it: unquoted RFC3339 dates, an
        // `aliases:` field (a key literally named "aliases", NOT a YAML alias),
        // HTML in `summary`. Must still parse under strict() + DenyAnchors.
        let raw = "---\n\
            source: https://thewrap.com/x\n\
            title: 'Rob Bonta Rebuffs FCC'\n\
            aliases:\n- 'Rob Bonta Rebuffs FCC'\n\
            created: 2026-07-16T11:31:50.924447+00:00\n\
            published: 2026-07-16T02:03:31+00:00\n\
            updated: 2026-07-16T05:49:57+00:00\n\
            summary: '<p>\"Spinning off one channel\" the AG adds</p>'\n\
            tags:\n- unread\n\
            ---\n\n\
            body\n";
        let (fm, _body) = split_document(raw).expect("real serde_yaml file parses");
        assert_eq!(fm.created_at, "2026-07-16T11:31:50.924447+00:00");
        assert_eq!(fm.tags, vec!["unread".to_string()]);
        assert!(fm.summary.unwrap().contains("Spinning off"));
    }

    #[test]
    fn missing_delimiters_is_a_delimiter_error() {
        assert!(matches!(
            split_document("no frontmatter here"),
            Err(MdError::Delimiter)
        ));
    }
}
