//! Shared staging and explicit byte-validation boundary. No network or library
//! writes happen here; consumers persist only the validator's accepted output.
use std::{
    fs::File,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

pub(crate) trait ByteValidator: Send + 'static {
    type Accepted: Send + 'static;
    fn validate(self, bytes: &[u8]) -> Result<Self::Accepted, String>;
}

pub(crate) struct StagedDownload<V: ByteValidator> {
    directory: tempfile::TempDir,
    path: PathBuf,
    max_bytes: u64,
    validator: V,
}
impl<V: ByteValidator> StagedDownload<V> {
    pub(crate) fn new(validator: V, max_bytes: u64) -> Result<Self, String> {
        // OS-private temporary storage; remote filenames never enter this path.
        let directory = tempfile::Builder::new()
            .prefix("xaga-resource-")
            .tempdir()
            .map_err(|_| "cannot create download staging directory")?;
        let path = directory.path().join("resource");
        Ok(Self {
            directory,
            path,
            max_bytes,
            validator,
        })
    }
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn finish(self, success: bool) -> Result<V::Accepted, String> {
        if !success {
            return Err("download failed or was cancelled".into());
        }
        let metadata =
            std::fs::symlink_metadata(&self.path).map_err(|_| "downloaded file is missing")?;
        if !metadata.file_type().is_file() || metadata.len() > self.max_bytes {
            return Err("downloaded file exceeds policy or is not a regular file".into());
        }
        let mut bytes = Vec::new();
        File::open(&self.path)
            .map_err(|_| "cannot read downloaded file")?
            .take(self.max_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "cannot read downloaded file")?;
        if bytes.len() as u64 > self.max_bytes {
            return Err("download exceeds byte limit".into());
        }
        let accepted = self.validator.validate(&bytes);
        // TempDir is retained through validation and removes even rejected bytes.
        drop(self.directory);
        accepted
    }
}

/// Favicon conversion is a consumer operation, after shared image validation.
/// Formats without a Rust thumbnail decoder retain their validated original bytes.
pub(crate) fn prepare_favicon(data: String) -> Result<String, String> {
    use base64::Engine;
    let (header, payload) = data.split_once(',').ok_or("invalid image payload")?;
    if matches!(
        header,
        "data:image/svg+xml;base64" | "data:image/avif;base64"
    ) {
        return Ok(data);
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| "invalid image payload")?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "unrecognized image bytes")?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    let icon = reader
        .decode()
        .map_err(|_| "cannot decode icon")?
        .thumbnail(64, 64);
    let mut png = Cursor::new(Vec::new());
    icon.write_to(&mut png, image::ImageFormat::Png)
        .map_err(|_| "cannot encode icon")?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png.into_inner())
    ))
}

/// Parse SVG as data, without rendering, executing scripts, resolving entities,
/// or fetching external resources. Stored bytes remain untrusted at render time.
fn validate_svg(bytes: &[u8]) -> Result<(), String> {
    use quick_xml::{NsReader, events::Event, name::ResolveResult};
    let mut reader = NsReader::from_reader(bytes);
    let mut depth = 0usize;
    let mut root = false;
    loop {
        let (namespace, event) = reader
            .read_resolved_event()
            .map_err(|_| "invalid SVG XML")?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if depth == 0 {
                    if root
                        || element.local_name().as_ref() != b"svg"
                        || !matches!(namespace, ResolveResult::Bound(ref ns) if ns.as_ref() == b"http://www.w3.org/2000/svg")
                    {
                        return Err("not an SVG document".into());
                    }
                    root = true;
                }
                // Parse attributes too: quick-xml reads these lazily.
                for attr in element.attributes() {
                    attr.map_err(|_| "invalid SVG attribute")?;
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or("invalid SVG XML")?;
            }
            Event::Text(ref text) if depth == 0 && !text.iter().all(u8::is_ascii_whitespace) => {
                return Err("text outside SVG root".into());
            }
            Event::CData(_) if depth == 0 => return Err("text outside SVG root".into()),
            Event::Eof => {
                return if root && depth == 0 {
                    Ok(())
                } else {
                    Err("incomplete SVG document".into())
                };
            }
            _ => {}
        }
    }
}

