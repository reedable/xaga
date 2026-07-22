//! Entry image assets: extracting inlined images out of an entry's markdown
//! body into per-entry files, and reading them back as `data:` URIs.
//!
//! The article loader inlines remote images as `data:` URIs before capture, so
//! the markdown handed to `save_article` can carry base64 image payloads. Two
//! reasons those must not stay in the body: `article_embed_text`
//! (`embeddings/row.rs`) falls back to the body when an entry has no summary, so
//! base64 would become embedding input; and keeping remote URLs in stored
//! content is the read-time tracking surface this feature removes.
//!
//! [`extract_images`] rewrites the body — image bytes move to files under
//! `library::entry_images_dir`, referenced by a relative `../images/<id>/<hash>`
//! path; images that failed to inline (still `http(s):`) are neutralised to
//! `<img … data-xaga-src>` so they carry no `src` and fetch nothing. The scan
//! is a targeted, fail-safe span replacement, never a markdown round-trip:
//! anything it cannot parse with certainty is left byte-for-byte untouched, so a
//! parse it doesn't understand degrades to today's behaviour rather than
//! corrupting an irreplaceable body.

use std::collections::HashSet;
use std::path::Path;

use base64::Engine;
use sha2::{Digest, Sha256};

use crate::library_writer::WriteToken;

/// Per-save cap on the total decoded image bytes extracted from one body,
/// mirroring the loader's own `__XAGA_INLINE_TOTAL_BYTES__` budget. Images past
/// this are dropped (alt text kept), never written.
pub(crate) const MAX_ENTRY_IMAGE_BYTES: usize = 24 * 1024 * 1024;

/// Per-file read cap when reconstructing a `data:` URI at read time, mirroring
/// `fs::MAX_MD_BYTES` in spirit. A file over this reads as `None`.
pub(crate) const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

/// Cap on the total decoded bytes one `read_entry_images` call will return, so a
/// pathological entry cannot balloon the IPC payload. Refs past the cap read as
/// `None`.
pub(crate) const MAX_READ_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

/// An image decoded out of a body, ready to be written to disk. `file_name`
/// already carries its content-derived name and extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtractedImage {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// The result of rewriting a body: the new text-only markdown and the images to
/// persist. `images` is deduplicated by `file_name` (identical bytes appearing
/// twice yield one file, two references).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Extraction {
    pub body: String,
    pub images: Vec<ExtractedImage>,
}

/// Maps an image media type to the file extension used on disk. The extension
/// comes from the media type carried in the `data:` URI — which is the response
/// `Content-Type` the loader gated on — never from the source URL. Returns
/// `None` for anything not on the allowlist; such references are dropped rather
/// than persisted.
pub(crate) fn ext_for_media_type(media_type: &str) -> Option<&'static str> {
    match media_type {
        "image/jpeg" => Some("jpg"),
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
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    }
}

/// First 12 hex chars of the SHA-256 of the bytes — the same compact,
/// filesystem-safe idiom as `util::url_hash12`, but content-addressed: identical
/// bytes always yield the same filename, so re-saving an unchanged article
/// rewrites nothing and duplicate images within one article store once.
pub(crate) fn image_hash12(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)[..12].to_string()
}

/// Classification of a markdown image's destination URL.
enum Dest<'a> {
    /// A base64 `data:` URI with an allowlisted image media type.
    InlineImage { ext: &'static str, payload: &'a str },
    /// A `data:` URI we will not persist (unknown media type, or not base64).
    DropData,
    /// A remote reference to neutralise into `data-xaga-src`.
    Remote,
    /// Anything else (already-local `../images/…`, other relative paths): leave
    /// the whole reference untouched.
    Leave,
}

fn classify(url: &str) -> Dest<'_> {
    if let Some(rest) = url.strip_prefix("data:") {
        // `data:<media-type>;base64,<payload>`. Only base64 image payloads are
        // stored; percent-encoded (`data:<mt>,<payload>`) is dropped.
        if let Some((meta, payload)) = rest.split_once(',') {
            if let Some(media_type) = meta.strip_suffix(";base64") {
                if let Some(ext) = ext_for_media_type(media_type) {
                    return Dest::InlineImage { ext, payload };
                }
            }
        }
        return Dest::DropData;
    }
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Dest::Remote;
    }
    Dest::Leave
}

/// Escapes a string for use inside a double-quoted HTML attribute.
fn html_attr_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
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
            if bytes.get(gt + 1) != Some(&b')') {
                i = bang + 2;
                continue;
            }
            (&markdown[url_open + 1..gt], gt + 2)
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

