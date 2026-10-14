//! Entry image assets: rewriting an entry body's remote image references to
//! local files, and reading those files back as `data:` URIs at render.
//!
//! The body handed to `save_article` carries remote image URLs (`![](https://…)`);
//! `processEntry` supplies a `url → data:` map for the images the loader actually
//! fetched. [`extract_images`] prepares local URLs for supplied bytes. After
//! writing the files, [`Extraction::persisted_body`] keeps local URLs only for
//! successful writes; other images retain their original remote URLs. No
//! original-URL map is stored in frontmatter.
//!
//! The scan is a targeted, fail-safe span replacement, never a markdown
//! round-trip: anything it cannot parse with certainty is left byte-for-byte
//! untouched, so a parse it doesn't understand degrades rather than corrupting
//! an irreplaceable body.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use base64::Engine;
use sha2::{Digest, Sha256};

use crate::library_writer::WriteToken;

/// Per-save cap on the total decoded image bytes extracted from one body,
/// mirroring the loader's own `__XAGA_IMAGE_TOTAL_BYTES__` budget. Images past
/// this are dropped (alt text kept), never written.
pub(crate) const MAX_ENTRY_IMAGE_BYTES: usize = 24 * 1024 * 1024;

/// Per-file read cap when reconstructing a `data:` URI at read time, mirroring
/// `fs::MAX_MD_BYTES` in spirit. A file over this reads as `None`.
pub(crate) const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

/// Cap on the total decoded bytes one `read_entry_images` call will return, so a
/// pathological entry cannot balloon the IPC payload. Refs past the cap read as
/// `None`.
pub(crate) const MAX_READ_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

/// An image decoded out of the caller-supplied `url → data:` map, ready to be
/// written to disk. `file_name` is `<url-hash>.<ext>` (the extension from the
/// fetched media type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtractedImage {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// The result of rewriting a body:
/// - `body` — candidate Markdown with captured images assigned local URLs.
///   Call `persisted_body` after writing files before saving this body.
/// - `images` — the files to write (only for URLs whose bytes were supplied),
///   deduplicated by `file_name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Extraction {
    body: String,
    pub images: Vec<ExtractedImage>,
    rewrites: Vec<(std::ops::Range<usize>, String, String)>,
}

impl Extraction {
    /// Commit local URLs only for files whose atomic write succeeded.
    pub(crate) fn persisted_body(mut self, landed: &[String]) -> String {
        // Reverse order keeps earlier byte offsets valid as replacements change length.
        for (span, original, file_name) in self.rewrites.into_iter().rev() {
            if !landed.contains(&file_name) {
                self.body.replace_range(span, &original);
            }
        }
        self.body
    }
}

/// Maps an image media type to the file extension used on disk. The extension
/// comes from the media type carried in the `data:` URI — which is the response
/// `Content-Type` the loader gated on — never from the source URL. Returns
/// `None` for anything not on the allowlist; those bytes are not persisted.
pub(crate) fn ext_for_media_type(media_type: &str) -> Option<&'static str> {
    match media_type {
        "image/jpeg" => Some("jpg"),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some("ico"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/avif" => Some("avif"),
        "image/svg+xml" => Some("svg"),
        _ => None,
    }
}