/// Content images retain their original encoding, dimensions and animation.
/// The persistence read cap also bounds acquisition, so every accepted resource
/// can subsequently be read by the reader.
pub(crate) const IMAGE_MAX_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) struct ImageValidator;
impl ByteValidator for ImageValidator {
    type Accepted = String;
    fn validate(self, bytes: &[u8]) -> Result<String, String> {
        use base64::Engine;
        if bytes.len() as u64 > IMAGE_MAX_BYTES {
            return Err("image exceeds byte limit".into());
        }
        let format = image::guess_format(bytes).ok();
        let media = if format == Some(image::ImageFormat::Avif) {
            let avif = avif_parse::AvifData::from_reader(&mut Cursor::new(bytes))
                .map_err(|_| "invalid AVIF container")?;
            let metadata = avif
                .primary_item_metadata()
                .map_err(|_| "invalid AVIF image metadata")?;
            avif.alpha_item_metadata()
                .map_err(|_| "invalid AVIF alpha metadata")?;
            if u64::from(metadata.max_frame_width.get())
                * u64::from(metadata.max_frame_height.get())
                > 8 * 1024 * 1024
            {
                return Err("oversized image".into());
            }
            "image/avif"
        } else if format.is_none() {
            validate_svg(bytes)?;
            "image/svg+xml"
        } else {
            let media = match format {
                Some(image::ImageFormat::Png) => "image/png",
                Some(image::ImageFormat::Jpeg) => "image/jpeg",
                Some(image::ImageFormat::Gif) => "image/gif",
                Some(image::ImageFormat::WebP) => "image/webp",
                Some(image::ImageFormat::Ico) => "image/x-icon",
                _ => return Err("unsupported image format".into()),
            };
            let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format.unwrap());
            let mut limits = image::Limits::default();
            limits.max_alloc = Some(32 * 1024 * 1024);
            reader.limits(limits);
            reader
                .decode()
                .map_err(|_| "invalid, unsupported, or oversized image")?;
            media
        };
        Ok(format!(
            "data:{media};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }
}

/// Renderer bytes cross the same validator as a native download. MIME supplied
/// by the page is not evidence of format.
pub(crate) fn validate_image_data(data: &str) -> Result<String, String> {
    use base64::Engine;
    if data.len() > (IMAGE_MAX_BYTES as usize * 4 / 3) + 128 {
        return Err("image exceeds byte limit".into());
    }
    let (header, payload) = data.split_once(',').ok_or("invalid image payload")?;
    if !header.starts_with("data:") || !header.ends_with(";base64") {
        return Err("invalid image payload".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| "invalid image payload")?;
    ImageValidator.validate(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_images_preserve_original_bytes_dimensions_and_encoding() {
        use base64::Engine;
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1200, 300)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let original = bytes.into_inner();
        let accepted = ImageValidator.validate(&original).unwrap();
        let restored = base64::engine::general_purpose::STANDARD
            .decode(accepted.split_once(',').unwrap().1)
            .unwrap();
        assert_eq!(restored, original);
        assert!(ImageValidator.validate(b"<html>error</html>").is_err());
        assert!(validate_image_data("data:image/png;base64,PGh0bWw+").is_err());
    }

    #[test]
    fn svg_uses_the_same_acceptance_for_download_callback_and_favicon() {
        use base64::Engine;
        let svg = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="red"/></svg>"#;
        let accepted = ImageValidator.validate(svg).unwrap();
        assert!(accepted.starts_with("data:image/svg+xml;base64,"));
        assert_eq!(prepare_favicon(accepted.clone()).unwrap(), accepted);
        // A page's MIME claim does not decide format acceptance.
        let callback = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(svg)
        );
        assert_eq!(validate_image_data(&callback).unwrap(), accepted);
        let job = StagedDownload::new(ImageValidator, IMAGE_MAX_BYTES).unwrap();
        std::fs::write(job.path(), svg).unwrap();
        assert_eq!(job.finish(true).unwrap(), accepted);
        for invalid in [
            b"<html>error</html>".as_slice(),
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"><g></svg>",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\">",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"/><html/>",
            b"<svg xmlns=\"https://wrong.example\"/>",
        ] {
            assert!(ImageValidator.validate(invalid).is_err());
        }
    }

    #[test]
    fn svg_validation_parses_without_transforming_active_content() {
        use base64::Engine;
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><script>alert(2)</script><image href="https://example.invalid/image"/></svg>"#;
        let accepted = ImageValidator.validate(svg).unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(accepted.split_once(',').unwrap().1)
                .unwrap(),
            svg
        );
    }

    #[test]
    fn avif_uses_the_same_acceptance_for_download_callback_and_favicon() {
        use base64::Engine;
        let bytes = include_bytes!("../tests/fixtures/color-profile.avif");
        let accepted = ImageValidator.validate(bytes).unwrap();
        assert!(accepted.starts_with("data:image/avif;base64,"));
        assert_eq!(prepare_favicon(accepted.clone()).unwrap(), accepted);
        assert_eq!(validate_image_data(&accepted).unwrap(), accepted);
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(accepted.split_once(',').unwrap().1)
                .unwrap(),
            bytes
        );
        let job = StagedDownload::new(ImageValidator, IMAGE_MAX_BYTES).unwrap();
        std::fs::write(job.path(), bytes).unwrap();
        assert_eq!(job.finish(true).unwrap(), accepted);
        assert!(ImageValidator.validate(&bytes[..32]).is_err());
        assert!(
            ImageValidator
                .validate(&vec![0; IMAGE_MAX_BYTES as usize + 1])
                .is_err()
        );
    }

