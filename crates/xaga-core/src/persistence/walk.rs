//! Shared validated discovery. Filenames index saved identity; rejected records
//! make discovery unreliable for absence-based deletion.
use super::identity::{self, Record};
use crate::{
    fs::WalkOutcome,
    library_writer::LibraryWriter,
    md::{Frontmatter, Markdown, MdError},
};
use futures::StreamExt;
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

fn warn_parse_failure(operation: &str, path: &Path, diagnostic: &str) {
    log::warn!("{operation}: skipped malformed persisted record at {path:?}: {diagnostic}");
}

fn parse_markdown(
    raw: &str,
    path: &Path,
    operation: &str,
) -> Result<(Frontmatter, String), MdError> {
    let (fm, body) =
        crate::md::parse_document_with_diagnostic(raw).map_err(|(error, diagnostic)| {
            warn_parse_failure(operation, path, &diagnostic);
            error
        })?;
    crate::md::resolve_identity(&fm, path)?;
    Ok((fm, body))
}

pub(crate) async fn read_entry(
    path: &Path,
    writer: Option<&LibraryWriter>,
    operation: &str,
) -> Result<String, super::entries::EntryError> {
    identity::read_markdown_with(path, writer, Record::Entry, |raw| {
        parse_markdown(raw, path, operation)
    })
    .await
}

pub(crate) async fn for_each_md_with_writer<T, P, V>(
    dir: &Path,
    parse: &P,
    visit: V,
    writer: Option<&LibraryWriter>,
    kind: Record,
    operation: &str,
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
                let parse = |raw: &str| parse_markdown(raw, &path, operation);
                let (fm, _) = parse(&raw)?;
                let raw = if fm.id.is_none() && writer.is_some() {
                    identity::read_markdown_with(&path, writer, kind, parse).await?
                } else {
                    raw
                };
                let (fm, body) = crate::md::parse_indexed(&raw, &path)?;
                let id = crate::md::resolve_identity(&fm, &path)?;
                Ok::<_, super::entries::EntryError>(Markdown { id, fm, body })
            }
            .await;
            match result {
                Ok(md) => visit.lock().unwrap()(parse(md)),
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

pub(super) async fn for_each_atom_with_writer<V>(
    dir: &Path,
    operation: &str,
    visit: V,
    writer: Option<&LibraryWriter>,
) -> WalkOutcome
where
    V: FnMut(PathBuf, crate::atom::Atom),
{
    let rejected = AtomicBool::new(false);
    let visit = Mutex::new(visit);
    let outcome = crate::fs::for_each_file_async(dir, "atom", |path, raw| {
        let rejected = &rejected;
        let visit = &visit;
        async move {
            let result = async {
                let parse = |raw: &str| {
                    crate::atom::parse_atom_with_diagnostic(raw).map_err(|(error, diagnostic)| {
                        warn_parse_failure(operation, &path, &diagnostic);
                        error
                    })
                };
                let mut atom = parse(&raw)?;
                let missing = crate::atom::ensure_local_id(&mut atom, &path)?;
                if missing && writer.is_some() {
                    atom = identity::read_atom_with(&path, writer, parse)
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
    let paths = crate::library::scan_entry_paths(library);
    futures::pin_mut!(paths);
    while let Some(path) = paths.next().await {
        let result = async {
            let path = path?;
            let raw = read_entry(&path, writer, "Entry discovery").await?;
            let (fm, body) = crate::md::parse_indexed(&raw, &path)?;
            let id = crate::md::resolve_identity(&fm, &path)?;
            Ok::<_, super::entries::EntryError>(Markdown { id, fm, body })
        }
        .await;
        match result {
            Ok(markdown) => visit(parse(markdown)),
            Err(_) => outcome = WalkOutcome::Unreadable,
        }
    }
    outcome
}