/// Inverse of [`ext_for_media_type`], used when reconstructing a `data:` URI at
/// read time from a stored file's extension.
pub(crate) fn media_type_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "jpg" => Some("image/jpeg"),
        "ico" => Some("image/x-icon"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

/// First 12 hex chars of the SHA-256 of the (raw, unescaped) image URL — the
/// same compact, filesystem-safe idiom as `util::url_hash12`, but over the image
/// URL rather than a normalised entry URL. The URL is the identity: the same URL
/// always names the same file, and the reader recomputes this from the URL to
/// find the file. Raw (not normalised) because image URLs carry meaningful query
/// params (`?quality=75`) that a normaliser would strip.
pub(crate) fn image_url_hash(url: &str) -> String {
    let digest = Sha256::digest(url.as_bytes());
    hex::encode(digest)[..12].to_string()
}

/// Decodes a `data:<media-type>;base64,<payload>` into `(<hash>.<ext>, bytes)`,
/// or `None` if it is not a base64 image of an allowlisted media type, fails to
/// decode, or exceeds the per-image cap. The extension comes from the media type.
fn decode_data_uri(
    hash: &str,
    data_uri: &str,
    engine: &base64::engine::GeneralPurpose,
) -> Option<ExtractedImage> {
    let rest = data_uri.strip_prefix("data:")?;
    let (meta, payload) = rest.split_once(',')?;
    let media_type = meta.strip_suffix(";base64")?;
    let ext = ext_for_media_type(media_type)?;
    let bytes = engine.decode(payload.trim()).ok()?;
    if bytes.len() > MAX_ENTRY_IMAGE_BYTES {
        return None;
    }
    Some(ExtractedImage {
        file_name: format!("{hash}.{ext}"),
        bytes,
    })
}

/// Whether a markdown image's destination is a remote URL we localise, or
/// anything else (an already-local `../images/…` ref, a relative path) we leave.
fn is_remote(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Removes markdown backslash-escapes before ASCII punctuation, recovering the
/// literal text/URL that remark-stringify escaped (e.g. `?a=1\&b=2` → `?a=1&b=2`,
/// `\[x\]` → `[x]`). Only ASCII-punctuation escapes are markdown-meaningful; a
/// backslash before anything else is preserved.
fn markdown_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_punctuation() {
                    out.push(next);
                    chars.next();
                    continue;
                }
            }
        }
        out.push(c);
    }
    out
}

/// A parsed markdown image reference located in the body.
struct ImageMatch<'a> {
    /// Byte range of the whole `![alt](url)` in the source.
    span: std::ops::Range<usize>,
    /// The alt text exactly as it appears in the source (still markdown-escaped).
    alt_raw: &'a str,
    /// The destination URL exactly as it appears (still markdown-escaped, or
    /// `<>`-unwrapped).
    url_raw: &'a str,
}

/// Finds the next `![alt](url)` image at or after `from`, or `None`. Fail-safe:
/// any candidate whose structure is not unambiguously a complete image (alt not
/// closed on its line, missing `(`, url not closed) is rejected and the search
/// resumes one byte on, so a malformed candidate can never consume trailing
/// body text into a replacement.
fn next_image(markdown: &str, from: usize) -> Option<ImageMatch<'_>> {
    let bytes = markdown.as_bytes();
    let mut i = from;
    while let Some(rel) = markdown[i..].find("![") {
        let bang = i + rel;
        // A `\!` is an escaped bang, not an image. Count preceding backslashes.
        let mut bs = 0usize;
        while bang > bs && bytes[bang - 1 - bs] == b'\\' {
            bs += 1;
        }
        if bs % 2 == 1 {
            i = bang + 2;
            continue;
        }

        // Alt text: from after `![` to the first unescaped `]`, no newline.
        let alt_start = bang + 2;
        let mut j = alt_start;
        let mut alt_end = None;
        while j < bytes.len() {
            match bytes[j] {
                b'\\' => j += 2,
                b'\n' => break,
                b']' => {
                    alt_end = Some(j);
                    break;
                }
                _ => j += 1,
            }
        }
        let Some(alt_end) = alt_end else {
            i = bang + 2;
            continue;
        };
        // Require `(` immediately after `]`.
        if bytes.get(alt_end + 1) != Some(&b'(') {
            i = bang + 2;
            continue;
        }

        let url_open = alt_end + 2;
        let (url_raw, span_end) = if bytes.get(url_open) == Some(&b'<') {
            // `<...>` wrapped url: read to the first unescaped `>`, then `)`.
            let mut k = url_open + 1;
            let mut gt = None;
            while k < bytes.len() {
                match bytes[k] {
                    b'\\' => k += 2,
                    b'\n' => break,
                    b'>' => {
                        gt = Some(k);
                        break;
                    }
                    _ => k += 1,
                }
            }
            let Some(gt) = gt else {
                i = bang + 2;
                continue;
            };
            let tail = &markdown[gt + 1..];
            let Some(close) = tail.find(')') else { i = bang + 2; continue; };
            if tail[..close].contains('\n') { i = bang + 2; continue; }
            (&markdown[url_open + 1..gt], gt + 1 + close + 1)
        } else {
            // Bare url: read to whitespace (optional title follows) or `)`.
            let mut k = url_open;
            let mut url_end = None;
            let mut close = None;
            while k < bytes.len() {
                match bytes[k] {
                    b'\\' => k += 2,
                    b'\n' => break,
                    b' ' | b'\t' if url_end.is_none() => {
                        url_end = Some(k);
                        k += 1;
                    }
                    b')' => {
                        close = Some(k);
                        break;
                    }
                    _ => k += 1,
                }
            }
            let Some(close) = close else {
                i = bang + 2;
                continue;
            };
            let url_end = url_end.unwrap_or(close);
            (&markdown[url_open..url_end], close + 1)
        };

        return Some(ImageMatch {
            span: bang..span_end,
            alt_raw: &markdown[alt_start..alt_end],
            url_raw,
        });
    }
    None
}