    struct Signature;
    impl ByteValidator for Signature {
        type Accepted = Vec<u8>;
        fn validate(self, bytes: &[u8]) -> Result<Vec<u8>, String> {
            if bytes.starts_with(b"accepted") {
                Ok(bytes.to_vec())
            } else {
                Err("rejected".into())
            }
        }
    }
    #[test]
    fn validation_is_required_and_staging_is_cleaned_on_all_results() {
        for (bytes, success, accepted) in [
            (b"accepted".as_slice(), true, true),
            (b"bad", true, false),
            (b"accepted", false, false),
        ] {
            let job = StagedDownload::new(Signature, 16).unwrap();
            let path = job.path().to_owned();
            std::fs::write(&path, bytes).unwrap();
            assert_eq!(job.finish(success).is_ok(), accepted);
            assert!(!path.parent().unwrap().exists());
        }
    }
    #[test]
    fn rejects_oversize_and_cleans_abandoned_downloads() {
        let job = StagedDownload::new(Signature, 2).unwrap();
        let path = job.path().to_owned();
        std::fs::write(&path, b"accepted").unwrap();
        assert!(job.finish(true).is_err());
        assert!(!path.exists());
        let job = StagedDownload::new(Signature, 16).unwrap();
        let dir = job.path().parent().unwrap().to_owned();
        drop(job);
        assert!(!dir.exists());
    }
    #[cfg(unix)]
    #[test]
    fn staging_does_not_follow_a_symlink() {
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"accepted").unwrap();
        let job = StagedDownload::new(Signature, 16).unwrap();
        std::os::unix::fs::symlink(outside.path(), job.path()).unwrap();
        assert!(job.finish(true).is_err());
        assert_eq!(std::fs::read(outside.path()).unwrap(), b"accepted");
    }
    #[test]
    fn image_acceptance_does_not_depend_on_favicon_dimensions() {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2049, 1)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert!(ImageValidator.validate(bytes.get_ref()).is_ok());
    }

    #[test]
    fn favicon_thumbnailing_follows_shared_validation() {
        assert!(
            ImageValidator
                .validate(b"<html>not an icon</html>")
                .is_err()
        );
        let input = image::DynamicImage::new_rgba8(100, 40);
        let mut bytes = Cursor::new(Vec::new());
        input.write_to(&mut bytes, image::ImageFormat::Ico).unwrap();
        let output = prepare_favicon(ImageValidator.validate(bytes.get_ref()).unwrap()).unwrap();
        use base64::Engine;
        let png = base64::engine::general_purpose::STANDARD
            .decode(output.strip_prefix("data:image/png;base64,").unwrap())
            .unwrap();
        let result = image::load_from_memory(&png).unwrap();
        assert_eq!((result.width(), result.height()), (64, 26));
    }
}

/// Existing PDF format check, injected into the same staging boundary as icons.
pub(crate) struct PdfValidator;
impl ByteValidator for PdfValidator {
    type Accepted = Vec<u8>;
    fn validate(self, bytes: &[u8]) -> Result<Vec<u8>, String> {
        xaga_core::persistence::pdfs::validate(bytes)?;
        Ok(bytes.to_vec())
    }
}
