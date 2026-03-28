use sha2::{Digest, Sha256};

/// Parses and normalises a URL: lowercases scheme + host (via the `url` crate)
/// and strips any trailing slash.
pub fn normalize_url(raw: &str) -> Result<String, url::ParseError> {
    let parsed = url::Url::parse(raw)?;
    let mut s = parsed.to_string();
    if s.ends_with('/') {
        s.pop();
    }
    Ok(s)
}

/// Returns the first 12 hex characters of the SHA-256 of `normalized`.
/// Used as a compact, filesystem-safe identifier for both feeds and documents.
pub fn url_hash12(normalized: &str) -> String {
    let hash = Sha256::digest(normalized.as_bytes());
    hex::encode(hash)[..12].to_string()
}

/// Returns the stable `id` for a feed entry.
/// Uses the Atom `<entry><id>` value when non-empty; falls back to
/// `url_hash12(normalize_url(link_href))` when the entry id is absent or blank
/// (malformed Atom).
pub fn feed_entry_id(atom_id: &str, link_href: &str) -> String {
    if !atom_id.is_empty() {
        return atom_id.to_string();
    }
    normalize_url(link_href)
        .map(|n| url_hash12(&n))
        .unwrap_or_else(|_| url_hash12(link_href))
}

pub struct Frontmatter {
    pub source: String,
    pub title: String,
    pub created_at: String,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub summary: Option<String>,
}

impl Frontmatter {
    /// The text used for embedding: summary when present, otherwise the full body.
    pub fn embed_text<'a>(&'a self, body: &'a str) -> &'a str {
        self.summary.as_deref().unwrap_or(body)
    }

    /// SHA-256 checksum of the embeddable content (title + embed_text).
    pub fn content_checksum(&self, body: &str) -> String {
        use sha2::Digest;
        let mut h = sha2::Sha256::new();
        h.update(self.title.as_bytes());
        h.update(b"\n\n");
        h.update(self.embed_text(body).as_bytes());
        hex::encode(h.finalize())
    }

    /// Builds an `EmbeddingEntry` from this frontmatter given the item's id and vector.
    pub fn to_embedding_entry(
        &self,
        id: String,
        body: &str,
        vector: Vec<f32>,
    ) -> crate::embeddings::db::EmbeddingEntry {
        crate::embeddings::db::EmbeddingEntry {
            id,
            source: self.source.clone(),
            title: self.title.clone(),
            created_at: self.created_at.clone(),
            published_at: self.published_at.clone().unwrap_or_default(),
            updated_at: self.updated_at.clone().unwrap_or_default(),
            checksum: self.content_checksum(body),
            vector,
        }
    }
}

/// Returns the canonical ID for any library item: the file stem.
pub fn item_id(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// Parses only the header fields from a `---`-delimited document.
/// Returns `None` if the format is invalid.
pub fn parse_frontmatter(raw: &str) -> Option<Frontmatter> {
    Some(split_document(raw)?.0)
}

/// Parses both the header fields and the markdown body from a `---`-delimited document.
/// Returns `None` if the format is invalid.
pub fn parse_document(raw: &str) -> Option<(Frontmatter, String)> {
    split_document(raw)
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
}

fn split_document(raw: &str) -> Option<(Frontmatter, String)> {
    let raw = raw.trim_start();
    let rest = raw.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    let yaml = &rest[..end];
    let body = rest[end + 5..].trim_start_matches('\n').to_string();

    let fm: FrontmatterRaw = serde_yaml::from_str(yaml).ok()?;

    Some((
        Frontmatter {
            source: fm.source,
            title: fm.title,
            created_at: fm.created,
            published_at: fm.published,
            updated_at: fm.updated,
            summary: fm.summary,
        },
        body,
    ))
}