/// Prepare local image URLs and decoded files for captured images. Missing or
/// unusable bytes leave the original Markdown image untouched. Finalize with
/// `persisted_body` after writing the files to retain remote URLs on write failure.
pub(crate) fn extract_images(
    id: &str,
    markdown: &str,
    url_data: &std::collections::HashMap<String, String>,
) -> Extraction {
    // No image syntax at all: nothing to do, return the body unchanged. (This
    // is the common no-op path for image-free entries.)
    if !markdown.contains("![") {
        return Extraction {
            body: markdown.to_string(),
            images: Vec::new(),
            rewrites: Vec::new(),
        };
    }

    let engine = base64::engine::general_purpose::STANDARD;
    let mut out = String::with_capacity(markdown.len());
    let mut images: Vec<ExtractedImage> = Vec::new();
    let mut cursor = 0usize;
    let mut rewrites = Vec::new();

    while let Some(m) = next_image(markdown, cursor) {
        out.push_str(&markdown[cursor..m.span.start]);

        if is_remote(m.url_raw) {
            // The body carries the markdown-escaped URL; recover the real URL to
            // hash it and to key `url_data` (whose keys are raw URLs).
            let url = markdown_unescape(m.url_raw);
            let hash = image_url_hash(&url);

            let image = url_data.get(&url)
                .and_then(|data_uri| decode_data_uri(&hash, data_uri, &engine));
            if let Some(image) = image {
                let start = out.len();
                // Replace only the destination: preserve the title, alt text,
                // delimiters and whitespace exactly as supplied.
                let url_start = m.url_raw.as_ptr() as usize - markdown.as_ptr() as usize;
                let url_end = url_start + m.url_raw.len();
                out.push_str(&markdown[m.span.start..url_start]);
                out.push_str(&crate::library::entry_image_ref(id, &hash));
                out.push_str(&markdown[url_end..m.span.end]);
                rewrites.push((start..out.len(), markdown[m.span.clone()].to_string(), image.file_name.clone()));
                if !images.iter().any(|img| img.file_name == image.file_name) {
                    images.push(image);
                }
            } else {
                out.push_str(&markdown[m.span.clone()]);
            }
        } else if m.url_raw.starts_with("data:") {
            // A stray `data:` image (an original page data-URI, not one the loader
            // fetched via a URL — those are restored to their URL before this
            // runs). Drop it to alt text; never leave base64 in the stored body.
            out.push_str(m.alt_raw);
        } else {
            // Already-local or other non-remote reference: leave untouched.
            out.push_str(&markdown[m.span.clone()]);
        }

        cursor = m.span.end;
    }
    out.push_str(&markdown[cursor..]);

    Extraction {
        body: out,
        images,
        rewrites,
    }
}