/// Rewrites `markdown` so no image reference carries a remote or inline-data
/// `src`: inlined `data:` images become files referenced by `../images/<id>/…`,
/// unsupported/undecodable `data:` images are dropped to their alt text, and
/// remote images are neutralised to `<img … data-xaga-src>`. Every byte outside
/// a recognised image reference is preserved exactly.
pub(crate) fn extract_images(id: &str, markdown: &str) -> Extraction {
    // No image syntax at all: nothing to do, return the body unchanged. (This
    // is the common no-op path for image-free entries.)
    if !markdown.contains("![") {
        return Extraction {
            body: markdown.to_string(),
            images: Vec::new(),
        };
    }

    let engine = base64::engine::general_purpose::STANDARD;
    let mut out = String::with_capacity(markdown.len());
    let mut images: Vec<ExtractedImage> = Vec::new();
    let mut budget_used: usize = 0;
    let mut cursor = 0usize;

    while let Some(m) = next_image(markdown, cursor) {
        out.push_str(&markdown[cursor..m.span.start]);

        match classify(m.url_raw) {
            Dest::InlineImage { ext, payload } => {
                match engine.decode(payload.trim()) {
                    Ok(bytes) if budget_used + bytes.len() <= MAX_ENTRY_IMAGE_BYTES => {
                        budget_used += bytes.len();
                        let file_name = format!("{}.{ext}", image_hash12(&bytes));
                        if !images.iter().any(|img| img.file_name == file_name) {
                            images.push(ExtractedImage {
                                file_name: file_name.clone(),
                                bytes,
                            });
                        }
                        // Preserve the alt bytes verbatim — no re-escaping.
                        out.push_str("![");
                        out.push_str(m.alt_raw);
                        out.push_str("](");
                        out.push_str(&crate::library::entry_image_ref(id, &file_name));
                        out.push(')');
                    }
                    // Undecodable or over budget: drop to alt text, never leave
                    // base64 in the body.
                    _ => out.push_str(m.alt_raw),
                }
            }
            Dest::DropData => out.push_str(m.alt_raw),
            Dest::Remote => {
                let alt = html_attr_escape(&markdown_unescape(m.alt_raw));
                let url = html_attr_escape(&markdown_unescape(m.url_raw));
                out.push_str("<img alt=\"");
                out.push_str(&alt);
                out.push_str("\" data-xaga-src=\"");
                out.push_str(&url);
                out.push_str("\">");
            }
            Dest::Leave => out.push_str(&markdown[m.span.clone()]),
        }

        cursor = m.span.end;
    }
    out.push_str(&markdown[cursor..]);

    Extraction { body: out, images }
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
/// saved even if an image cannot be — a missing image is one reload away, and a
/// reference to it degrades to a broken image with no network access, not data
/// loss.
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
/// that still references it. Also sweeps `write_atomic`'s `.tmp` leftovers.
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
                if !keep.contains(name.as_ref()) {
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

/// The set of image file names an entry body references (the `<file>` in each
/// `../images/<id>/<file>`). Used by the CLI sweep to decide which stored files
/// are still live. Scans for the reference prefix and reads each file name up to
/// the first delimiter (`)`, `]`, quote, or whitespace) — the shapes a markdown
/// image or raw `<img>` src can end with.
pub fn referenced_image_files(id: &str, body: &str) -> HashSet<String> {
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

/// Reads bytes back for each body reference, returning a complete `data:` URI or
/// `None`, index-aligned with `refs`. `None` covers every non-fault case: a ref
/// that is not a well-formed `../images/<id>/<file>` for THIS entry, an
/// unknown extension, a missing/oversize file, or the response byte budget being
/// exhausted. A bad reference is data, never an error — the reader leaves the
/// original `src` in place, and in the sandboxed blob iframe that fetches
/// nothing.
pub async fn read_entry_images(library_dir: &Path, id: &str, refs: &[String]) -> Vec<Option<String>> {
    let engine = base64::engine::general_purpose::STANDARD;
    let mut out = Vec::with_capacity(refs.len());
    let mut total: usize = 0;
    for reference in refs {
        out.push(resolve_ref(library_dir, id, reference, &engine, &mut total).await);
    }
    out
}

async fn resolve_ref(
    library_dir: &Path,
    id: &str,
    reference: &str,
    engine: &base64::engine::GeneralPurpose,
    total: &mut usize,
) -> Option<String> {
    let (ref_id, file_name) = crate::library::parse_entry_image_ref(reference)?;
    // An entry may only reach into its own image directory.
    if ref_id != id {
        return None;
    }
    let ext = file_name.rsplit_once('.').map(|(_, e)| e)?;
    let media_type = media_type_for_ext(ext)?;

    let path = crate::library::entry_image_path(library_dir, id, file_name);
    let bytes = read_capped_bytes(&path, MAX_IMAGE_BYTES).await?;

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

    #[test]
    fn noop_when_no_image_syntax() {
        let md = "# Title\n\nJust prose with a [link](https://x.test/) and no images.\n";
        let out = extract_images("abc123", md);
        assert_eq!(out.body, md);
        assert!(out.images.is_empty());
    }

    #[test]
    fn inlines_data_image_to_file_and_keeps_alt() {
        let md = format!("before ![a cat]({PNG_DATA}) after");
        let out = extract_images("entry1", &md);
        assert_eq!(out.images.len(), 1);
        let file = &out.images[0].file_name;
        assert!(file.ends_with(".png"));
        assert_eq!(
            out.body,
            format!("before ![a cat](../images/entry1/{file}) after")
        );
        assert!(!out.body.contains("base64"));
    }

    #[test]
    fn leaves_https_and_relative_spans_byte_identical_outside_the_ref() {
        // A body with only a relative ../images ref and prose must round-trip
        // unchanged (idempotent re-save).
        let md = "text ![local](../images/entry1/4f2a91c3e0b7.png) more text\n";
        let out = extract_images("entry1", md);
        assert_eq!(out.body, md);
        assert!(out.images.is_empty());
    }

    #[test]
    fn neutralises_remote_image_and_unescapes_url() {
        // remark escapes `&` as `\&`; the stored attribute must hold the real URL.
        let md = "![pixel \\[x\\]](https://tracker.test/p.gif?u=1\\&v=2)";
        let out = extract_images("e", md);
        assert!(out.images.is_empty());
        assert_eq!(
            out.body,
            "<img alt=\"pixel [x]\" data-xaga-src=\"https://tracker.test/p.gif?u=1&amp;v=2\">"
        );
    }

    #[test]
    fn dedupes_identical_bytes_to_one_file_two_refs() {
        let md = format!("![one]({PNG_DATA}) and ![two]({PNG_DATA})");
        let out = extract_images("e", &md);
        assert_eq!(out.images.len(), 1);
        let file = &out.images[0].file_name;
        let expected = format!(
            "![one](../images/e/{file}) and ![two](../images/e/{file})"
        );
        assert_eq!(out.body, expected);
    }

    #[test]
    fn drops_unknown_media_type_without_leaking_base64() {
        let md = "![x](data:application/pdf;base64,JVBERi0=)";
        let out = extract_images("e", &md);
        assert!(out.images.is_empty());
        assert_eq!(out.body, "x");
        assert!(!out.body.contains("base64"));
    }

    #[test]
    fn keeps_svg_data_image() {
        let svg = "data:image/svg+xml;base64,PHN2Zz48L3N2Zz4=";
        let md = format!("![diagram]({svg})");
        let out = extract_images("e", &md);
        assert_eq!(out.images.len(), 1);
        assert!(out.images[0].file_name.ends_with(".svg"));
    }

    #[test]
    fn stops_at_total_byte_budget() {
        // Two distinct large images; only the first fits the budget.
        let big_a = base64::engine::general_purpose::STANDARD
            .encode(vec![0xABu8; MAX_ENTRY_IMAGE_BYTES - 100]);
        let big_b = base64::engine::general_purpose::STANDARD
            .encode(vec![0xCDu8; MAX_ENTRY_IMAGE_BYTES - 100]);
        let md = format!(
            "![a](data:image/png;base64,{big_a}) ![b](data:image/png;base64,{big_b})"
        );
        let out = extract_images("e", &md);
        assert_eq!(out.images.len(), 1);
        // The dropped one falls back to alt text, no base64 remains.
        assert!(out.body.contains("![a](../images/e/"));
        assert!(out.body.ends_with(" b"));
        assert!(!out.body.contains("base64"));
    }

    #[test]
    fn ignores_escaped_bang_and_plain_links() {
        // `\!` is not an image; `[text](data:...)` is a link, not an image.
        let md = "\\![notimg](https://x.test/a.png) and [text](data:image/png;base64,AAAA)";
        let out = extract_images("e", md);
        assert_eq!(out.body, md);
        assert!(out.images.is_empty());
    }

    #[test]
    fn hash_is_stable_and_content_addressed() {
        assert_eq!(image_hash12(b"hello"), image_hash12(b"hello"));
        assert_ne!(image_hash12(b"hello"), image_hash12(b"world"));
        assert_eq!(image_hash12(b"hello").len(), 12);
    }

    // ── I/O round-trip ──────────────────────────────────────────────────────

    fn token() -> WriteToken {
        WriteToken::for_test()
    }

    #[tokio::test]
    async fn write_then_read_round_trips_a_data_uri() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();

        let ex = extract_images("e1", &format!("![c]({PNG_DATA})"));
        let landed = write_entry_images(lib, "e1", &ex.images, token()).await;
        assert_eq!(landed.len(), 1);

        // The stored ref, pulled from the rewritten body, reads back as a data URI.
        let file = &ex.images[0].file_name;
        let reference = crate::library::entry_image_ref("e1", file);
        let read = read_entry_images(lib, "e1", &[reference]).await;
        assert_eq!(read.len(), 1);
        let uri = read[0].as_ref().expect("should resolve");
        assert!(uri.starts_with("data:image/png;base64,"));
    }

    #[tokio::test]
    async fn read_refuses_traversal_and_foreign_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();
        // Plant a real file under entry `victim`.
        let ex = extract_images("victim", &format!("![x]({PNG_DATA})"));
        write_entry_images(lib, "victim", &ex.images, token()).await;
        let victim_file = &ex.images[0].file_name;

        let bad = vec![
            // Another entry reaching into victim's dir.
            crate::library::entry_image_ref("victim", victim_file),
            "../images/victim/../../entries/x.md".to_string(),
            "https://evil.test/x.png".to_string(),
        ];
        // Caller id is `attacker`, so even the well-formed victim ref must be None.
        let read = read_entry_images(lib, "attacker", &bad).await;
        assert_eq!(read, vec![None, None, None]);
    }

    #[tokio::test]
    async fn gc_removes_unreferenced_and_keeps_referenced() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();

        let a = extract_images("e", &format!("![a]({PNG_DATA})"));
        write_entry_images(lib, "e", &a.images, token()).await;
        let keep_name = a.images[0].file_name.clone();

        // A stray file that nothing references.
        let dir = crate::library::entry_images_dir(lib, "e");
        tokio::fs::write(dir.join("orphan.png"), b"stale").await.unwrap();

        let keep: HashSet<String> = std::iter::once(keep_name.clone()).collect();
        gc_entry_images(lib, "e", &keep, token()).await;

        assert!(dir.join(&keep_name).exists());
        assert!(!dir.join("orphan.png").exists());
        // A file survived, so the directory must remain.
        assert!(dir.exists());
    }

    #[tokio::test]
    async fn gc_removes_the_directory_when_nothing_is_referenced() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();

        let a = extract_images("e", &format!("![a]({PNG_DATA})"));
        write_entry_images(lib, "e", &a.images, token()).await;
        let dir = crate::library::entry_images_dir(lib, "e");
        assert!(dir.exists());

        // A later save references no images: keep is empty, so every file is
        // swept and the now-empty directory is removed.
        gc_entry_images(lib, "e", &HashSet::new(), token()).await;
        assert!(!dir.exists());
    }

    #[tokio::test]
    async fn read_caps_oversize_file() {
        let tmp = tempfile::tempdir().unwrap();
        let lib = tmp.path();
        let dir = crate::library::entry_images_dir(lib, "e");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let name = "0123456789ab.png";
        tokio::fs::write(dir.join(name), vec![0u8; (MAX_IMAGE_BYTES + 1) as usize])
            .await
            .unwrap();

        let reference = crate::library::entry_image_ref("e", name);
        let read = read_entry_images(lib, "e", &[reference]).await;
        assert_eq!(read, vec![None]);
    }

    #[tokio::test]
    async fn read_missing_file_is_none_not_error() {
        let tmp = tempfile::tempdir().unwrap();
        let reference = crate::library::entry_image_ref("e", "deadbeef0000.png");
        let read = read_entry_images(tmp.path(), "e", &[reference]).await;
        assert_eq!(read, vec![None]);
    }

    #[test]
    fn referenced_image_files_extracts_names_for_this_entry_only() {
        let body = "\
intro ![a](../images/e1/4f2a91c3e0b7.png) and \
![b](../images/e1/9c1d55ae32f0.jpg) then a foreign one \
![c](../images/other/deadbeef0000.png) and a remote \
<img data-xaga-src=\"https://x.test/p.gif\">";
        let files = referenced_image_files("e1", body);
        assert_eq!(files.len(), 2);
        assert!(files.contains("4f2a91c3e0b7.png"));
        assert!(files.contains("9c1d55ae32f0.jpg"));
        // The other entry's ref is not attributed to e1.
        assert!(!files.contains("deadbeef0000.png"));
    }
}
