//! The library's ordered pin membership. Entity files and query rows do not own pins.
use noyalib::{ParserConfig, policy::DenyAnchors};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum PinTarget {
    Entry { id: String },
    Feed { id: String },
    Topic { id: String },
}

impl PinTarget {
    pub fn validate(&self) -> Result<(), PinsError> {
        let (Self::Entry { id } | Self::Feed { id } | Self::Topic { id }) = self;
        if crate::persistence::entries::is_safe_id(id) {
            Ok(())
        } else {
            Err(PinsError::InvalidTarget)
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub pins: Vec<PinTarget>,
}

#[derive(Debug, thiserror::Error)]
pub enum PinsError {
    #[error("pin store IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid pin store: {0}")]
    Format(String),
    #[error("invalid pin target")]
    InvalidTarget,
}
impl PinsError {
    pub fn code(&self) -> &'static str {
        "pins:invalid-store"
    }
    pub fn user_message(&self) -> &'static str {
        "Couldn't read or save Shortcuts."
    }
}

impl Pins {
    fn validate(&self) -> Result<(), PinsError> {
        let mut seen = HashSet::new();
        for target in &self.pins {
            target.validate()?;
            if !seen.insert(target) {
                return Err(PinsError::Format("duplicate target".into()));
            }
        }
        Ok(())
    }

    /// Validated encoding shared with the explicit offline converter.
    pub fn serialize(&self) -> Result<String, PinsError> {
        self.validate()?;
        let yaml = noyalib::to_string_with_config(
            self,
            &noyalib::SerializerConfig::new().block_scalars(false),
        )
        .map_err(|error| PinsError::Format(error.to_string()))?;
        if yaml.len() > 12 * 1024 * 1024 {
            return Err(PinsError::Format("store exceeds 12 MiB".into()));
        }
        Ok(yaml)
    }
    /// Missing stores are empty. Malformed stores fail closed before any mutation.
    pub async fn read(dir: &Path) -> Result<Self, PinsError> {
        use tokio::io::AsyncReadExt;
        let file = match tokio::fs::File::open(crate::library::pins_path(dir)).await {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        let mut bytes = Vec::new();
        file.take(12 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .await?;
        if bytes.len() > 12 * 1024 * 1024 {
            return Err(PinsError::Format("store exceeds 12 MiB".into()));
        }
        let raw = std::str::from_utf8(&bytes).map_err(|e| PinsError::Format(e.to_string()))?;
        let store: Self =
            noyalib::from_str_with_config(raw, &ParserConfig::strict().with_policy(DenyAnchors))
                .map_err(|e| PinsError::Format(e.to_string()))?;
        store.validate()?;
        Ok(store)
    }

    /// Resolve article targets from source files using this one membership snapshot.
    /// Missing targets remain in the store and are represented by the UI resolver.
    pub async fn list_entries(
        &self,
        dir: &Path,
    ) -> Result<Vec<crate::types::Entry>, crate::persistence::entries::EntryError> {
        use crate::persistence::entries::{EntryError, read_entry_metadata};
        let mut entries = Vec::new();
        for target in &self.pins {
            if let PinTarget::Entry { id } = target {
                match read_entry_metadata(dir, id).await {
                    Ok(mut entry) => {
                        entry.is_pinned = true;
                        entries.push(entry);
                    }
                    Err(EntryError::NotFound) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(entries)
    }

    pub fn contains(&self, target: &PinTarget) -> bool {
        self.pins.contains(target)
    }

    pub(crate) async fn set(
        dir: &Path,
        target: PinTarget,
        pinned: bool,
        _token: crate::library_writer::WriteToken,
    ) -> Result<Self, PinsError> {
        target.validate()?;
        let mut store = Self::read(dir).await?;
        if store.contains(&target) == pinned {
            return Ok(store);
        }
        if pinned {
            store.pins.push(target);
        } else {
            store.pins.retain(|value| value != &target);
        }
        let yaml = store.serialize()?;
        crate::fs::write_atomic_async(crate::library::pins_path(dir), yaml.into_bytes()).await?;
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{library, library_writer::LibraryWriter};

    #[tokio::test]
    async fn typed_identity_order_unresolved_targets_and_noop_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let entry = PinTarget::Entry {
            id: "missing".into(),
        };
        let feed = PinTarget::Feed {
            id: "missing".into(),
        };
        writer.set_pin(entry.clone(), true).await.unwrap();
        writer.set_pin(feed.clone(), true).await.unwrap();
        let store = Pins::read(dir.path()).await.unwrap();
        assert_eq!(store.pins, vec![entry.clone(), feed.clone()]);
        let path = library::pins_path(dir.path());
        let raw = std::fs::read_to_string(&path).unwrap();
        let commented = format!("# user comment\n{raw}");
        std::fs::write(&path, &commented).unwrap();
        writer.set_pin(entry.clone(), true).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), commented);
        writer.set_pin(entry.clone(), false).await.unwrap();
        writer.shutdown().await;
        assert_eq!(Pins::read(dir.path()).await.unwrap().pins, vec![feed]);
        assert!(writer.set_pin(entry, true).await.is_err());
    }

    #[tokio::test]
    async fn restored_articles_resolve_without_cache_and_pin_changes_leave_content_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let target = PinTarget::Entry {
            id: "restored".into(),
        };
        writer.set_pin(target.clone(), true).await.unwrap();
        let pins = Pins::read(dir.path()).await.unwrap();
        assert!(pins.list_entries(dir.path()).await.unwrap().is_empty());
        writer
            .save_article(
                crate::library_writer::Tier::Interactive,
                "restored".into(),
                "https://example.org/restored".into(),
                "Restored title".into(),
                None,
                None,
                None,
                None,
                "User body".into(),
                Default::default(),
            )
            .await
            .unwrap();
        let resolved = pins.list_entries(dir.path()).await.unwrap();
        assert_eq!(resolved[0].title, "Restored title");
        assert!(resolved[0].is_pinned);
        assert!(!library::index_path(dir.path()).exists());
        let path = library::entry_path(dir.path(), "restored");
        let before = std::fs::read(&path).unwrap();
        writer.set_pin(target, false).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(
            !crate::persistence::entries::read_entry(dir.path(), "restored")
                .await
                .unwrap()
                .is_pinned
        );
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn invalid_stores_and_targets_never_overwrite_membership() {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let path = library::pins_path(dir.path());
        for raw in [
            "pins: [broken",
            "pins: [{type: entry, id: ../outside}]",
            "pins: [{type: entry, id: x}, {type: entry, id: x}]",
            "pins: [{type: group, id: g, title: Group, children: []}]",
            "pins: [{type: entry, id: x, unexpected: true}]",
            "pins: &anchor []\ncopy: *anchor",
        ] {
            std::fs::write(&path, raw).unwrap();
            assert!(
                writer
                    .set_pin(PinTarget::Topic { id: "safe".into() }, true)
                    .await
                    .is_err(),
                "{raw}"
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        }
        std::fs::write(&path, "pins: []\n").unwrap();
        for id in ["", "../escape", "a/b", "a\\b", "x\0y"] {
            assert!(
                writer
                    .set_pin(PinTarget::Entry { id: id.into() }, true)
                    .await
                    .is_err()
            );
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "pins: []\n");
        writer.shutdown().await;
    }
}