/// Localises an entry's favicon the same way [`extract_images`] localises a body
/// image: hash the remote URL, and decode the bytes when the loader supplied
/// them. Returns `None` when `url` is not a remote reference — an already-local
/// `../images/…` ref (a re-save) or anything else passes through untouched.
///
/// A `Some((hash, None))` means the URL is ours to rewrite but no bytes arrived
/// (the fetch failed, or the media type is off the allowlist): the caller stores
/// a local path with no file, as with a body image whose bytes are absent.
pub(crate) fn extract_favicon(
    url: &str,
    url_data: &HashMap<String, String>,
) -> Option<(String, Option<ExtractedImage>)> {
    if !is_remote(url) {
        return None;
    }
    let engine = base64::engine::general_purpose::STANDARD;
    let hash = image_url_hash(url);
    let image = url_data
        .get(url)
        .and_then(|data_uri| decode_data_uri(&hash, data_uri, &engine));
    Some((hash, image))
}

// ── Disk I/O ──────────────────────────────────────────────────────────────────
//
// Ordering invariant (see docs/architecture/crash_consistency.md): images are
// written BEFORE the entry `.md`, so a committed `../images/…` reference always
// has durable bytes behind it. A crash between the two leaves at most orphan
// image files, which are benign — invisible to every `.md`/`.atom` walk (the
// extension filter in `fs::for_each_file`), never read, and unable to affect a
// `WalkOutcome`. The next save of this id sweeps them.

/// Writes each extracted image into `<library>/images/<id>/`, atomically. Returns
/// the file names that actually landed. A per-image write failure is logged and
/// skipped, never propagated: the article body is irreplaceable and must be
/// saved even if an image cannot be. The caller uses the returned file names
/// to finalize the body, retaining remote URLs for failed writes.
pub(crate) async fn write_entry_images(
    library_dir: &Path,
    id: &str,
    images: &[ExtractedImage],
    _token: WriteToken,
) -> Vec<String> {
    if images.is_empty() {
        return Vec::new();
    }
    let dir = crate::library::entry_images_dir(library_dir, id);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        log::warn!("could not create image dir for entry {id}: {e}");
        return Vec::new();
    }
    let mut landed = Vec::with_capacity(images.len());
    for img in images {
        let path = crate::library::entry_image_path(library_dir, id, &img.file_name);
        match crate::fs::write_atomic_async(path, img.bytes.clone()).await {
            Ok(()) => landed.push(img.file_name.clone()),
            Err(e) => log::warn!("failed to write image {} for entry {id}: {e}", img.file_name),
        }
    }
    landed
}

/// Deletes every file in the entry's image directory whose name is not in `keep`
/// — the set of file names the just-committed body references. Runs after the
/// `.md` write, inside the same write lane, so no concurrent writer can be
/// mid-update. Best-effort: every error is logged and swallowed, because a
/// failed GC leaves only benign orphans and must never fail the save. `keep`
/// deliberately holds every referenced name (not only the ones that landed this
/// save), so a write that failed this round is not deleted out from under a body
/// that still references it. `keep` holds the referenced url-hashes; a file
/// `<hash>.<ext>` is kept iff its stem is in `keep`. Also sweeps `write_atomic`'s
/// `.tmp` leftovers (no matching stem).
pub(crate) async fn gc_entry_images(
    library_dir: &Path,
    id: &str,
    keep: &HashSet<String>,
    _token: WriteToken,
) {
    let dir = crate::library::entry_images_dir(library_dir, id);
    let mut rd = match tokio::fs::read_dir(&dir).await {
        Ok(rd) => rd,
        // No directory means nothing to collect.
        Err(_) => return,
    };
    loop {
        match rd.next_entry().await {
            Ok(Some(entry)) => {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                // The ref is ext-less, so keep by file stem (`<hash>` of `<hash>.<ext>`).
                let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name.as_ref());
                if !keep.contains(stem) {
                    let path = entry.path();
                    if let Err(e) = tokio::fs::remove_file(&path).await {
                        log::warn!("gc could not remove {}: {e}", path.display());
                    }
                }
            }
            Ok(None) => break,
            Err(e) => {
                log::warn!("gc read_dir error for entry {id}: {e}");
                break;
            }
        }
    }
    // Drop the read handle before removing the directory (it holds the dir open
    // on some platforms), then remove it if it is now empty — so an entry that
    // no longer stores any images leaves no empty directory behind. `remove_dir`
    // only succeeds on an empty directory, so one with surviving files is left
    // intact.
    drop(rd);
    let _ = tokio::fs::remove_dir(&dir).await;
}

