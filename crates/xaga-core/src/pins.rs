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
    Site { hostname: String },
}

impl PinTarget {
    /// Canonical identity at every persistence/command boundary.
    pub fn normalized(mut self) -> Result<Self, PinsError> {
        self.validate()?;
        if let Self::Site { hostname } = &mut self {
            *hostname =
                crate::site::normalize_pin_hostname(hostname).ok_or(PinsError::InvalidTarget)?;
        }
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), PinsError> {
        match self {
            Self::Site { hostname } => crate::site::normalize_pin_hostname(hostname)
                .map(|_| ())
                .ok_or(PinsError::InvalidTarget),
            Self::Entry { id } | Self::Feed { id } | Self::Topic { id } => {
                if crate::persistence::entries::is_safe_id(id) {
                    Ok(())
                } else {
                    Err(PinsError::InvalidTarget)
                }
            }
        }
    }
}

/// Ordered navigation nodes; target identity is independent of group placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum PinNode {
    Group {
        id: String,
        title: String,
        children: Vec<PinNode>,
    },
    #[serde(untagged)]
    Target(PinTarget),
}

impl From<PinTarget> for PinNode {
    fn from(target: PinTarget) -> Self {
        Self::Target(target)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum PinEdit {
    CreateGroup {
        id: String,
        title: String,
        parent: Option<String>,
    },
    RenameGroup {
        id: String,
        title: String,
    },
    RemoveGroup {
        id: String,
    },
    Move {
        node: PinNodeRef,
        parent: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum PinNodeRef {
    Group {
        id: String,
    },
    #[serde(untagged)]
    Target(PinTarget),
}

fn matches_node(node: &PinNode, reference: &PinNodeRef) -> bool {
    match (node, reference) {
        (PinNode::Group { id, .. }, PinNodeRef::Group { id: other }) => id == other,
        (PinNode::Target(target), PinNodeRef::Target(other)) => target == other,
        _ => false,
    }
}

fn container<'a>(
    nodes: &'a mut Vec<PinNode>,
    parent: Option<&str>,
) -> Option<&'a mut Vec<PinNode>> {
    let Some(parent) = parent else {
        return Some(nodes);
    };
    for node in nodes {
        if let PinNode::Group { id, children, .. } = node {
            if id == parent {
                return Some(children);
            }
            if let Some(found) = container(children, Some(parent)) {
                return Some(found);
            }
        }
    }
    None
}

fn take_node(nodes: &mut Vec<PinNode>, reference: &PinNodeRef, promote: bool) -> Option<PinNode> {
    if let Some(index) = nodes.iter().position(|node| matches_node(node, reference)) {
        let node = nodes.remove(index);
        if promote {
            if let PinNode::Group { children, .. } = &node {
                nodes.splice(index..index, children.iter().cloned());
            }
        }
        return Some(node);
    }
    for node in nodes {
        if let PinNode::Group { children, .. } = node {
            if let Some(found) = take_node(children, reference, promote) {
                return Some(found);
            }
        }
    }
    None
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pins {
    pub pins: Vec<PinNode>,
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
        fn visit(
            nodes: &[PinNode],
            depth: usize,
            targets: &mut HashSet<PinTarget>,
            groups: &mut HashSet<String>,
        ) -> Result<(), PinsError> {
            // Strict YAML already bounds structural depth to 64. Bound typed IPC
            // and constructed trees as well, before recursive encoding/traversal.
            if depth > 24 {
                return Err(PinsError::Format("groups exceed 24 levels".into()));
            }
            for node in nodes {
                match node {
                    PinNode::Target(target) => {
                        target.validate()?;
                        if !targets.insert(target.clone()) {
                            return Err(PinsError::Format("duplicate target".into()));
                        }
                    }
                    PinNode::Group {
                        id,
                        title,
                        children,
                    } => {
                        if !crate::persistence::entries::is_safe_id(id)
                            || title.trim().is_empty()
                            || !groups.insert(id.clone())
                        {
                            return Err(PinsError::Format("invalid or duplicate group".into()));
                        }
                        visit(children, depth + 1, targets, groups)?;
                    }
                }
            }
            Ok(())
        }
        visit(&self.pins, 0, &mut HashSet::new(), &mut HashSet::new())
    }

    fn normalize(&mut self) -> Result<(), PinsError> {
        fn visit(nodes: &mut [PinNode]) -> Result<(), PinsError> {
            for node in nodes {
                match node {
                    PinNode::Target(target) => *target = target.clone().normalized()?,
                    PinNode::Group {
                        title, children, ..
                    } => {
                        *title = title.trim().into();
                        visit(children)?;
                    }
                }
            }
            Ok(())
        }
        self.validate()?;
        visit(&mut self.pins)?;
        self.validate()
    }

    /// Flatten only target references for membership, recovery and cache projection.
    pub fn targets(&self) -> Vec<PinTarget> {
        fn visit(nodes: &[PinNode], result: &mut Vec<PinTarget>) {
            for node in nodes {
                match node {
                    PinNode::Target(target) => result.push(target.clone()),
                    PinNode::Group { children, .. } => visit(children, result),
                }
            }
        }
        let mut targets = Vec::new();
        visit(&self.pins, &mut targets);
        targets
    }

    fn edit(&mut self, edit: PinEdit) -> Result<(), PinsError> {
        let invalid = || PinsError::Format("group or destination unavailable".into());
        match edit {
            PinEdit::CreateGroup { id, title, parent } => {
                container(&mut self.pins, parent.as_deref())
                    .ok_or_else(invalid)?
                    .push(PinNode::Group {
                        id,
                        title,
                        children: vec![],
                    });
            }
            PinEdit::RenameGroup { id, title } => {
                fn rename(nodes: &mut [PinNode], id: &str, title: &str) -> bool {
                    for node in nodes {
                        if let PinNode::Group {
                            id: current,
                            title: current_title,
                            children,
                        } = node
                        {
                            if current == id {
                                *current_title = title.into();
                                return true;
                            }
                            if rename(children, id, title) {
                                return true;
                            }
                        }
                    }
                    false
                }
                if !rename(&mut self.pins, &id, &title) {
                    return Err(invalid());
                }
            }
            PinEdit::RemoveGroup { id } => {
                take_node(&mut self.pins, &PinNodeRef::Group { id }, true).ok_or_else(invalid)?;
            }
            PinEdit::Move { node, parent } => {
                let node = match node {
                    PinNodeRef::Target(target) => PinNodeRef::Target(target.normalized()?),
                    group => group,
                };
                // Removing first excludes both the group itself and its descendants
                // from valid destinations. An error never reaches the file write.
                let moved = take_node(&mut self.pins, &node, false).ok_or_else(invalid)?;
                container(&mut self.pins, parent.as_deref())
                    .ok_or_else(invalid)?
                    .push(moved);
            }
        }
        self.normalize()
    }

    pub(crate) async fn organize(
        dir: &Path,
        edit: PinEdit,
        _token: crate::library_writer::WriteToken,
    ) -> Result<Self, PinsError> {
        let mut store = Self::read(dir).await?;
        store.edit(edit)?;
        let yaml = store.serialize()?;
        crate::fs::write_atomic_async(crate::library::pins_path(dir), yaml.into_bytes()).await?;
        Ok(store)
    }

    /// Validated encoding shared with the explicit offline converter.
    pub fn serialize(&self) -> Result<String, PinsError> {
        let mut normalized = self.clone();
        normalized.normalize()?;
        let yaml = noyalib::to_string_with_config(
            &normalized,
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
        let mut store: Self =
            noyalib::from_str_with_config(raw, &ParserConfig::strict().with_policy(DenyAnchors))
                .map_err(|e| PinsError::Format(e.to_string()))?;
        store.normalize()?;
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
        for target in &self.targets() {
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
        self.targets().contains(target)
    }

    pub(crate) async fn set(
        dir: &Path,
        target: PinTarget,
        pinned: bool,
        _token: crate::library_writer::WriteToken,
    ) -> Result<Self, PinsError> {
        let target = target.normalized()?;
        let mut store = Self::read(dir).await?;
        if store.contains(&target) == pinned {
            return Ok(store);
        }
        if pinned {
            store.pins.push(target.into());
        } else {
            take_node(&mut store.pins, &PinNodeRef::Target(target), false);
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
    async fn groups_move_promote_and_unpin_through_the_serialized_writer() {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let target = PinTarget::Entry {
            id: "article".into(),
        };
        writer.set_pin(target.clone(), true).await.unwrap();
        for (id, parent) in [("outer", None), ("inner", Some("outer"))] {
            writer
                .organize_pins(PinEdit::CreateGroup {
                    id: id.into(),
                    title: " Research ".into(),
                    parent: parent.map(str::to_owned),
                })
                .await
                .unwrap();
        }
        writer
            .organize_pins(PinEdit::Move {
                node: PinNodeRef::Target(target.clone()),
                parent: Some("inner".into()),
            })
            .await
            .unwrap();
        writer.set_pin(target.clone(), true).await.unwrap();
        let path = library::pins_path(dir.path());
        let before = std::fs::read(&path).unwrap();
        assert!(
            writer
                .organize_pins(PinEdit::Move {
                    node: PinNodeRef::Group { id: "outer".into() },
                    parent: Some("inner".into())
                })
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(
            writer
                .organize_pins(PinEdit::CreateGroup {
                    id: "inner".into(),
                    title: "Duplicate ID".into(),
                    parent: None
                })
                .await
                .is_err()
        );
        assert!(
            writer
                .organize_pins(PinEdit::RenameGroup {
                    id: "inner".into(),
                    title: "  ".into()
                })
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        writer
            .organize_pins(PinEdit::RenameGroup {
                id: "inner".into(),
                title: " Renamed ".into(),
            })
            .await
            .unwrap();
        writer
            .organize_pins(PinEdit::RemoveGroup { id: "outer".into() })
            .await
            .unwrap();
        let store = Pins::read(dir.path()).await.unwrap();
        assert_eq!(store.targets(), vec![target.clone()]);
        assert!(
            matches!(&store.pins[0], PinNode::Group { id, title, .. } if id == "inner" && title == "Renamed")
        );
        writer.set_pin(target, false).await.unwrap();
        assert!(Pins::read(dir.path()).await.unwrap().targets().is_empty());
        writer.shutdown().await;
        assert!(Pins::read(dir.path()).await.unwrap().targets().is_empty());
    }

    #[test]
    fn removing_groups_promotes_children_in_place_without_changing_other_nodes() {
        let target = |id: &str| PinNode::Target(PinTarget::Entry { id: id.into() });
        let mut pins = Pins {
            pins: vec![
                target("before"),
                PinNode::Group {
                    id: "outer".into(),
                    title: "Group".into(),
                    children: vec![
                        target("first"),
                        PinNode::Group {
                            id: "nested".into(),
                            title: "Group".into(),
                            children: vec![target("second")],
                        },
                        target("last"),
                    ],
                },
                target("after"),
            ],
        };
        pins.edit(PinEdit::RemoveGroup { id: "outer".into() })
            .unwrap();
        assert_eq!(
            pins.pins,
            vec![
                target("before"),
                target("first"),
                PinNode::Group {
                    id: "nested".into(),
                    title: "Group".into(),
                    children: vec![target("second")]
                },
                target("last"),
                target("after")
            ]
        );
        pins.edit(PinEdit::RemoveGroup {
            id: "nested".into(),
        })
        .unwrap();
        assert_eq!(
            pins.pins,
            ["before", "first", "second", "last", "after"]
                .map(target)
                .to_vec()
        );
    }

    #[tokio::test]
    async fn group_edits_and_flat_pins_share_one_lane_without_lost_updates() {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let mut tasks = vec![];
        for i in 0..12 {
            let writer = writer.clone();
            tasks.push(tokio::spawn(async move {
                writer
                    .organize_pins(PinEdit::CreateGroup {
                        id: format!("g{i}"),
                        title: "Group".into(),
                        parent: None,
                    })
                    .await
                    .unwrap();
                let target = PinTarget::Topic {
                    id: format!("t{i}"),
                };
                writer.set_pin(target.clone(), true).await.unwrap();
                writer
                    .organize_pins(PinEdit::Move {
                        node: PinNodeRef::Target(target),
                        parent: Some(format!("g{i}")),
                    })
                    .await
                    .unwrap();
            }));
        }
        for task in tasks {
            task.await.unwrap();
        }
        let store = Pins::read(dir.path()).await.unwrap();
        assert_eq!(store.pins.len(), 12);
        assert_eq!(store.targets().len(), 12);
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn nested_validation_rejects_duplicate_targets_groups_and_excessive_depth() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(library::pins_path(dir.path()).parent().unwrap()).unwrap();
        for raw in [
            "pins: [{type: group, id: g, title: G, children: [{type: entry, id: a}]}, {type: entry, id: a}]",
            "pins: [{type: group, id: g, title: G, children: [{type: group, id: g, title: G, children: []}]}]",
            "pins: [{type: group, id: ../g, title: G, children: []}]",
            "pins: [{type: group, id: g, title: G, children: [], extra: true}]",
        ] {
            std::fs::write(library::pins_path(dir.path()), raw).unwrap();
            assert!(Pins::read(dir.path()).await.is_err(), "{raw}");
        }
        let mut nodes = vec![];
        for depth in 0..25 {
            nodes = vec![PinNode::Group {
                id: format!("g{depth}"),
                title: "G".into(),
                children: nodes,
            }];
        }
        assert!(
            Pins {
                pins: nodes.clone()
            }
            .serialize()
            .is_err()
        );
        let raw = serde_json::to_string(&Pins { pins: nodes }).unwrap();
        std::fs::write(library::pins_path(dir.path()), raw).unwrap();
        assert!(Pins::read(dir.path()).await.is_err());
    }

    #[tokio::test]
    async fn site_pins_normalize_persist_and_isolate_without_entity_files() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            first.path().into(),
            library::acquire_lock(first.path()).unwrap(),
        );
        for hostname in ["Example.COM.", "example.com"] {
            writer
                .set_pin(
                    PinTarget::Site {
                        hostname: hostname.into(),
                    },
                    true,
                )
                .await
                .unwrap();
        }
        let pins = Pins::read(first.path()).await.unwrap();
        assert_eq!(
            pins.targets(),
            vec![PinTarget::Site {
                hostname: "example.com".into()
            }]
        );
        assert_eq!(Pins::read(second.path()).await.unwrap(), Pins::default());
        assert!(!library::index_path(first.path()).exists());
        assert!(!library::entries_dir(first.path()).exists());
        for hostname in [
            "",
            "../escape",
            "https://example.com",
            "example.com/path",
            "user@example.com",
            "127.0.0.1",
            "[::1]",
            "bad host",
            "example.com:80",
            "bad%2eexample",
            "-bad.example",
        ] {
            assert!(
                writer
                    .set_pin(
                        PinTarget::Site {
                            hostname: hostname.into()
                        },
                        true
                    )
                    .await
                    .is_err(),
                "{hostname}"
            );
        }
        assert_eq!(Pins::read(first.path()).await.unwrap(), pins);
        writer.shutdown().await;
        assert_eq!(Pins::read(first.path()).await.unwrap(), pins);
    }

    #[tokio::test]
    async fn site_identity_with_www_survives_repeated_reads_and_writes() {
        let dir = tempfile::tempdir().unwrap();
        let hostname = crate::site::normalize_site_hostname("https://www.www.example.com/article")
            .unwrap()
            .to_string();
        assert_eq!(hostname, "www.example.com");
        let target = PinTarget::Site { hostname };
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        writer.set_pin(target.clone(), true).await.unwrap();
        writer
            .set_pin(
                PinTarget::Site {
                    hostname: "other.example".into(),
                },
                true,
            )
            .await
            .unwrap();
        for _ in 0..3 {
            let pins = Pins::read(dir.path()).await.unwrap();
            assert!(pins.contains(&target));
            std::fs::write(library::pins_path(dir.path()), pins.serialize().unwrap()).unwrap();
        }
        writer.set_pin(target.clone(), false).await.unwrap();
        assert!(!Pins::read(dir.path()).await.unwrap().contains(&target));
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn concurrent_site_changes_share_the_pin_lane() {
        let dir = tempfile::tempdir().unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let mut handles = Vec::new();
        for i in 0..12 {
            let writer = writer.clone();
            handles.push(tokio::spawn(async move {
                writer
                    .set_pin(
                        PinTarget::Site {
                            hostname: format!("site{i}.example"),
                        },
                        true,
                    )
                    .await
                    .unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        let pins = Pins::read(dir.path()).await.unwrap();
        assert_eq!(pins.pins.len(), 12);
        writer
            .set_pin(
                PinTarget::Site {
                    hostname: "site0.example".into(),
                },
                false,
            )
            .await
            .unwrap();
        assert_eq!(Pins::read(dir.path()).await.unwrap().pins.len(), 11);
        writer.shutdown().await;
    }

    #[tokio::test]
    async fn site_metadata_rejects_normalized_duplicates_and_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(library::pins_path(dir.path()).parent().unwrap()).unwrap();
        for raw in [
            "pins: [{type: site, hostname: example.com}, {type: site, hostname: EXAMPLE.COM.}]",
            "pins: [{type: site, id: example.com}]",
            "pins: [{type: site, hostname: example.com, id: x}]",
            "pins: [{type: site, hostname: ../escape}]",
        ] {
            std::fs::write(library::pins_path(dir.path()), raw).unwrap();
            assert!(Pins::read(dir.path()).await.is_err(), "{raw}");
            assert_eq!(
                std::fs::read_to_string(library::pins_path(dir.path())).unwrap(),
                raw
            );
        }
    }

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
        assert_eq!(store.targets(), vec![entry.clone(), feed.clone()]);
        let path = library::pins_path(dir.path());
        let raw = std::fs::read_to_string(&path).unwrap();
        let commented = format!("# user comment\n{raw}");
        std::fs::write(&path, &commented).unwrap();
        writer.set_pin(entry.clone(), true).await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), commented);
        writer.set_pin(entry.clone(), false).await.unwrap();
        writer.shutdown().await;
        assert_eq!(Pins::read(dir.path()).await.unwrap().targets(), vec![feed]);
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
        std::fs::create_dir_all(library::pins_path(dir.path()).parent().unwrap()).unwrap();
        let writer = LibraryWriter::start(
            dir.path().into(),
            library::acquire_lock(dir.path()).unwrap(),
        );
        let path = library::pins_path(dir.path());
        for raw in [
            "pins: [broken",
            "pins: [{type: entry, id: ../outside}]",
            "pins: [{type: entry, id: x}, {type: entry, id: x}]",
            "pins: [{type: group, id: g, title: ' ' , children: []}]",
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
