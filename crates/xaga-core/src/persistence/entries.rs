use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use crate::{
    library_writer::WriteToken,
    md::{FrontmatterAnnotation, parse_frontmatter},
    md::{Markdown, md_to_entry, parse_document},
    types::{Article, Entry},
    util::{normalize_url, url_hash12},
};

#[derive(Debug, Error)]
pub enum EntryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Invalid id")]
    InvalidId,
    #[error("Entry not found")]
    NotFound,
    #[error("malformed frontmatter: {0}")]
    Malformed(#[from] crate::md::MdError),
    #[error("file too large (max {max_bytes} bytes)")]
    TooLarge { max_bytes: u64 },
    #[error("a color change is delete/create — use replace_annotation")]
    ColorChangeRequiresReplace,
}

impl From<crate::fs::ReadMdError> for EntryError {
    fn from(e: crate::fs::ReadMdError) -> Self {
        match e {
            crate::fs::ReadMdError::TooLarge { max_bytes } => EntryError::TooLarge { max_bytes },
            crate::fs::ReadMdError::Io(e) => EntryError::Io(e),
        }
    }
}

impl EntryError {
    /// Stable code the UI matches on — mirrored in `src/platform/errors.ts`.
    /// See `crate::error::DomainError` for the funnel that carries it.
    pub fn code(&self) -> &'static str {
        match self {
            EntryError::Io(_) => "entry:io",
            EntryError::InvalidUrl(_) => "entry:invalid-url",
            EntryError::InvalidId => "entry:invalid-id",
            EntryError::NotFound => "entry:not-found",
            EntryError::Malformed(_) => "entry:malformed",
            EntryError::TooLarge { .. } => "entry:too-large",
            EntryError::ColorChangeRequiresReplace => "entry:color-change-requires-replace",
        }
    }

    /// User-safe message (no file fragments, ids, or URLs). The full technical
    /// text stays in `Display`, which the funnel logs as the error's detail.
    pub fn user_message(&self) -> &'static str {
        match self {
            EntryError::Io(_) => "Couldn't read or write this item.",
            EntryError::InvalidUrl(_) => "This item's address is invalid.",
            EntryError::InvalidId => "This item's identifier is invalid.",
            EntryError::NotFound => "This item could not be found.",
            EntryError::Malformed(_) => "This item's file is malformed and couldn't be read.",
            EntryError::TooLarge { .. } => "This item's file is too large to open.",
            EntryError::ColorChangeRequiresReplace => "Couldn't update the highlight color.",
        }
    }
}

/// Returns true when `id` is safe to interpolate into a file path: a single
/// path component with no separators or parent-directory references. Entry and
/// article ids are URL hashes (`url_hash12`) or Atom ids — never paths — so a
/// value containing `/`, `\`, `..` or a NUL byte is an attempted traversal.
pub(crate) fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains("..")
        && !id.contains('\0')
}

// ── Frontmatter rendering ─────────────────────────────────────────────────────

/// Encodes an [`Article`] to its `.md` representation (frontmatter + body) — the single
/// entry/article encoder. `tags` and `aliases` are reconstructed from the domain object (state
/// booleans / title); both are storage-only keys the domain doesn't carry directly.
pub(crate) fn render_entry(article: &Article) -> String {
    #[derive(serde::Serialize)]
    struct Fm<'a> {
        source: &'a str,
        title: &'a str,
        aliases: Vec<&'a str>,
        created: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        published: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        updated: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        summary: Option<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        annotations: Vec<FrontmatterAnnotation>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        tags: &'a [String],
        #[serde(skip_serializing_if = "Option::is_none")]
        favicon: Option<&'a str>,
        #[serde(skip_serializing_if = "<[_]>::is_empty")]
        images: &'a [crate::types::ImageRef],
    }

    let meta = &article.meta;
    let mut tags: Vec<String> = Vec::new();
    if meta.is_unread {
        tags.push(crate::tags::UNREAD.to_string());
    }
    if meta.is_bookmarked {
        tags.push(crate::tags::BOOKMARK.to_string());
    }

    let clean_summary = meta
        .description
        .as_deref()
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "));

    let fm = Fm {
        source: &meta.url,
        title: &meta.title,
        aliases: vec![meta.title.as_str()],
        created: &meta.created_at,
        published: meta.published_at.as_deref(),
        updated: meta.updated_at.as_deref(),
        summary: clean_summary,
        annotations: article.annotations.iter().map(Into::into).collect(),
        tags: &tags,
        favicon: article.favicon.as_deref(),
        images: &article.images,
    };

    let yaml = noyalib::to_string(&fm).expect("frontmatter serialization is infallible");
    format!("---\n{}\n---\n\n{}\n", yaml.trim_end(), article.content)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Finds the current path of an entry by id, or `None` if the file does not exist.
pub(crate) async fn find_entry_path(library_dir: &Path, id: &str) -> Option<PathBuf> {
    if !is_safe_id(id) {
        return None;
    }

    let path = crate::library::entry_path(library_dir, id);

    tokio::fs::try_exists(&path)
        .await
        .unwrap_or(false)
        .then_some(path)
}

