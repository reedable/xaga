//! Library tag definitions and resumable file/index transition records.
use noyalib::{ParserConfig, policy::DenyAnchors};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

const MAX_STORE_BYTES: usize = 12 * 1024 * 1024;

fn parser() -> ParserConfig {
    // Anchors are denied, so scalar/node budgets can follow the bounded input
    // instead of imposing the strict preset's unrelated 1,024-definition limit.
    ParserConfig::strict()
        .max_document_length(MAX_STORE_BYTES)
        .max_total_scalar_bytes(MAX_STORE_BYTES)
        .max_sequence_length(MAX_STORE_BYTES)
        .max_nodes(MAX_STORE_BYTES)
        .max_events(MAX_STORE_BYTES * 2)
        .with_policy(DenyAnchors)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

/// A dispatcher persists the pending batch before writes, retries it after a
/// crash, and advances the ordered Entry-ID cursor only after the batch commits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Progress {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<String>,
    #[serde(default)]
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub(crate) enum Transition {
    Rename {
        #[serde(default)]
        revision: u64,
        id: String,
        old_name: String,
        files: Progress,
    },
    Merge {
        #[serde(default)]
        revision: u64,
        source_id: String,
        destination_id: String,
        survivor_id: String,
        destination_name: String,
        files: Progress,
        index: Progress,
    },
    Delete {
        #[serde(default)]
        revision: u64,
        id: String,
        files: Progress,
        index: Progress,
    },
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tags {
    pub tags: Vec<Tag>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) transitions: Vec<Transition>,
}
/// The committed lookup/creation result used by the command's notification.
#[derive(Debug)]
pub struct TagCreation {
    pub tag: Tag,
    pub created: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum TagsError {
    #[error("empty tag name")]
    InvalidName,
    #[error("tag is being deleted")]
    Deleting,
    #[error("tag store IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid tag store: {0}")]
    Invalid(String),
}
impl TagsError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidName => "tags:invalid-name",
            Self::Deleting => "tags:deleting",
            _ => "tags:invalid-store",
        }
    }
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::InvalidName => "Enter a tag name.",
            Self::Deleting => "This tag is being deleted.",
            _ => "Couldn't read or save tags.",
        }
    }
}
fn invalid(message: &str) -> TagsError {
    TagsError::Invalid(message.into())
}
fn key(name: &str) -> String {
    name.trim().to_lowercase()
}
fn valid_name(name: &str) -> Result<(), TagsError> {
    if name.trim().is_empty() {
        Err(invalid("empty tag name"))
    } else {
        Ok(())
    }
}
impl Progress {
    fn validate(&self) -> Result<(), TagsError> {
        if self.pending.len() > 1024 || (self.complete && !self.pending.is_empty()) {
            return Err(invalid("invalid or oversized checkpoint"));
        }
        let mut seen = HashSet::new();
        for id in self.pending.iter().chain(self.after.iter()) {
            if !crate::persistence::entries::is_safe_id(id) {
                return Err(invalid("unsafe checkpoint ID"));
            }
        }
        for id in &self.pending {
            if !seen.insert(id) {
                return Err(invalid("duplicate pending ID"));
            }
        }
        Ok(())
    }
}
impl Tags {
    pub async fn read(dir: &Path) -> Result<Self, TagsError> {
        use tokio::io::AsyncReadExt;
        let path = crate::library::tags_path(dir);
        match tokio::fs::symlink_metadata(&path).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
            Ok(m) if !m.file_type().is_file() => {
                return Err(invalid("expected a regular tag store"));
            }
            _ => {}
        }
        let mut raw = Vec::new();
        tokio::fs::File::open(path)
            .await?
            .take((MAX_STORE_BYTES + 1) as u64)
            .read_to_end(&mut raw)
            .await?;
        if raw.len() > MAX_STORE_BYTES {
            return Err(invalid("tag store exceeds 12 MiB"));
        }
        let raw = std::str::from_utf8(&raw).map_err(|_| invalid("non-UTF-8 tag store"))?;
        let store: Self = noyalib::from_str_with_config(raw, &parser())
            .map_err(|e| TagsError::Invalid(e.to_string()))?;
        store.validate()?;
        Ok(store)
    }
    pub fn serialize(&self) -> Result<String, TagsError> {
        self.validate()?;
        let raw = noyalib::to_string_with_config(
            self,
            &noyalib::SerializerConfig::new().block_scalars(false),
        )
        .map_err(|e| TagsError::Invalid(e.to_string()))?;
        if raw.len() > MAX_STORE_BYTES {
            return Err(invalid("tag store exceeds 12 MiB"));
        }
        Ok(raw)
    }
    fn validate(&self) -> Result<(), TagsError> {
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for tag in &self.tags {
            valid_name(&tag.name)?;
            let id = uuid::Uuid::parse_str(&tag.id).map_err(|_| invalid("invalid tag UUID"))?;
            if id.get_version_num() != 4
                || id.get_variant() != uuid::Variant::RFC4122
                || id.to_string() != tag.id
                || tag.name != tag.name.trim()
                || !ids.insert(&tag.id)
                || !names.insert(key(&tag.name))
            {
                return Err(invalid("invalid or duplicate tag definition"));
            }
        }
        let mut busy = HashSet::new();
        let mut aliases = HashSet::new();
        for transition in &self.transitions {
            for id in transition.ids() {
                if !ids.contains(id) || !busy.insert(id) {
                    return Err(invalid("missing or busy transition participant"));
                }
            }
            match transition {
                Transition::Rename {
                    id,
                    old_name,
                    files,
                    ..
                } => {
                    valid_name(old_name)?;
                    let old_key = key(old_name);
                    if self
                        .tags
                        .iter()
                        .any(|t| &t.id != id && key(&t.name) == old_key)
                        || !aliases.insert(old_key)
                    {
                        return Err(invalid("reserved rename name is owned by another tag"));
                    }
                    files.validate()?;
                }
                Transition::Merge {
                    source_id,
                    destination_id,
                    survivor_id,
                    destination_name,
                    files,
                    index,
                    ..
                } => {
                    if source_id == destination_id
                        || (survivor_id != source_id && survivor_id != destination_id)
                        || self.tag(destination_id)?.name != *destination_name
                    {
                        return Err(invalid("invalid merge participants"));
                    }
                    files.validate()?;
                    index.validate()?;
                }
                Transition::Delete { files, index, .. } => {
                    files.validate()?;
                    index.validate()?;
                }
            }
        }
        Ok(())
    }
    fn tag(&self, id: &str) -> Result<&Tag, TagsError> {
        self.tags
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| invalid("unknown tag ID"))
    }
    /// Build once per snapshot; label lookup does not scan the definition list.
    pub fn resolver(&self) -> TagResolver {
        let mut names: HashMap<String, Option<String>> = self
            .tags
            .iter()
            .map(|t| (key(&t.name), Some(t.id.clone())))
            .collect();
        for transition in &self.transitions {
            match transition {
                Transition::Rename { id, old_name, .. } => {
                    names.insert(key(old_name), Some(id.clone()));
                }
                Transition::Merge {
                    source_id,
                    destination_id,
                    survivor_id,
                    ..
                } => {
                    for id in [source_id, destination_id] {
                        if let Ok(t) = self.tag(id) {
                            names.insert(key(&t.name), Some(survivor_id.clone()));
                        }
                    }
                }
                Transition::Delete { id, .. } => {
                    if let Ok(t) = self.tag(id) {
                        names.insert(key(&t.name), None);
                    }
                }
            }
        }
        TagResolver { names }
    }
    /// Offline conversion holds the library lock for its entire scan.
    pub async fn write_upgrade(
        &self,
        dir: &Path,
        _lock: &crate::fs::FileLock,
    ) -> Result<(), TagsError> {
        crate::fs::write_atomic_async(
            crate::library::tags_path(dir),
            self.serialize()?.into_bytes(),
        )
        .await?;
        Ok(())
    }
    pub fn discover(&mut self, names: &[String]) -> Result<bool, TagsError> {
        for name in names {
            valid_name(name)?;
        }
        let mut resolver = self.resolver();
        let mut changed = false;
        for name in names {
            let comparison = key(name);
            if resolver.names.contains_key(&comparison) {
                continue;
            }
            let tag = Tag {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.trim().into(),
            };
            resolver.names.insert(comparison, Some(tag.id.clone()));
            self.tags.push(tag);
            changed = true;
        }
        let observed: HashSet<_> = self.observation_ids(names).into_iter().collect();
        for transition in &mut self.transitions {
            if observed.contains(transition.ids()[0]) {
                let files = transition.files_mut();
                files.after = None;
                files.complete = false;
                if let Transition::Merge { index, .. } | Transition::Delete { index, .. } =
                    transition
                {
                    index.after = None;
                    index.complete = false;
                }
                *transition.revision_mut() = transition
                    .revision()
                    .checked_add(1)
                    .ok_or_else(|| invalid("transition revision exhausted"))?;
                changed = true;
            }
        }
        Ok(changed)
    }
    pub(crate) fn observation_ids(&self, names: &[String]) -> Vec<String> {
        self.transitions
            .iter()
            .filter(|t| match t {
                Transition::Rename { id, old_name, .. } => names
                    .iter()
                    .any(|n| key(n) == key(old_name) && n.trim() != self.tag(id).unwrap().name),
                Transition::Merge {
                    source_id,
                    destination_id,
                    destination_name,
                    ..
                } => names.iter().any(|n| {
                    (key(n) == key(&self.tag(source_id).unwrap().name)
                        || key(n) == key(&self.tag(destination_id).unwrap().name))
                        && n.trim() != destination_name
                }),
                Transition::Delete { id, .. } => names
                    .iter()
                    .any(|n| key(n) == key(&self.tag(id).unwrap().name)),
            })
            .map(|t| t.ids()[0].clone())
            .collect()
    }
    pub(crate) async fn create_write(
        dir: &Path,
        name: &str,
        _token: crate::library_writer::WriteToken,
    ) -> Result<TagCreation, TagsError> {
        valid_name(name).map_err(|_| TagsError::InvalidName)?;
        let mut store = Self::read(dir).await?;
        if let Some(existing) = store.resolver().names.get(&key(name)) {
            let id = existing.as_deref().ok_or(TagsError::Deleting)?;
            let (id, name) = store.assignment(id)?;
            return Ok(TagCreation {
                tag: Tag { id, name },
                created: false,
            });
        }
        store.discover(&[name.to_owned()])?;
        let tag = store.tags.last().unwrap().clone();
        crate::fs::write_atomic_async(
            crate::library::tags_path(dir),
            store.serialize()?.into_bytes(),
        )
        .await?;
        Ok(TagCreation { tag, created: true })
    }

    pub(crate) async fn discover_write(
        dir: &Path,
        names: &[String],
        _token: crate::library_writer::WriteToken,
    ) -> Result<Self, TagsError> {
        let mut store = Self::read(dir).await?;
        if store.discover(names)? {
            crate::fs::write_atomic_async(
                crate::library::tags_path(dir),
                store.serialize()?.into_bytes(),
            )
            .await?;
        }
        Ok(store)
    }
}
impl Transition {
    fn ids(&self) -> Vec<&String> {
        match self {
            Self::Rename { id, .. } | Self::Delete { id, .. } => vec![id],
            Self::Merge {
                source_id,
                destination_id,
                ..
            } => vec![source_id, destination_id],
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct TagResolver {
    names: HashMap<String, Option<String>>,
}
impl TagResolver {
    pub fn resolve(&self, names: &[String]) -> Result<Vec<String>, TagsError> {
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        for name in names {
            match self.names.get(&key(name)) {
                Some(Some(id)) if seen.insert(id) => ids.push(id.clone()),
                Some(_) => {}
                None => {
                    return Err(invalid(
                        "unknown tag name; recovery must register it before indexing",
                    ));
                }
            }
        }
        Ok(ids)
    }
}

/// Read-side mirror of the registry. Its file identity is checked before reuse;
/// atomic registry replacement invalidates it without reparsing per article.
#[derive(Clone)]
pub(crate) struct CachedResolver {
    dir: std::path::PathBuf,
    state: std::sync::Arc<tokio::sync::Mutex<Option<(FileIdentity, TagResolver)>>>,
}
#[derive(PartialEq, Eq)]
struct FileIdentity {
    modified: Option<std::time::SystemTime>,
    len: u64,
    #[cfg(unix)]
    inode: u64,
}
impl CachedResolver {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.into(),
            state: Default::default(),
        }
    }
    pub async fn resolve(&self, names: &[String]) -> Result<Vec<String>, TagsError> {
        if names.is_empty() {
            return Ok(Vec::new());
        }
        let mut state = self.state.lock().await;
        let identity = match tokio::fs::symlink_metadata(crate::library::tags_path(&self.dir)).await
        {
            Ok(metadata) => {
                if !metadata.file_type().is_file() {
                    return Err(invalid("expected a regular tag store"));
                }
                #[cfg(unix)]
                use std::os::unix::fs::MetadataExt;
                Some(FileIdentity {
                    modified: metadata.modified().ok(),
                    len: metadata.len(),
                    #[cfg(unix)]
                    inode: metadata.ino(),
                })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        match identity {
            None => {
                *state = None;
                TagResolver::default().resolve(names)
            }
            Some(identity) => {
                if state
                    .as_ref()
                    .is_none_or(|(previous, _)| previous != &identity)
                {
                    *state = Some((identity, Tags::read(&self.dir).await?.resolver()));
                }
                state.as_ref().unwrap().1.resolve(names)
            }
        }
    }
}

/// Storage operations used by transition dispatchers. No file/index work is
/// performed here: checkpoints follow that work in the producing command.
#[derive(Debug, Clone)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "0.14 storage foundation; production transition commands are implemented separately"
    )
)]
pub(crate) enum Mutation {
    Rename {
        id: String,
        name: String,
    },
    Merge {
        source: String,
        destination: String,
        source_count: u64,
        destination_count: u64,
    },
    Delete {
        id: String,
    },
    Checkpoint {
        id: String,
        revision: u64,
        phase: Phase,
        progress: Progress,
    },
    Finish {
        id: String,
        revision: u64,
    },
}
#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "0.14 storage foundation; production transition commands are implemented separately"
    )
)]
pub(crate) enum Phase {
    Files,
    Index,
}
impl Transition {
    fn revision(&self) -> u64 {
        match self {
            Self::Rename { revision, .. }
            | Self::Merge { revision, .. }
            | Self::Delete { revision, .. } => *revision,
        }
    }
    fn revision_mut(&mut self) -> &mut u64 {
        match self {
            Self::Rename { revision, .. }
            | Self::Merge { revision, .. }
            | Self::Delete { revision, .. } => revision,
        }
    }
    fn files_mut(&mut self) -> &mut Progress {
        match self {
            Self::Rename { files, .. } | Self::Merge { files, .. } | Self::Delete { files, .. } => {
                files
            }
        }
    }
    fn complete(&self) -> bool {
        match self {
            Self::Rename { files, .. } => files.complete,
            Self::Merge { files, index, .. } | Self::Delete { files, index, .. } => {
                files.complete && index.complete
            }
        }
    }
}
impl Tags {
    fn idle(&self, id: &str) -> Result<(), TagsError> {
        self.tag(id)?;
        if self
            .transitions
            .iter()
            .any(|t| t.ids().iter().any(|v| v.as_str() == id))
        {
            return Err(invalid("tag has an active transition"));
        }
        Ok(())
    }
    fn transition_mut(&mut self, id: &str, revision: u64) -> Result<&mut Transition, TagsError> {
        self.transitions
            .iter_mut()
            .find(|t| t.ids().iter().any(|v| v.as_str() == id) && t.revision() == revision)
            .ok_or_else(|| invalid("stale or unknown tag transition"))
    }
    /// Returns the canonical readable name for a new assignment; deletion refuses it.
    pub(crate) fn assignment(&self, id: &str) -> Result<(String, String), TagsError> {
        let tag = self.tag(id)?;
        for transition in &self.transitions {
            match transition {
                Transition::Delete { id: deleting, .. } if deleting == id => {
                    return Err(invalid("tag is deleting"));
                }
                Transition::Merge {
                    source_id,
                    destination_id,
                    survivor_id,
                    destination_name,
                    ..
                } if id == source_id || id == destination_id => {
                    return Ok((survivor_id.clone(), destination_name.clone()));
                }
                _ => {}
            }
        }
        Ok((tag.id.clone(), tag.name.clone()))
    }
    fn apply(&mut self, mutation: Mutation) -> Result<(), TagsError> {
        match mutation {
            Mutation::Rename { id, name } => {
                self.idle(&id)?;
                valid_name(&name)?;
                if self
                    .resolver()
                    .names
                    .get(&key(&name))
                    .is_some_and(|owner| owner.as_deref() != Some(id.as_str()))
                {
                    return Err(invalid("tag name is reserved"));
                }
                let tag = self.tags.iter_mut().find(|t| t.id == id).unwrap();
                let old_name = std::mem::replace(&mut tag.name, name.trim().into());
                self.transitions.push(Transition::Rename {
                    id,
                    old_name,
                    files: Progress::default(),
                    revision: 0,
                });
            }
            Mutation::Merge {
                source,
                destination,
                source_count,
                destination_count,
            } => {
                self.idle(&source)?;
                self.idle(&destination)?;
                if source == destination {
                    return Err(invalid("cannot merge a tag into itself"));
                }
                let survivor_id = if source_count > destination_count {
                    source.clone()
                } else {
                    destination.clone()
                };
                let destination_name = self.tag(&destination)?.name.clone();
                self.transitions.push(Transition::Merge {
                    source_id: source,
                    destination_id: destination,
                    survivor_id,
                    destination_name,
                    files: Progress::default(),
                    index: Progress::default(),
                    revision: 0,
                });
            }
            Mutation::Delete { id } => {
                self.idle(&id)?;
                self.transitions.push(Transition::Delete {
                    id,
                    files: Progress::default(),
                    index: Progress::default(),
                    revision: 0,
                });
            }
            Mutation::Checkpoint {
                id,
                revision,
                phase,
                progress,
            } => {
                progress.validate()?;
                let transition = self.transition_mut(&id, revision)?;
                let current = match phase {
                    Phase::Files => transition.files_mut(),
                    Phase::Index => match transition {
                        Transition::Rename { .. } => {
                            return Err(invalid("rename has no index conversion"));
                        }
                        Transition::Merge { index, .. } | Transition::Delete { index, .. } => index,
                    },
                };
                *current = progress;
                *transition.revision_mut() = revision
                    .checked_add(1)
                    .ok_or_else(|| invalid("transition revision exhausted"))?;
            }
            Mutation::Finish { id, revision } => {
                let transition = self.transition_mut(&id, revision)?.clone();
                if !transition.complete() {
                    return Err(invalid("tag conversion is incomplete"));
                }
                match &transition {
                    Transition::Rename { .. } => {}
                    Transition::Delete { id, .. } => self.tags.retain(|t| &t.id != id),
                    Transition::Merge {
                        source_id,
                        destination_id,
                        survivor_id,
                        destination_name,
                        ..
                    } => {
                        self.tags.retain(|t| {
                            (&t.id != source_id && &t.id != destination_id) || &t.id == survivor_id
                        });
                        self.tags
                            .iter_mut()
                            .find(|t| &t.id == survivor_id)
                            .unwrap()
                            .name = destination_name.clone();
                    }
                }
                self.transitions.retain(|t| t != &transition);
            }
        }
        self.validate()
    }
    pub(crate) async fn mutate_write(
        dir: &Path,
        mutation: Mutation,
        _token: crate::library_writer::WriteToken,
    ) -> Result<Self, TagsError> {
        let mut store = Self::read(dir).await?;
        store.apply(mutation)?;
        crate::fs::write_atomic_async(
            crate::library::tags_path(dir),
            store.serialize()?.into_bytes(),
        )
        .await?;
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{library, library_writer::LibraryWriter};

    fn complete() -> Progress {
        Progress {
            complete: true,
            ..Progress::default()
        }
    }
    async fn fixture(names: &[&str]) -> (tempfile::TempDir, LibraryWriter, Tags) {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let store = writer
            .discover_tags(names.iter().map(|s| s.to_string()).collect())
            .await
            .unwrap();
        (dir, writer, store)
    }
    #[tokio::test]
    async fn tag_creation_shares_discovery_identity_and_respects_reserved_names() {
        let (dir, writer, _) = fixture(&[]).await;
        let (created, discovered) = tokio::join!(
            writer.create_tag("Research".into()),
            writer.discover_tags(vec!["RESEARCH".into()])
        );
        let created = created.unwrap();
        let discovered = discovered.unwrap();
        assert_eq!(discovered.tags.len(), 1);
        assert_eq!(created.tag.id, discovered.tags[0].id);
        let id = created.tag.id;
        writer
            .mutate_tags(Mutation::Rename {
                id: id.clone(),
                name: "After".into(),
            })
            .await
            .unwrap();
        let before = tokio::fs::read(library::tags_path(dir.path()))
            .await
            .unwrap();
        let alias = writer.create_tag("Research".into()).await.unwrap();
        assert!(!alias.created);
        assert_eq!(
            alias.tag,
            Tag {
                id: id.clone(),
                name: "After".into()
            }
        );
        assert_eq!(
            tokio::fs::read(library::tags_path(dir.path()))
                .await
                .unwrap(),
            before
        );
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 0,
                phase: Phase::Files,
                progress: complete(),
            })
            .await
            .unwrap();
        writer
            .mutate_tags(Mutation::Finish {
                id: id.clone(),
                revision: 1,
            })
            .await
            .unwrap();
        writer.mutate_tags(Mutation::Delete { id }).await.unwrap();
        assert!(matches!(
            writer.create_tag("After".into()).await,
            Err(crate::library_writer::WriterError::Tags(
                TagsError::Deleting
            ))
        ));
        assert_eq!(Tags::read(dir.path()).await.unwrap().tags.len(), 1);
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_creation_comparison_concurrency_and_restart() {
        let (dir, writer, _) = fixture(&[
            "Research", "straße", "strasse", "---", "École", "école", "é", "e\u{301}",
        ])
        .await;
        let mut tasks = Vec::new();
        for _ in 0..12 {
            let writer = writer.clone();
            tasks.push(tokio::spawn(async move {
                writer
                    .discover_tags(vec![" RESEARCH ".into()])
                    .await
                    .unwrap()
            }));
        }
        for task in tasks {
            assert_eq!(task.await.unwrap().tags.len(), 7);
        }
        let store = Tags::read(dir.path()).await.unwrap();
        assert_eq!(store.tags[0].name, "Research");
        assert_eq!(
            store
                .resolver()
                .resolve(&["Research".into(), "research".into()])
                .unwrap(),
            vec![store.tags[0].id.clone()]
        );
        assert_ne!(
            store.resolver().resolve(&["straße".into()]).unwrap(),
            store.resolver().resolve(&["strasse".into()]).unwrap()
        );
        assert!(writer.discover_tags(vec!["  ".into()]).await.is_err());
        assert_eq!(Tags::read(dir.path()).await.unwrap(), store);
        writer.shutdown().await;
        assert_eq!(Tags::read(dir.path()).await.unwrap(), store);
        let isolated = tempfile::tempdir().unwrap();
        assert!(Tags::read(isolated.path()).await.unwrap().tags.is_empty());
        assert!(!library::tags_path(isolated.path()).exists());
    }
    #[tokio::test]
    async fn tag_rename_checkpoint_reservation_and_retirement() {
        let (dir, writer, store) = fixture(&["Before", "Other"]).await;
        let id = store.tags[0].id.clone();
        let renamed = writer
            .mutate_tags(Mutation::Rename {
                id: id.clone(),
                name: "After".into(),
            })
            .await
            .unwrap();
        let resolver = Tags::read(dir.path()).await.unwrap().resolver();
        assert_eq!(
            resolver
                .resolve(&["Before".into(), "After".into()])
                .unwrap(),
            vec![id.clone()]
        );
        assert!(
            writer
                .mutate_tags(Mutation::Delete { id: id.clone() })
                .await
                .is_err()
        );
        assert!(
            writer
                .mutate_tags(Mutation::Rename {
                    id: store.tags[1].id.clone(),
                    name: "Before".into()
                })
                .await
                .is_err()
        );
        assert!(
            writer
                .mutate_tags(Mutation::Finish {
                    id: id.clone(),
                    revision: 0
                })
                .await
                .is_err()
        );
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 0,
                phase: Phase::Files,
                progress: complete(),
            })
            .await
            .unwrap();
        // Newly encountered old labels invalidate completed scans and stale checkpoints.
        let observed = writer.discover_tags(vec!["Before".into()]).await.unwrap();
        assert_eq!(observed.tags.len(), renamed.tags.len());
        assert!(
            writer
                .mutate_tags(Mutation::Finish {
                    id: id.clone(),
                    revision: 1
                })
                .await
                .is_err()
        );
        assert!(
            writer
                .mutate_tags(Mutation::Finish {
                    id: id.clone(),
                    revision: 2
                })
                .await
                .is_err()
        );
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 2,
                phase: Phase::Files,
                progress: complete(),
            })
            .await
            .unwrap();
        writer
            .mutate_tags(Mutation::Finish {
                id: id.clone(),
                revision: 3,
            })
            .await
            .unwrap();
        let final_store = writer.discover_tags(vec!["Before".into()]).await.unwrap();
        assert_ne!(
            final_store.resolver().resolve(&["Before".into()]).unwrap(),
            vec![id]
        );
        writer.shutdown().await;
    }
    #[tokio::test]
    async fn tag_merge_persists_survivor_and_requires_both_phases() {
        for (source_count, destination_count, survivor_index) in [(7, 2, 0), (2, 7, 1), (2, 2, 1)] {
            let (dir, writer, store) = fixture(&["Source", "Destination"]).await;
            let source = store.tags[0].id.clone();
            let destination = store.tags[1].id.clone();
            writer
                .mutate_tags(Mutation::Merge {
                    source: source.clone(),
                    destination: destination.clone(),
                    source_count,
                    destination_count,
                })
                .await
                .unwrap();
            let restarted = Tags::read(dir.path()).await.unwrap();
            let survivor = store.tags[survivor_index].id.clone();
            assert_eq!(
                restarted
                    .resolver()
                    .resolve(&["Source".into(), "Destination".into()])
                    .unwrap(),
                vec![survivor.clone()]
            );
            assert_eq!(
                restarted.assignment(&source).unwrap(),
                (survivor.clone(), "Destination".into())
            );
            assert!(
                writer
                    .mutate_tags(Mutation::Delete { id: destination })
                    .await
                    .is_err()
            );
            writer
                .mutate_tags(Mutation::Checkpoint {
                    id: source.clone(),
                    revision: 0,
                    phase: Phase::Files,
                    progress: complete(),
                })
                .await
                .unwrap();
            assert!(
                writer
                    .mutate_tags(Mutation::Finish {
                        id: source.clone(),
                        revision: 1
                    })
                    .await
                    .is_err()
            );
            writer
                .mutate_tags(Mutation::Checkpoint {
                    id: source.clone(),
                    revision: 1,
                    phase: Phase::Index,
                    progress: complete(),
                })
                .await
                .unwrap();
            let done = writer
                .mutate_tags(Mutation::Finish {
                    id: source,
                    revision: 2,
                })
                .await
                .unwrap();
            assert_eq!(
                done.tags,
                vec![Tag {
                    id: survivor,
                    name: "Destination".into()
                }]
            );
            writer.shutdown().await;
        }
    }
    #[tokio::test]
    async fn tag_delete_reserves_name_until_cleanup_and_preserves_unrelated_state() {
        let (dir, writer, store) = fixture(&["Deleting", "Keep"]).await;
        let id = store.tags[0].id.clone();
        writer
            .mutate_tags(Mutation::Delete { id: id.clone() })
            .await
            .unwrap();
        let deleting = Tags::read(dir.path()).await.unwrap();
        assert!(deleting.assignment(&id).is_err());
        assert!(
            deleting
                .resolver()
                .resolve(&["Deleting".into()])
                .unwrap()
                .is_empty()
        );
        let observed = writer
            .discover_tags(vec!["Deleting".into(), "New".into()])
            .await
            .unwrap();
        assert_eq!(observed.tags.len(), 3);
        assert_eq!(observed.tags[1], store.tags[1]);
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 1,
                phase: Phase::Files,
                progress: complete(),
            })
            .await
            .unwrap();
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 2,
                phase: Phase::Index,
                progress: complete(),
            })
            .await
            .unwrap();
        writer
            .mutate_tags(Mutation::Finish {
                id: id.clone(),
                revision: 3,
            })
            .await
            .unwrap();
        let recreated = writer.discover_tags(vec!["Deleting".into()]).await.unwrap();
        assert_ne!(
            recreated.resolver().resolve(&["Deleting".into()]).unwrap(),
            vec![id]
        );
        writer.shutdown().await;
    }
    #[tokio::test]
    async fn tag_malformed_store_and_stale_checkpoint_are_not_overwritten() {
        let (dir, writer, store) = fixture(&["Before"]).await;
        let id = store.tags[0].id.clone();
        writer
            .mutate_tags(Mutation::Rename {
                id: id.clone(),
                name: "After".into(),
            })
            .await
            .unwrap();
        let pending = Progress {
            pending: vec!["entry-1".into()],
            ..Progress::default()
        };
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 0,
                phase: Phase::Files,
                progress: pending,
            })
            .await
            .unwrap();
        let before = tokio::fs::read(library::tags_path(dir.path()))
            .await
            .unwrap();
        assert!(
            writer
                .mutate_tags(Mutation::Checkpoint {
                    id,
                    revision: 0,
                    phase: Phase::Files,
                    progress: complete()
                })
                .await
                .is_err()
        );
        assert_eq!(
            tokio::fs::read(library::tags_path(dir.path()))
                .await
                .unwrap(),
            before
        );
        for malformed in [
            "tags: [broken",
            "tags: []\nunknown: true\n",
            "tags: &t []\n",
            "tags: [{id: bad, name: test}]\n",
        ] {
            tokio::fs::write(library::tags_path(dir.path()), malformed)
                .await
                .unwrap();
            assert!(writer.discover_tags(vec!["New".into()]).await.is_err());
            assert_eq!(
                tokio::fs::read_to_string(library::tags_path(dir.path()))
                    .await
                    .unwrap(),
                malformed
            );
        }
        writer.shutdown().await;
    }
    #[tokio::test]
    async fn tag_late_discovery_invalidates_file_and_index_completion() {
        let (_dir, writer, store) = fixture(&["Old", "Destination"]).await;
        let id = store.tags[0].id.clone();
        writer
            .mutate_tags(Mutation::Merge {
                source: id.clone(),
                destination: store.tags[1].id.clone(),
                source_count: 2,
                destination_count: 1,
            })
            .await
            .unwrap();
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 0,
                phase: Phase::Files,
                progress: complete(),
            })
            .await
            .unwrap();
        writer
            .mutate_tags(Mutation::Checkpoint {
                id: id.clone(),
                revision: 1,
                phase: Phase::Index,
                progress: complete(),
            })
            .await
            .unwrap();
        let observed = writer.discover_tags(vec!["Old".into()]).await.unwrap();
        assert!(
            matches!(&observed.transitions[0], Transition::Merge {files, index, revision: 3, ..} if !files.complete && !index.complete)
        );
        assert!(
            writer
                .mutate_tags(Mutation::Finish { id, revision: 3 })
                .await
                .is_err()
        );
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn tag_registry_round_trips_more_than_strict_default_sequence_limit() {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let names = (0..1025).map(|i| format!("tag-{i}")).collect();
        let stored = writer.discover_tags(names).await.unwrap();
        assert_eq!(Tags::read(dir.path()).await.unwrap(), stored);
        writer.shutdown().await;
    }

    #[tokio::test]
    #[ignore = "measures registry parsing and lookup independently of article corpus size"]
    async fn tag_registry_snapshot_costs() {
        use std::time::Instant;
        let dir = tempfile::tempdir().unwrap();
        let lock = library::acquire_lock(dir.path()).unwrap();
        let mut store = Tags::default();
        let names: Vec<String> = (0..10_000).map(|i| format!("tag-{i}")).collect();
        let started = Instant::now();
        store.discover(&names).unwrap();
        store.write_upgrade(dir.path(), &lock).await.unwrap();
        println!(
            "tag_registry definitions=10000 write_ms={} bytes={}",
            started.elapsed().as_millis(),
            store.serialize().unwrap().len()
        );
        let started = Instant::now();
        let loaded = Tags::read(dir.path()).await.unwrap();
        let resolver = loaded.resolver();
        println!(
            "tag_registry read_and_resolver_ms={}",
            started.elapsed().as_millis()
        );
        let started = Instant::now();
        for i in 0..2_000_000 {
            let resolved = resolver.resolve(&[names[i % names.len()].clone()]).unwrap();
            assert_eq!(resolved, vec![store.tags[i % store.tags.len()].id.clone()]);
        }
        println!(
            "tag_registry lookups=2000000 resolve_ms={}",
            started.elapsed().as_millis()
        );
    }

    #[tokio::test]
    async fn tag_cached_resolution_follows_atomic_registry_replacements() {
        let (dir, writer, store) = fixture(&["Before"]).await;
        let cached = CachedResolver::new(dir.path());
        assert_eq!(
            cached.resolve(&["Before".into()]).await.unwrap(),
            vec![store.tags[0].id.clone()]
        );
        writer
            .mutate_tags(Mutation::Rename {
                id: store.tags[0].id.clone(),
                name: "After".into(),
            })
            .await
            .unwrap();
        assert_eq!(
            cached.resolve(&["After".into()]).await.unwrap(),
            vec![store.tags[0].id.clone()]
        );
        writer.shutdown().await;
    }
}
