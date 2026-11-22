//! Validated reads and serialized, incremental identity repair.
use super::entries::EntryError;
use crate::library_writer::LibraryWriter;
use std::path::Path;

/// Shared saved-ID/filename-index contract for every persisted entity format.
pub(crate) fn resolve(saved: Option<&str>, path: &Path) -> Result<String, String> {
    let indexed = crate::util::entity_id(path);
    if !super::entries::is_safe_id(&indexed) {
        return Err("invalid filename identity".into());
    }
    if let Some(saved) = saved {
        if saved != indexed {
            log::warn!(
                "persisted identity mismatch: {}: filename={indexed}, saved={saved}",
                path.display()
            );
            return Err(format!(
                "identity mismatch: filename={indexed}, saved={saved}"
            ));
        }
        return Ok(saved.to_owned());
    }
    Ok(indexed)
}

#[derive(Clone, Copy)]
pub(crate) enum Record {
    Entry,
    Topic,
    Feed,
}
impl Record {
    pub(crate) fn lane(self, path: &Path) -> String {
        let kind = match self {
            Self::Entry => "entry",
            Self::Topic => "topic",
            Self::Feed => "feed",
        };
        format!("{kind}/{}", crate::util::entity_id(path))
    }
}

pub(crate) async fn read_markdown(
    path: &Path,
    writer: Option<&LibraryWriter>,
    kind: Record,
) -> Result<String, EntryError> {
    let raw = crate::fs::read_md_capped(path).await?;
    let (fm, _) = crate::md::parse_indexed(&raw, path)?;
    if fm.id.is_none() {
        if let Some(writer) = writer {
            if let Err(error) = writer.repair_identity(path.to_owned(), kind).await {
                log::warn!(
                    "identity repair not saved: {}: {}",
                    path.display(),
                    error.code()
                );
                // A stale read must not hide a subsequently conflicting identity.
                if let Ok(current) = crate::fs::read_md_capped(path).await {
                    crate::md::parse_indexed(&current, path)?;
                }
                return Ok(raw);
            }
            let current = crate::fs::read_md_capped(path).await?;
            crate::md::parse_indexed(&current, path)?;
            return Ok(current);
        }
    }
    Ok(raw)
}

pub(crate) async fn read_atom(
    path: &Path,
    writer: Option<&LibraryWriter>,
) -> Result<crate::atom::Atom, super::feeds::FeedError> {
    use super::feeds::FeedError;
    let raw = tokio::fs::read_to_string(path).await?;
    let mut atom = crate::atom::parse_atom(&raw).map_err(FeedError::AtomParse)?;
    let missing = crate::atom::ensure_local_id(&mut atom, path).map_err(FeedError::AtomParse)?;
    if missing {
        if let Some(writer) = writer {
            if let Err(error) = writer.repair_identity(path.to_owned(), Record::Feed).await {
                log::warn!(
                    "identity repair not saved: {}: {}",
                    path.display(),
                    error.code()
                );
            }
            // Observe any concurrent rename/refresh without restoring the old snapshot.
            if let Ok(raw) = tokio::fs::read_to_string(path).await {
                atom = crate::atom::parse_atom(&raw).map_err(FeedError::AtomParse)?;
                crate::atom::ensure_local_id(&mut atom, path).map_err(FeedError::AtomParse)?;
            }
        }
    }
    Ok(atom)
}

pub(crate) async fn repair(
    library: &Path,
    path: &Path,
    kind: Record,
    token: crate::library_writer::WriteToken,
) -> Result<(), crate::library_writer::WriterError> {
    let id = crate::util::entity_id(path);
    let expected = match kind {
        Record::Entry => crate::library::resolve_entry_path(library, &id)
            .await
            .map_err(EntryError::from)?,
        Record::Topic => Some(crate::library::topic_path(library, &id)),
        Record::Feed => Some(crate::library::feed_path(library, &id)),
    };
    if !super::entries::is_safe_id(&id) || expected.as_deref() != Some(path) {
        return Err(EntryError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "identity repair target is outside the writer's selected library record",
        ))
        .into());
    }
    match kind {
        Record::Entry | Record::Topic => super::documents::repair_identity(path, token)
            .await
            .map_err(Into::into),
        Record::Feed => {
            let raw = tokio::fs::read_to_string(path)
                .await
                .map_err(super::feeds::FeedError::from)?;
            let mut atom =
                crate::atom::parse_atom(&raw).map_err(super::feeds::FeedError::AtomParse)?;
            let created_at = crate::atom::feed_created_at(&atom).map(str::to_owned);
            if crate::atom::ensure_local_id(&mut atom, path)
                .map_err(super::feeds::FeedError::AtomParse)?
            {
                super::feeds::preserve_created_at(path, created_at.as_deref(), &mut atom)?;
                let bytes =
                    crate::atom::atom_to_xml(&atom).map_err(super::feeds::FeedError::AtomParse)?;
                crate::fs::write_atomic_async(path.to_owned(), bytes)
                    .await
                    .map_err(super::feeds::FeedError::from)?;
            }
            Ok(())
        }
    }
}
