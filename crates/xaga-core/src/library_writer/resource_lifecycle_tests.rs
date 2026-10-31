use super::*;
use crate::persistence::images;
use base64::Engine;
use std::collections::HashMap;

const PNG: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/pLvAAAAAElFTkSuQmCC";
const ICON: &str = "https://example.org/icon.png";

fn start(dir: &std::path::Path) -> LibraryWriter {
    LibraryWriter::start(dir.into(), crate::library::acquire_lock(dir).unwrap())
}

async fn save(writer: &LibraryWriter, id: &str, icon: &str, bytes: Option<&str>) -> Article {
    writer
        .save_article(
            Tier::Interactive,
            id.into(),
            format!("https://example.org/{id}"),
            "Article".into(),
            None,
            None,
            None,
            Some(icon.into()),
            "preserved body".into(),
            bytes
                .map(|data| HashMap::from([(icon.into(), data.into())]))
                .unwrap_or_default(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn shared_favicon_refresh_survives_writer_restart_and_failed_acquisition() {
    let dir = tempfile::tempdir().unwrap();
    let writer = start(dir.path());
    let first = save(&writer, "first", ICON, Some(PNG)).await;
    let second = save(&writer, "second", ICON, None).await;
    assert_eq!(first.favicon, second.favicon);
    let reference = first.favicon.unwrap();
    let initial = images::read_entry_images(dir.path(), "first", &[reference.clone()]).await;
    assert_eq!(initial, vec![Some(PNG.into())]);

    let mut changed = base64::engine::general_purpose::STANDARD
        .decode(PNG.split_once(',').unwrap().1)
        .unwrap();
    changed.extend_from_slice(b"refresh");
    let changed = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(changed)
    );
    let refreshed = save(&writer, "second", ICON, Some(&changed)).await;
    assert_eq!(refreshed.favicon.as_deref(), Some(reference.as_str()));
    assert_eq!(
        images::read_entry_images(dir.path(), "first", &[reference.clone()]).await,
        vec![Some(changed.clone())]
    );
    save(&writer, "second", ICON, None).await;
    assert_eq!(
        images::read_entry_images(dir.path(), "second", &[reference.clone()]).await,
        vec![Some(changed.clone())]
    );

    writer.shutdown().await;
    drop(writer);
    let writer = start(dir.path());
    for id in ["first", "second"] {
        let article = articles::read_article(dir.path(), id).await.unwrap();
        assert_eq!(article.content, "preserved body");
        assert_eq!(article.favicon.as_deref(), Some(reference.as_str()));
        assert_eq!(
            images::read_entry_images(dir.path(), id, &[reference.clone()]).await,
            vec![Some(changed.clone())]
        );
    }
    writer.shutdown().await;
}

#[tokio::test]
async fn legacy_favicon_refresh_and_failed_new_destination_preserve_existing_asset() {
    let dir = tempfile::tempdir().unwrap();
    let writer = start(dir.path());
    let legacy = "../images/legacy/0123456789ab";
    let image_dir = crate::library::entry_images_dir(dir.path(), "legacy");
    std::fs::create_dir_all(&image_dir).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(PNG.split_once(',').unwrap().1)
        .unwrap();
    std::fs::write(image_dir.join("0123456789ab.png"), &bytes).unwrap();
    save(&writer, "legacy", legacy, None).await;
    assert_eq!(
        images::read_entry_images(dir.path(), "legacy", &[legacy.into()]).await,
        vec![Some(PNG.into())]
    );
    assert_eq!(
        save(&writer, "legacy", ICON, None).await.favicon.as_deref(),
        Some(legacy)
    );
    let refreshed = save(&writer, "legacy", ICON, Some(PNG)).await;
    let reference = refreshed.favicon.unwrap();
    assert!(reference.starts_with("../favicons/"));
    let blocked_url = "https://example.org/new-icon.png";
    let hash = crate::util::url_hash12(&crate::util::normalize_url(blocked_url).unwrap());
    std::fs::create_dir(crate::library::favicon_path(dir.path(), &hash)).unwrap();
    let failed = save(&writer, "legacy", blocked_url, Some(PNG)).await;
    assert_eq!(failed.favicon.as_deref(), Some(reference.as_str()));
    assert_eq!(failed.content, "preserved body");
    assert_eq!(
        images::read_entry_images(dir.path(), "legacy", &[reference]).await,
        vec![Some(PNG.into())]
    );
    writer.shutdown().await;
}

#[tokio::test]
async fn explicit_refresh_recreates_missing_image_bytes_from_new_capture() {
    let dir = tempfile::tempdir().unwrap();
    let writer = start(dir.path());
    let remote_body = "![image](https://example.org/image.png)";
    let capture = HashMap::from([("https://example.org/image.png".into(), PNG.into())]);
    for missing in [false, true] {
        let saved = writer
            .save_article(
                Tier::Interactive,
                "article".into(),
                "https://example.org/article".into(),
                "Article".into(),
                None,
                None,
                None,
                None,
                remote_body.into(),
                capture.clone(),
            )
            .await
            .unwrap();
        let reference = saved
            .content
            .strip_prefix("![image](")
            .unwrap()
            .strip_suffix(')')
            .unwrap();
        assert_eq!(
            images::read_entry_images(dir.path(), "article", &[reference.into()]).await,
            vec![Some(PNG.into())]
        );
        if !missing {
            let image_dir = crate::library::entry_images_dir(dir.path(), "article");
            for file in std::fs::read_dir(image_dir).unwrap() {
                std::fs::remove_file(file.unwrap().path()).unwrap();
            }
            assert_eq!(
                images::read_entry_images(dir.path(), "article", &[reference.into()]).await,
                vec![None]
            );
            assert_eq!(
                articles::read_article(dir.path(), "article")
                    .await
                    .unwrap()
                    .content,
                saved.content
            );
        }
    }
    writer.shutdown().await;
}
