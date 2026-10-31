use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use crate::{
    library_writer::WriteToken,
    md::parse_frontmatter,
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

/// Writes an Entry to `entries/{hash12}.md` with `is_unread: true`, returning its [`Article`].
///
/// Existing entries are left untouched; new entries use the shared field-patch upsert.
pub(crate) async fn write_entry(
    library_dir: &Path,
    url: &str,
    title: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    _token: WriteToken,
) -> Result<Article, EntryError> {
    write_initial_entry(
        library_dir,
        url,
        title,
        &Utc::now().to_rfc3339(),
        published_at,
        updated_at,
        summary,
        &[],
        true,
        _token,
    )
    .await
}

/// Writes a new Entry to `entries/{hash12}.md` with the supplied initial tags.
/// If an Entry already exists at that path it is left untouched. Returns the [`Article`] for the
/// saved entry so the caller can queue an embed.
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
    write_initial_entry(
        library_dir,
        url,
        title,
        created_at,
        published_at,
        updated_at,
        summary,
        tags,
        false,
        _token,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn write_initial_entry(
    library_dir: &Path,
    url: &str,
    title: &str,
    created_at: &str,
    published_at: Option<&str>,
    updated_at: Option<&str>,
    summary: Option<&str>,
    tags: &[String],
    is_unread: bool,
    _token: WriteToken,
) -> Result<Article, EntryError> {
    let normalized = normalize_url(url)?;
    let hash12 = url_hash12(&normalized);
    let path = crate::library::entry_path(library_dir, &hash12);

    // Feed discovery and bookmark import leave already-saved entries untouched.
    if !tokio::fs::try_exists(&path).await? {
        let mut updates = vec![
            ("source", serde_json::json!(normalized)),
            ("title", serde_json::json!(title)),
            ("aliases", serde_json::json!([title])),
            ("created", serde_json::json!(created_at)),
            ("tags", serde_json::json!(tags)),
            ("is_unread", serde_json::json!(is_unread)),
        ];
        for (key, value) in [
            ("published", published_at),
            ("updated", updated_at),
            ("summary", summary),
        ] {
            if let Some(value) = value {
                updates.push((key, serde_json::json!(value)));
            }
        }
        super::documents::upsert(&path, &updates, None, _token).await?;
    }
    super::articles::read_article(library_dir, &hash12).await
}

/// Reads an entry by id and returns its metadata. Does not modify the file.
pub async fn read_entry(library_dir: &Path, id: &str) -> Result<Entry, EntryError> {
    let mut entry = read_entry_metadata(library_dir, id).await?;
    entry.is_pinned = crate::pins::Pins::read(library_dir)
        .await
        .map_err(std::io::Error::other)?
        .contains(&crate::pins::PinTarget::Entry { id: id.into() });
    Ok(entry)
}

pub(crate) async fn read_entry_metadata(library_dir: &Path, id: &str) -> Result<Entry, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let raw = crate::fs::read_md_capped(&path).await?;
    let fm = parse_frontmatter(&raw)?;
    let entry = md_to_entry(Markdown {
        id: id.to_string(),
        fm,
        body: String::new(),
    });
    Ok(entry)
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
/// The caller queues the corresponding LanceDB state update.
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
    // Derive the return value from the new tags, but patch only the tags field
    // on disk so all other frontmatter and the exact body survive.
    let article = Article {
        meta: Entry {
            id: id.to_string(),
            url: fm.source,
            title: fm.title,
            created_at: fm.created_at,
            published_at: fm.published_at,
            updated_at: fm.updated_at,
            description: fm.summary,
            is_unread: fm.is_unread.unwrap_or(false),
            is_bookmarked: crate::tags::is_bookmarked(&tags),
            is_pinned: crate::pins::Pins::read(library_dir)
                .await
                .map_err(std::io::Error::other)?
                .contains(&crate::pins::PinTarget::Entry { id: id.into() }),
            color: None,
        },
        content: body,
        annotations: fm.annotations,
        favicon: fm.favicon,
    };
    super::documents::upsert(&path, &[("tags", serde_json::json!(tags))], None, _token).await?;
    Ok(article.meta)
}

/// Reading state is independent of tags. Call inside the entry lane.
pub(crate) async fn set_entry_unread(
    library_dir: &Path,
    id: &str,
    is_unread: bool,
    token: WriteToken,
) -> Result<Entry, EntryError> {
    let path = find_entry_path(library_dir, id)
        .await
        .ok_or(EntryError::NotFound)?;
    let rendered = super::documents::upsert(
        &path,
        &[("is_unread", serde_json::json!(is_unread))],
        None,
        token,
    )
    .await?;
    let fm = parse_frontmatter(&rendered)?;
    let mut entry = md_to_entry(Markdown {
        id: id.to_owned(),
        fm,
        body: String::new(),
    });
    entry.is_pinned = crate::pins::Pins::read(library_dir)
        .await
        .map_err(std::io::Error::other)?
        .contains(&crate::pins::PinTarget::Entry { id: id.into() });
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reading_state_writes_preserve_metadata_and_exact_body() {
        let dir = tempfile::tempdir().unwrap();
        let path = crate::library::entry_path(dir.path(), "saved");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "---\nsource: https://example.org/a\ntags: [research, bookmark]\ncreated: yesterday\ncustom: {nested: [one, two]}\nannotations: []\n---\n\nBody\n\n";
        std::fs::write(&path, original).unwrap();
        for unread in [true, false, true, false] {
            let entry = set_entry_unread(dir.path(), "saved", unread, WriteToken::for_test())
                .await
                .unwrap();
            assert_eq!(entry.is_unread, unread);
            assert!(entry.is_bookmarked);
            let saved = std::fs::read_to_string(&path).unwrap();
            let mut expected = crate::md::test_frontmatter_values(original);
            if unread {
                expected.insert("is_unread".into(), noyalib::Value::Bool(true));
            }
            assert_eq!(crate::md::test_frontmatter_values(&saved), expected);
            assert!(saved.ends_with("---\n\nBody\n\n"));
        }
        assert!(
            set_entry_unread(dir.path(), "missing", true, WriteToken::for_test())
                .await
                .is_err()
        );
        assert!(!crate::library::entry_path(dir.path(), "missing").exists());
    }

    #[tokio::test]
    async fn new_feed_entries_are_unread_and_bookmark_imports_are_read() {
        let dir = tempfile::tempdir().unwrap();
        let fetched = write_entry(
            dir.path(),
            "https://example.org/feed",
            "Feed",
            None,
            None,
            None,
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert!(fetched.meta.is_unread);
        assert!(
            read_entry_tags(dir.path(), &fetched.meta.id)
                .await
                .unwrap()
                .is_empty()
        );
        let imported = write_read_entry(
            dir.path(),
            "https://example.org/import",
            "Import",
            "then",
            None,
            None,
            None,
            &["bookmark".into()],
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert!(!imported.meta.is_unread);
        let raw =
            std::fs::read_to_string(crate::library::entry_path(dir.path(), &imported.meta.id))
                .unwrap();
        assert!(!crate::md::test_frontmatter_values(&raw).contains_key("is_unread"));
        assert!(imported.meta.is_bookmarked);
    }

    #[tokio::test]
    async fn creation_workflows_preserve_existing_document_and_return_saved_data() {
        let dir = tempfile::tempdir().unwrap();
        let url = "https://example.com/a";
        let id = url_hash12(url);
        let path = crate::library::entry_path(dir.path(), &id);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = "---\nsource: https://example.com/a\ntitle: Saved\nsummary: Saved summary\ntags: [research, bookmark]\ncustom: retained\n---\n\nSaved body\n";
        std::fs::write(&path, original).unwrap();
        let result = write_entry(
            dir.path(),
            url,
            "Replacement",
            None,
            None,
            None,
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(result.meta.description.as_deref(), Some("Saved summary"));
        assert_eq!(result.content, "Saved body");
        let result = write_read_entry(
            dir.path(),
            url,
            "Replacement",
            "now",
            None,
            None,
            None,
            &[],
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(result.meta.description.as_deref(), Some("Saved summary"));
        assert!(result.meta.is_bookmarked);
    }

    #[tokio::test]
    async fn bookmark_tag_preserves_existing_feed_tags() {
        let dir = tempfile::tempdir().unwrap();
        let article = write_entry(
            dir.path(),
            "https://example.com/a",
            "Title",
            None,
            None,
            None,
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        set_entry_tags(
            dir.path(),
            &article.meta.id,
            vec!["unread".into(), "bookmark".into()],
            crate::library_writer::WriteToken::for_test(),
        )
        .await
        .unwrap();
        assert_eq!(
            read_entry_tags(dir.path(), &article.meta.id).await.unwrap(),
            vec!["unread", "bookmark"]
        );
    }

    #[tokio::test]
    async fn entry_write_escapes_frontmatter_injection_in_title() {
        let dir = tempfile::tempdir().unwrap();
        let hostile = "safe\n---\ntags:\n- bookmark";
        let article = write_entry(
            dir.path(),
            "https://example.com/x",
            hostile,
            None,
            None,
            None,
            WriteToken::for_test(),
        )
        .await
        .unwrap();
        let saved = read_entry(dir.path(), &article.meta.id).await.unwrap();
        assert_eq!(saved.title, hostile);
        assert!(!saved.is_bookmarked);
    }
}
