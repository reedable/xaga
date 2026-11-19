//! Shared validated discovery. Filenames index saved identity; rejected records
//! make discovery unreliable for absence-based deletion.
use crate::{fs::WalkOutcome, library_writer::LibraryWriter, md::Markdown};
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

pub(crate) async fn for_each_md_with_writer<T, P, V>(
    dir: &Path,
    parse: &P,
    visit: V,
    writer: Option<&LibraryWriter>,
    kind: super::identity::Record,
) -> WalkOutcome
where
    P: Fn(Markdown) -> T,
    V: FnMut(T),
{
    walk_markdown(dir, parse, visit, writer, kind, None).await
}

async fn walk_markdown<T, P, V>(
    dir: &Path,
    parse: &P,
    visit: V,
    writer: Option<&LibraryWriter>,
    kind: super::identity::Record,
    library: Option<&Path>,
) -> WalkOutcome
where
    P: Fn(Markdown) -> T,
    V: FnMut(T),
{
    let rejected = AtomicBool::new(false);
    let visit = Mutex::new(visit);
    let outcome = crate::fs::for_each_file_async(dir, "md", |path, raw| {
        let rejected = &rejected;
        let visit = &visit;
        async move {
            let result = async {
                if let Some(library) = library {
                    let id = crate::util::entity_id(&path);
                    if crate::library::resolve_entry_path(library, &id)
                        .await?
                        .as_ref()
                        != Some(&path)
                    {
                        return Ok(None);
                    }
                }
                let (fm, _) = crate::md::parse_indexed(&raw, &path)?;
                let raw = if fm.id.is_none() && writer.is_some() {
                    super::identity::read_markdown(&path, writer, kind).await?
                } else {
                    raw
                };
                let (fm, body) = crate::md::parse_indexed(&raw, &path)?;
                let id = crate::md::resolve_identity(&fm, &path)?;
                Ok::<_, super::entries::EntryError>(Some(Markdown { id, fm, body }))
            }
            .await;
            match result {
                Ok(Some(md)) => visit.lock().unwrap()(parse(md)),
                Ok(None) => {}
                Err(_) => {
                    // The identity resolver reports conflicts without document content.
                    rejected.store(true, Ordering::Relaxed);
                }
            }
        }
    })
    .await;
    if rejected.load(Ordering::Relaxed) {
        WalkOutcome::Unreadable
    } else {
        outcome
    }
}

pub(crate) async fn for_each_atom_with_writer<E, P, V>(
    dir: &Path,
    parse: &P,
    visit: V,
    writer: Option<&LibraryWriter>,
) -> WalkOutcome
where
    P: Fn(&str) -> Result<crate::atom::Atom, E>,
    V: FnMut(PathBuf, crate::atom::Atom),
{
    let rejected = AtomicBool::new(false);
    let visit = Mutex::new(visit);
    let outcome = crate::fs::for_each_file_async(dir, "atom", |path, raw| {
        let rejected = &rejected;
        let visit = &visit;
        async move {
            let result = async {
                let mut atom = parse(&raw).map_err(|_| "malformed Atom".to_owned())?;
                let missing = crate::atom::ensure_local_id(&mut atom, &path)?;
                if missing && writer.is_some() {
                    atom = super::identity::read_atom(&path, writer)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Ok::<_, String>(atom)
            }
            .await;
            match result {
                Ok(atom) => visit.lock().unwrap()(path, atom),
                Err(_) => {
                    // The identity resolver reports conflicts without document content.
                    rejected.store(true, Ordering::Relaxed);
                }
            }
        }
    })
    .await;
    if rejected.load(Ordering::Relaxed) {
        WalkOutcome::Unreadable
    } else {
        outcome
    }
}

/// Discover each filename index once, including PDF companions. A rejected
/// canonical record never authorizes falling back to a competing companion.
pub(crate) async fn for_each_entry<T, P, V>(
    library: &Path,
    parse: &P,
    visit: V,
    writer: Option<&LibraryWriter>,
) -> WalkOutcome
where
    P: Fn(Markdown) -> T,
    V: FnMut(T),
{
    let mut visit = visit;
    let mut outcome = WalkOutcome::Enumerated;
    for dir in crate::library::entry_scan_dirs(library) {
        let current = walk_markdown(
            &dir,
            parse,
            &mut visit,
            writer,
            super::identity::Record::Entry,
            Some(library),
        )
        .await;
        if current == WalkOutcome::Unreadable {
            outcome = current;
        }
    }
    outcome
}