/// The set of image hashes an entry body references (the `<hash>` in each
/// `../images/<id>/<hash>`). Used by article saves and the CLI sweep to decide which stored files
/// are still live — a file `<hash>.<ext>` is live iff its `<hash>` is in this
/// set. Scans for the reference prefix and reads each hash up to the first
/// delimiter (`)`, `]`, quote, or whitespace).
pub fn referenced_image_hashes(id: &str, body: &str) -> HashSet<String> {
    let prefix = crate::library::entry_image_ref(id, "");
    let mut out = HashSet::new();
    let mut rest = body;
    while let Some(pos) = rest.find(&prefix) {
        let after = &rest[pos + prefix.len()..];
        let end = after
            .find(|c: char| {
                c == ')' || c == ']' || c == '"' || c == '\'' || c.is_whitespace()
            })
            .unwrap_or(after.len());
        if end > 0 {
            out.insert(after[..end].to_string());
        }
        rest = &after[end..];
    }
    out
}

/// The stored image files of one entry, mapped `hash → (path, ext)` — the entry
/// directory read once so a batch of refs resolves without one `read_dir` each.
async fn list_entry_image_files(library_dir: &Path, id: &str) -> HashMap<String, (std::path::PathBuf, String)> {
    let dir = crate::library::entry_images_dir(library_dir, id);
    let mut out = HashMap::new();
    let Ok(mut rd) = tokio::fs::read_dir(&dir).await else {
        return out;
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some((stem, ext)) = name.rsplit_once('.') {
            out.insert(stem.to_string(), (entry.path(), ext.to_string()));
        }
    }
    out
}

/// Reads bytes back for each body reference, returning a complete `data:` URI or
/// `None`, index-aligned with `refs`. `None` covers every non-fault case: a ref
/// that is not a well-formed `../images/<id>/<hash>` for THIS entry, a missing
/// file (including missing images in older articles), an unknown extension,
/// an oversize file, or the response byte budget being exhausted. A bad
/// reference is data, never an error — the reader leaves the ref in place, and
/// in the sandboxed blob iframe a relative ref fetches nothing.
pub async fn read_entry_images(library_dir: &Path, id: &str, refs: &[String]) -> Vec<Option<String>> {
    let engine = base64::engine::general_purpose::STANDARD;
    let files = list_entry_image_files(library_dir, id).await;
    let mut out = Vec::with_capacity(refs.len());
    let mut total: usize = 0;
    for reference in refs {
        out.push(resolve_ref(id, reference, &files, &engine, &mut total).await);
    }
    out
}

/// Reuse already-persisted bytes for another reference to the same remote URL.
/// This performs only local reads; the URL chooses a hash, never a filesystem path.
pub async fn read_entry_image_url(library_dir: &Path, id: &str, url: &str) -> Option<String> {
    if !super::entries::is_safe_id(id) || !is_remote(url) { return None; }
    let reference = crate::library::entry_image_ref(id, &image_url_hash(url));
    read_entry_images(library_dir, id, &[reference]).await.pop().flatten()
}

async fn resolve_ref(
    id: &str,
    reference: &str,
    files: &HashMap<String, (std::path::PathBuf, String)>,
    engine: &base64::engine::GeneralPurpose,
    total: &mut usize,
) -> Option<String> {
    let (ref_id, hash) = crate::library::parse_entry_image_ref(reference)?;
    // An entry may only reach into its own image directory.
    if ref_id != id {
        return None;
    }
    // Resolve the ext-less hash to the stored `<hash>.<ext>` file, if present.
    let (path, ext) = files.get(hash)?;
    let media_type = media_type_for_ext(ext)?;

    let bytes = read_capped_bytes(path, MAX_IMAGE_BYTES).await?;

    *total = total.checked_add(bytes.len())?;
    if *total > MAX_READ_RESPONSE_BYTES {
        return None;
    }

    Some(format!("data:{media_type};base64,{}", engine.encode(&bytes)))
}