/// Builds the [`Article`] for a freshly-written entry (empty body, no annotations) from its parts,
/// so the producer hands the domain object to the queue and the consumer never re-reads the file.
#[allow(clippy::too_many_arguments)]
fn entry_article(
    id: String,
    normalized_url: String,
    title: &str,
    created_at: String,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    tags: &[String],
) -> Article {
    Article {
        meta: Entry {
            id,
            url: normalized_url,
            title: title.to_string(),
            created_at,
            published_at: published_at.map(str::to_string),
            updated_at: updated_at.map(str::to_string),
            description: summary.map(str::to_string),
            is_unread: crate::tags::is_unread(tags),
            is_bookmarked: crate::tags::is_bookmarked(tags),
            color: None,
        },
        content: String::new(),
        annotations: vec![],
        favicon: None,
        images: vec![],
    }
}

/// Writes an Entry to `entries/{hash12}.md` with the `unread` tag, returning its [`Article`].
///
/// WARNING:
///     If the caller wishes to preserve preexisting file, it is the caller's responsibility to
///     check its existence before calling this function.
pub(crate) async fn write_entry(
    library_dir: &Path,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    _token: WriteToken,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;
    let hash12 = url_hash12(&normalized);
    let path = crate::library::entry_path(library_dir, &hash12);
    let created_at = Utc::now().to_rfc3339();
    let tags = vec![crate::tags::UNREAD.to_string()];

    let article = entry_article(
        hash12,
        normalized,
        title,
        created_at,
        published_at,
        updated_at,
        summary,
        &tags,
    );

    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    crate::fs::write_atomic_async(path.clone(), render_entry(&article).into_bytes()).await?;

    Ok(article)
}

/// Writes a new Entry to `entries/{hash12}.md` without the `unread` tag (already read).
/// If an Entry already exists at that path it is left untouched. Returns the [`Article`] for the
/// entry so the caller can queue an embed without re-reading the file.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn write_read_entry(
    library_dir: &Path,
    url: &str,
    title: &str,
    created_at: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    tags: &[String],
    _token: WriteToken,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;
    let hash12 = url_hash12(&normalized);
    let path = crate::library::entry_path(library_dir, &hash12);

    let article = entry_article(
        hash12,
        normalized,
        title,
        created_at.to_string(),
        published_at,
        updated_at,
        summary,
        tags,
    );

    if !tokio::fs::try_exists(&path).await? {
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        crate::fs::write_atomic_async(path.clone(), render_entry(&article).into_bytes()).await?;
    }

    Ok(article)
}

/// Reads an entry by id and returns its metadata. Does not modify the file.
pub async fn read_entry(library_dir: &Path, id: &str) -> Result<Entry, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = crate::fs::read_md_capped(&path).await?;
    let fm = parse_frontmatter(&raw)?;
    Ok(md_to_entry(Markdown {
        id: id.to_string(),
        fm,
        body: String::new(),
    }))
}

/// Returns an entry's frontmatter tags — a persistence detail, not on the domain `Entry`.
pub async fn read_entry_tags(library_dir: &Path, id: &str) -> Result<Vec<String>, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = crate::fs::read_md_capped(&path).await?;
    let fm = parse_frontmatter(&raw)?;
    Ok(fm.tags)
}

/// Replaces the tags on an entry's frontmatter and writes the file back.
/// The file watcher picks up the write and queues a priority LanceDB update.
pub(crate) async fn set_entry_tags(
    library_dir: &Path,
    id: &str,
    tags: Vec<String>,
    _token: WriteToken,
) -> Result<Entry, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = crate::fs::read_md_capped(&path).await?;
    let (fm, body) = parse_document(&raw)?;
    // Build the updated Article (the message) once, encode it, and return its meta. State comes from
    // the new `tags`; annotations/body/favicon are preserved from the parsed file.
    let article = Article {
        meta: Entry {
            id: id.to_string(),
            url: fm.source,
            title: fm.title,
            created_at: fm.created_at,
            published_at: fm.published_at,
            updated_at: fm.updated_at,
            description: fm.summary,
            is_unread: crate::tags::is_unread(&tags),
            is_bookmarked: crate::tags::is_bookmarked(&tags),
            color: None,
        },
        content: body,
        annotations: fm.annotations,
        favicon: fm.favicon,
        images: fm.images,
    };
    crate::fs::write_atomic_async(path.clone(), render_entry(&article).into_bytes()).await?;
    Ok(article.meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_entry_escapes_frontmatter_injection_in_title() {
        // A title crafted to break out of the YAML string and inject a `bookmark`
        // tag into the frontmatter. The serializer must escape it so it survives
        // as data, not structure — otherwise a hostile title could flip state.
        let hostile = "safe\n---\ntags:\n- bookmark";
        let article = Article {
            meta: Entry {
                id: "abc123".into(),
                url: "https://example.com/x".into(),
                title: hostile.into(),
                created_at: "2026-01-01".into(),
                published_at: None,
                updated_at: None,
                description: None,
                is_unread: false,
                is_bookmarked: false,
                color: None,
            },
            content: "body".into(),
            annotations: vec![],
            favicon: None,
            images: vec![],
        };

        let rendered = render_entry(&article);
        let (fm, _body) = crate::md::parse_document(&rendered).expect("re-parses");

        assert_eq!(fm.title, hostile, "title must round-trip as data, not inject");
        assert!(
            !fm.tags.iter().any(|t| t == "bookmark"),
            "injected `bookmark` tag must not appear: {:?}",
            fm.tags
        );
    }
}
