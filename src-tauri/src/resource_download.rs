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

pub(crate) const ICON_MAX_BYTES: u64 = 8 * 1024 * 1024;
pub(crate) struct IconValidator;
impl ByteValidator for IconValidator {
    type Accepted = String;
    fn validate(self, bytes: &[u8]) -> Result<String, String> {
        use base64::Engine;
        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| "unrecognized image bytes")?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(2048);
        limits.max_image_height = Some(2048);
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader
            .decode()
            .map_err(|_| "invalid, unsupported, or oversized image")?;
        let icon = decoded.thumbnail(64, 64);
        let mut png = Cursor::new(Vec::new());
        icon.write_to(&mut png, image::ImageFormat::Png)
            .map_err(|_| "cannot encode icon")?;
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.into_inner())
        ))
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
        let mut reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|_| "unrecognized image bytes")?;
        let media = match reader.format() {
            Some(image::ImageFormat::Png) => "image/png",
            Some(image::ImageFormat::Jpeg) => "image/jpeg",
            Some(image::ImageFormat::Gif) => "image/gif",
            Some(image::ImageFormat::WebP) => "image/webp",
            Some(image::ImageFormat::Ico) => "image/x-icon",
            _ => return Err("unsupported image format".into()),
        };
        let mut limits = image::Limits::default();
        limits.max_alloc = Some(32 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .map_err(|_| "invalid, unsupported, or oversized image")?;
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
    fn oversized_image_dimensions_are_rejected() {
        let mut bytes = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(2049, 1)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        assert!(IconValidator.validate(bytes.get_ref()).is_err());
    }

    #[test]
    fn icon_validator_decodes_bytes_not_extensions_and_rejects_non_images() {
        assert!(IconValidator.validate(b"<html>not an icon</html>").is_err());
        let input = image::DynamicImage::new_rgba8(100, 40);
        let mut bytes = Cursor::new(Vec::new());
        input.write_to(&mut bytes, image::ImageFormat::Ico).unwrap();
        let output = IconValidator.validate(bytes.get_ref()).unwrap();
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