/// Reads a file into a `Vec<u8>`, rejecting one over `max`. Enforced on the read
/// (reads at most `max + 1`), not via a `metadata` stat — the same TOCTOU-safe
/// shape as `fs::read_md_capped`, but for bytes rather than UTF-8.
async fn read_capped_bytes(path: &Path, max: u64) -> Option<Vec<u8>> {
    use tokio::io::AsyncReadExt;
    let file = tokio::fs::File::open(path).await.ok()?;
    let mut buf = Vec::new();
    file.take(max + 1).read_to_end(&mut buf).await.ok()?;
    if buf.len() as u64 > max {
        return None;
    }
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_DATA: &str =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC";

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(u, d)| (u.to_string(), d.to_string())).collect()
    }

    #[test]
    fn noop_when_no_image_syntax() {
        let md = "# Title\n\nJust prose with a [link](https://x.test/) and no images.\n";
        let out = extract_images("abc123", md, &HashMap::new());
        assert_eq!(out.body, md);
        assert!(out.images.is_empty());
    }

    #[test]
    fn remote_image_with_bytes_becomes_file_and_local_ref() {
        let url = "https://x.test/p.png";
        let md = format!("before ![a cat]({url}) after");
        let out = extract_images("entry1", &md, &map(&[(url, PNG_DATA)]));

        let hash = image_url_hash(url);
        assert_eq!(out.images.len(), 1);
        assert_eq!(out.images[0].file_name, format!("{hash}.png"));
        assert_eq!(
            out.body,
            format!("before ![a cat](../images/entry1/{hash}) after")
        );
        // No remote URL and no base64 survive in the body.
        assert!(!out.body.contains("http"));
        assert!(!out.body.contains("base64"));
    }

    #[test]
    fn remote_image_without_bytes_keeps_original_url() {
        // Missing capture bytes must not invent a local file.
        let url = "https://x.test/q.png";
        let md = format!("![b]({url})");
        let out = extract_images("entry1", &md, &HashMap::new());

        assert!(out.images.is_empty());
        assert_eq!(out.body, md);
    }

    #[test]
    fn failed_write_restores_exact_markdown_including_escaped_url_and_title() {
        let url = "https://x.test/p.png?a=1&b=2";
        let md = r#"![alt](https://x.test/p.png?a=1\&b=2 "title")"#;
        let out = extract_images("entry1", md, &map(&[(url, PNG_DATA)]));
        assert_eq!(out.images.len(), 1);
        assert_eq!(out.persisted_body(&[]), md);
    }

    #[test]
    fn successful_localization_preserves_title_and_alt() {
        let url = "https://x.test/p.png";
        let md = r#"![description](<https://x.test/p.png> "Additional content")"#;
        let out = extract_images("entry1", md, &map(&[(url, PNG_DATA)]));
        let file = out.images[0].file_name.clone();
        assert_eq!(out.persisted_body(&[file]), format!(
            r#"![description](<../images/entry1/{}> "Additional content")"#, image_url_hash(url)
        ));
    }

    #[test]
    fn invalid_base64_retains_remote_image() {
        let url = "https://x.test/p.png";
        let md = format!("![alt]({url})");
        let out = extract_images("entry1", &md, &map(&[(url, "data:image/png;base64,!!!")]));
        assert!(out.images.is_empty());
        assert_eq!(out.persisted_body(&[]), md);
    }

    #[test]
    fn leaves_local_and_non_remote_refs_untouched() {
        // Already-local refs and prose round-trip unchanged (idempotent re-save).
        let md = "text ![local](../images/entry1/4f2a91c3e0b7) more text\n";
        let out = extract_images("entry1", md, &HashMap::new());
        assert_eq!(out.body, md);
        assert!(out.images.is_empty());
    }

    #[test]
    fn unescapes_url_for_hash_and_map() {
        // remark escapes `&` as `\&` in the body; the real URL keys `url_data`
        // so extraction must unescape before hashing.
        let raw = "https://tracker.test/p.gif?u=1&v=2";
        let md = "![pixel](https://tracker.test/p.gif?u=1\\&v=2)";
        let out = extract_images("e", md, &map(&[(raw, PNG_DATA)]));

        let hash = image_url_hash(raw);
        assert_eq!(out.images.len(), 1);
        assert_eq!(out.body, format!("![pixel](../images/e/{hash})"));
    }

    #[test]
    fn dedupes_same_url_to_one_file_two_refs() {
        let url = "https://x.test/p.png";
        let md = format!("![one]({url}) and ![two]({url})");
        let out = extract_images("e", &md, &map(&[(url, PNG_DATA)]));

        let hash = image_url_hash(url);
        assert_eq!(out.images.len(), 1);
        assert_eq!(
            out.body,
            format!("![one](../images/e/{hash}) and ![two](../images/e/{hash})")
        );
    }

    #[test]
    fn keeps_svg() {
        let url = "https://x.test/d.svg";
        let svg = "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=";
        let out = extract_images("e", &format!("![d]({url})"), &map(&[(url, svg)]));
        assert_eq!(out.images.len(), 1);
        assert!(out.images[0].file_name.ends_with(".svg"));
    }

    #[test]
    fn bad_media_type_keeps_original_url() {
        let url = "https://x.test/doc.pdf";
        let bad = "data:application/pdf;base64,JVBERi0=";
        let out = extract_images("e", &format!("![x]({url})"), &map(&[(url, bad)]));
        assert!(out.images.is_empty());
        assert_eq!(out.body, format!("![x]({url})"));
        assert!(!out.body.contains("base64"));
    }

    #[test]
    fn drops_stray_data_image_without_leaking_base64() {
        // An original page data-URI (no URL to hash) is dropped to alt text.
        let md = "![icon](data:image/png;base64,AAAA)";
        let out = extract_images("e", md, &HashMap::new());
        assert_eq!(out.body, "icon");
        assert!(out.images.is_empty());
        assert!(!out.body.contains("base64"));
    }

    #[test]
    fn ignores_escaped_bang_and_plain_links() {
        // `\!` is not an image; `[text](url)` is a link, not an image.
        let md = "\\![notimg](https://x.test/a.png) and [text](https://x.test/b)";
        let out = extract_images("e", md, &HashMap::new());
        assert_eq!(out.body, md);
        assert!(out.images.is_empty());
    }

    // ── Favicon ─────────────────────────────────────────────────────────────

    #[test]
    fn favicon_with_bytes_yields_hash_and_file() {
        let url = "https://x.test/favicon.ico";
        let (hash, image) = extract_favicon(url, &map(&[(url, PNG_DATA)])).expect("remote");
        assert_eq!(hash, image_url_hash(url));
        assert_eq!(image.expect("bytes").file_name, format!("{hash}.png"));
    }

    #[test]
    fn favicon_without_bytes_yields_hash_only() {
        // A failed fetch: the caller still records the URL and writes a broken
        // local ref, so a reload can heal it.
        let url = "https://x.test/favicon.ico";
        let (hash, image) = extract_favicon(url, &HashMap::new()).expect("remote");
        assert_eq!(hash, image_url_hash(url));
        assert!(image.is_none());
    }

    #[test]
    fn favicon_off_allowlist_media_type_yields_no_file() {
        let url = "https://x.test/favicon.ico";
        let bad = "data:application/pdf;base64,JVBERi0=";
        let (_, image) = extract_favicon(url, &map(&[(url, bad)])).expect("remote");
        assert!(image.is_none());
    }

    #[test]
    fn favicon_that_is_already_local_is_left_alone() {
        // The re-save path: a second save must not re-hash an existing ref.
        assert!(extract_favicon("../images/e1/4f2a91c3e0b7", &HashMap::new()).is_none());
        assert!(extract_favicon("data:image/png;base64,AAAA", &HashMap::new()).is_none());
    }

    #[test]
    fn url_hash_is_stable_and_distinct() {
        assert_eq!(image_url_hash("https://x/a"), image_url_hash("https://x/a"));
        assert_ne!(image_url_hash("https://x/a"), image_url_hash("https://x/b"));
        assert_eq!(image_url_hash("https://x/a").len(), 12);
    }

    // ── I/O round-trip ──────────────────────────────────────────────────────

    fn token() -> WriteToken {
        WriteToken::for_test()
    }

    #[tokio::test]
    async fn write_then_read_round_trips_via_ext_less_ref() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();

        let url = "https://x.test/p.png";
        let ex = extract_images("e1", &format!("![c]({url})"), &map(&[(url, PNG_DATA)]));
        let landed = write_entry_images(lib, "e1", &ex.images, token()).await;
        assert_eq!(landed.len(), 1);

        // The stored body ref is ext-less; read resolves it to <hash>.<ext>.
        let hash = image_url_hash(url);
        let reference = crate::library::entry_image_ref("e1", &hash);
        let read = read_entry_images(lib, "e1", &[reference]).await;
        assert_eq!(read.len(), 1);
        assert!(read[0].as_ref().unwrap().starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn read_refuses_traversal_and_foreign_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();
        let url = "https://x.test/p.png";
        let ex = extract_images("victim", &format!("![x]({url})"), &map(&[(url, PNG_DATA)]));
        write_entry_images(lib, "victim", &ex.images, token()).await;
        let hash = image_url_hash(url);

        let bad = vec![
            // Another entry reaching into victim's dir.
            crate::library::entry_image_ref("victim", &hash),
            "../images/victim/../../entries/x.md".to_string(),
            "https://evil.test/x.png".to_string(),
        ];
        let read = read_entry_images(lib, "attacker", &bad).await;
        assert_eq!(read, vec![None, None, None]);
    }

    #[tokio::test]
    async fn gc_keeps_referenced_hashes_and_removes_the_rest() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();

        let url = "https://x.test/a.png";
        let a = extract_images("e", &format!("![a]({url})"), &map(&[(url, PNG_DATA)]));
        write_entry_images(lib, "e", &a.images, token()).await;
        let hash = image_url_hash(url);
        let kept_file = format!("{hash}.png");

        let dir = crate::library::entry_images_dir(lib, "e");
        tokio::fs::write(dir.join("orphan.png"), b"stale").await.unwrap();

        // keep is the set of referenced HASHES; files kept by stem.
        let keep: HashSet<String> = std::iter::once(hash).collect();
        gc_entry_images(lib, "e", &keep, token()).await;

        assert!(dir.join(&kept_file).exists());
        assert!(!dir.join("orphan.png").exists());
        assert!(dir.exists());
    }

    #[tokio::test]
    async fn gc_removes_the_directory_when_nothing_is_referenced() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();

        let url = "https://x.test/a.png";
        let a = extract_images("e", &format!("![a]({url})"), &map(&[(url, PNG_DATA)]));
        write_entry_images(lib, "e", &a.images, token()).await;
        let dir = crate::library::entry_images_dir(lib, "e");
        assert!(dir.exists());

        gc_entry_images(lib, "e", &HashSet::new(), token()).await;
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn read_caps_oversize_file() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();
        let dir = crate::library::entry_images_dir(lib, "e");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let hash = "0123456789ab";
        tokio::fs::write(
            dir.join(format!("{hash}.png")),
            vec![0u8; (MAX_IMAGE_BYTES + 1) as usize],
        )
        .await
        .unwrap();

        let reference = crate::library::entry_image_ref("e", hash);
        let read = read_entry_images(lib, "e", &[reference]).await;
        assert_eq!(read, vec![None]);
    }

    #[tokio::test]
    async fn read_missing_file_is_none_not_error() {
        let tmp = tempfile::tempdir().unwrap();
        let reference = crate::library::entry_image_ref("e", "deadbeef0000");
        let read = read_entry_images(tmp.path(), "e", &[reference]).await;
        assert_eq!(read, vec![None]);
    }

    #[test]
    fn referenced_image_hashes_for_this_entry_only() {
        let body = "\
intro ![a](../images/e1/4f2a91c3e0b7) and \
![b](../images/e1/9c1d55ae32f0) then a foreign one \
![c](../images/other/deadbeef0000)";
        let hashes = referenced_image_hashes("e1", body);
        assert_eq!(hashes.len(), 2);
        assert!(hashes.contains("4f2a91c3e0b7"));
        assert!(hashes.contains("9c1d55ae32f0"));
        assert!(!hashes.contains("deadbeef0000"));
    }
}
