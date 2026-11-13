use share_domain::{EntryKind, Error, RelativePath};
use share_storage::{FileStore, FsStore, Reader};
use tokio::io::AsyncReadExt;

fn path(value: &str) -> RelativePath {
    RelativePath::parse(value, true).unwrap()
}
fn bytes(value: &'static [u8]) -> Reader {
    Box::pin(value)
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
))]
#[tokio::test]
async fn directory_move_is_exclusive_and_rejects_moving_inside_itself() {
    let root = tempfile::tempdir().unwrap();
    let store = FsStore::new(root.path()).unwrap();
    store.mkdir(&path("source")).await.unwrap();
    store.mkdir(&path("target")).await.unwrap();
    store
        .upload(&path("source/a"), bytes(b"original"), 100)
        .await
        .unwrap();
    assert!(matches!(
        store.move_entry(&path("source"), &path("target")).await,
        Err(Error::TargetExists)
    ));
    assert!(store
        .move_entry(&path("source"), &path("source/inside"))
        .await
        .is_err());
    store
        .move_entry(&path("source"), &path("moved"))
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(root.path().join("moved/a")).unwrap(),
        b"original"
    );
    assert!(!root.path().join("source").exists());
}

#[tokio::test]
async fn upload_list_open_and_copy_move_preserve_bytes_and_metadata() {
    let root = tempfile::tempdir().unwrap();
    let store = FsStore::new(root.path()).unwrap();
    let directory = store.mkdir(&path("docs")).await.unwrap();
    assert_eq!(directory.kind, EntryKind::Directory);
    assert_eq!(directory.size, None);
    let entry = store
        .upload(&path("docs/a.txt"), bytes(b"abcdef"), 100)
        .await
        .unwrap();
    assert_eq!(entry.size, Some(6));
    assert_eq!(entry.path, "docs/a.txt");
    assert_eq!(store.list(&path("docs")).await.unwrap().len(), 1);
    let mut opened = store.open(&path("docs/a.txt"), 2).await.unwrap();
    let mut content = Vec::new();
    opened.reader.read_to_end(&mut content).await.unwrap();
    assert_eq!(content, b"cdef");
    store
        .copy(&path("docs/a.txt"), &path("copy.txt"))
        .await
        .unwrap();
    if !store.file_move_supported() {
        assert!(matches!(
            store
                .move_entry(&path("copy.txt"), &path("moved.txt"))
                .await,
            Err(Error::Unsupported)
        ));
        assert_eq!(
            std::fs::read(root.path().join("copy.txt")).unwrap(),
            b"abcdef"
        );
        assert!(!root.path().join("moved.txt").exists());
        return;
    }
    store
        .move_entry(&path("copy.txt"), &path("moved.txt"))
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(root.path().join("moved.txt")).unwrap(),
        b"abcdef"
    );
    assert!(!root.path().join("copy.txt").exists());
}

#[tokio::test]
async fn collisions_never_overwrite_and_failed_streams_remove_staging() {
    let root = tempfile::tempdir().unwrap();
    let store = FsStore::new(root.path()).unwrap();
    store
        .upload(&path("keep"), bytes(b"original"), 100)
        .await
        .unwrap();
    assert!(matches!(
        store.upload(&path("keep"), bytes(b"replace"), 100).await,
        Err(Error::TargetExists)
    ));
    assert!(matches!(
        store.upload(&path("huge"), bytes(b"12345"), 4).await,
        Err(Error::TooLarge)
    ));
    let (mut writer, reader) = tokio::io::duplex(8);
    use tokio::io::AsyncWriteExt;
    writer.write_all(b"partial").await.unwrap();
    let cancelled = path("cancelled");
    let pending = store.upload(&cancelled, Box::pin(reader), 100);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(40), pending)
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read(root.path().join("keep")).unwrap(),
        b"original"
    );
    let mut names: Vec<_> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["INOUTTMP", "keep"]);
    assert_eq!(
        std::fs::read_dir(root.path().join("INOUTTMP"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn root_mutations_and_nonempty_delete_are_guarded() {
    let root = tempfile::tempdir().unwrap();
    let store = FsStore::new(root.path()).unwrap();
    assert_eq!(store.remove(&path(""), true).await, Err(Error::InvalidPath));
    assert!(matches!(
        store.mkdir(&path("")).await,
        Err(Error::InvalidPath)
    ));
    store.mkdir(&path("dir")).await.unwrap();
    store
        .upload(&path("dir/child"), bytes(b"hello"), 10)
        .await
        .unwrap();
    assert_eq!(
        store.remove(&path("dir"), false).await,
        Err(Error::NotEmpty)
    );
    store.remove(&path("dir"), true).await.unwrap();
    assert!(!root.path().join("dir").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn symlinks_are_neither_listed_read_nor_followed_for_writes() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), b"private").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("secret"), root.path().join("file-link"))
        .unwrap();
    let store = FsStore::new(root.path()).unwrap();
    assert!(store.list(&path("")).await.unwrap().is_empty());
    assert!(matches!(
        store.open(&path("file-link"), 0).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store.open(&path("link/secret"), 0).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store.upload(&path("link/new"), bytes(b"oops"), 10).await,
        Err(Error::Forbidden)
    ));
    assert!(store.remove(&path("link"), true).await.is_err());
    assert_eq!(
        std::fs::read(outside.path().join("secret")).unwrap(),
        b"private"
    );
    assert!(!outside.path().join("new").exists());
}

#[cfg(unix)]
#[tokio::test]
async fn internal_stage_files_are_inaccessible_through_filesystem_identity_aliases() {
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let store = Arc::new(FsStore::new(root.path()).unwrap());
    let (mut writer, reader) = tokio::io::duplex(16);
    use tokio::io::AsyncWriteExt;
    writer.write_all(b"unfinished").await.unwrap();
    let uploading = store.clone();
    let task = tokio::spawn(async move {
        uploading
            .upload(&path("final"), Box::pin(reader), 100)
            .await
    });
    let staged = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            let mut files = Vec::new();
            for entry in std::fs::read_dir(root.path()).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    files.push(entry.path());
                }
                if entry.file_type().unwrap().is_dir() {
                    files.extend(
                        std::fs::read_dir(entry.path())
                            .unwrap()
                            .map(|child| child.unwrap().path()),
                    );
                }
            }
            if let Some(file) = files.into_iter().next() {
                break file;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    std::fs::hard_link(staged, root.path().join("ordinary-alias")).unwrap();
    assert!(matches!(
        store.open(&path("ordinary-alias"), 0).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store.copy(&path("ordinary-alias"), &path("copied")).await,
        Err(Error::Forbidden)
    ));
    assert!(matches!(
        store
            .move_entry(&path("ordinary-alias"), &path("moved"))
            .await,
        Err(Error::Forbidden)
    ));
    assert_eq!(
        store.remove(&path("ordinary-alias"), false).await,
        Err(Error::Forbidden)
    );
    assert!(store.list(&path("")).await.unwrap().is_empty());
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    std::fs::remove_file(root.path().join("ordinary-alias")).unwrap();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(
        std::fs::read_dir(root.path().join("INOUTTMP"))
            .unwrap()
            .count(),
        0
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn file_move_is_observed_as_one_native_rename_without_link_unlink_window() {
    use rustix::fs::inotify;
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source"), b"original").unwrap();
    let store = FsStore::new(root.path()).unwrap();
    let fd = inotify::init(inotify::CreateFlags::NONBLOCK | inotify::CreateFlags::CLOEXEC).unwrap();
    inotify::add_watch(
        &fd,
        root.path(),
        inotify::WatchFlags::MOVED_FROM
            | inotify::WatchFlags::MOVED_TO
            | inotify::WatchFlags::CREATE
            | inotify::WatchFlags::DELETE,
    )
    .unwrap();
    store
        .move_entry(&path("source"), &path("destination"))
        .await
        .unwrap();
    let mut buffer = [std::mem::MaybeUninit::uninit(); 4096];
    let mut reader = inotify::Reader::new(&fd, &mut buffer);
    let mut events = Vec::new();
    while let Ok(event) = reader.next() {
        events.push((
            event.events(),
            event.cookie(),
            event.file_name().unwrap().to_str().unwrap().to_owned(),
        ));
    }
    assert_eq!(events.len(), 2);
    assert!(events[0].0.contains(inotify::ReadFlags::MOVED_FROM));
    assert!(events[1].0.contains(inotify::ReadFlags::MOVED_TO));
    assert_ne!(events[0].1, 0);
    assert_eq!(events[0].1, events[1].1);
    assert_eq!(events[0].2, "source");
    assert_eq!(events[1].2, "destination");
    assert_eq!(
        std::fs::read(root.path().join("destination")).unwrap(),
        b"original"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn cross_volume_native_move_preserves_source_and_creates_no_destination() {
    use std::os::unix::fs::MetadataExt;
    let source = tempfile::tempdir().unwrap();
    let Ok(destination) = tempfile::tempdir_in("/dev/shm") else {
        return;
    };
    if source.path().metadata().unwrap().dev() == destination.path().metadata().unwrap().dev() {
        return;
    }
    std::fs::write(source.path().join("source"), b"original").unwrap();
    let store = FsStore::new("/").unwrap();
    let from = path(
        source
            .path()
            .join("source")
            .to_str()
            .unwrap()
            .strip_prefix('/')
            .unwrap(),
    );
    let to = path(
        destination
            .path()
            .join("target")
            .to_str()
            .unwrap()
            .strip_prefix('/')
            .unwrap(),
    );
    assert!(matches!(
        store.move_entry(&from, &to).await,
        Err(Error::Unsupported)
    ));
    assert_eq!(
        std::fs::read(source.path().join("source")).unwrap(),
        b"original"
    );
    assert!(!destination.path().join("target").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn independent_providers_publish_parallel_uploads_and_preserve_collision_winners() {
    use std::sync::Arc;
    const PAYLOADS: [&[u8]; 8] = [
        b"first", b"second", b"third", b"fourth", b"fifth", b"sixth", b"seventh", b"eighth",
    ];
    let root = tempfile::tempdir().unwrap();
    let stores: Vec<_> = (0..8)
        .map(|_| Arc::new(FsStore::new(root.path()).unwrap()))
        .collect();
    for round in 0..16 {
        let barrier = Arc::new(tokio::sync::Barrier::new(8));
        let mut tasks = Vec::new();
        for (worker, store) in stores.iter().enumerate() {
            let store = store.clone();
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                let name = format!("distinct-{round}-{worker}");
                barrier.wait().await;
                let entry = store
                    .upload(&path(&name), bytes(PAYLOADS[worker]), 100)
                    .await
                    .expect("distinct upload failed during another provider's cleanup");
                (name, entry.size, worker)
            }));
        }
        for task in tasks {
            let (name, size, worker) = task.await.unwrap();
            assert_eq!(size, Some(PAYLOADS[worker].len() as u64));
            assert_eq!(
                std::fs::read(root.path().join(name)).unwrap(),
                PAYLOADS[worker]
            );
        }
        let barrier = Arc::new(tokio::sync::Barrier::new(8));
        let mut tasks = Vec::new();
        for (worker, store) in stores.iter().enumerate() {
            let store = store.clone();
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                (
                    worker,
                    store
                        .upload(
                            &path(&format!("collision-{round}")),
                            bytes(PAYLOADS[worker]),
                            100,
                        )
                        .await,
                )
            }));
        }
        let mut winner = None;
        for task in tasks {
            let (worker, result) = task.await.unwrap();
            match result {
                Ok(_) => {
                    assert!(
                        winner.replace(worker).is_none(),
                        "multiple collision winners"
                    );
                }
                Err(error) => assert_eq!(error, Error::TargetExists),
            }
        }
        assert_eq!(
            std::fs::read(root.path().join(format!("collision-{round}"))).unwrap(),
            PAYLOADS[winner.unwrap()]
        );
    }
    assert_eq!(stores[0].list(&path("")).await.unwrap().len(), 144);
    assert_eq!(
        std::fs::read_dir(root.path().join("INOUTTMP"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn nonrecursive_delete_removes_an_otherwise_empty_user_folder_with_internal_container() {
    let root = tempfile::tempdir().unwrap();
    let store = FsStore::new(root.path()).unwrap();
    store.mkdir(&path("user-folder")).await.unwrap();
    store
        .upload(&path("user-folder/file"), bytes(b"original"), 100)
        .await
        .unwrap();
    assert!(root.path().join("user-folder/INOUTTMP").is_dir());
    assert_eq!(
        store.remove(&path("user-folder"), false).await,
        Err(Error::NotEmpty)
    );
    assert!(root.path().join("user-folder/INOUTTMP").is_dir());
    assert_eq!(
        std::fs::read(root.path().join("user-folder/file")).unwrap(),
        b"original"
    );
    store
        .remove(&path("user-folder/file"), false)
        .await
        .unwrap();
    assert!(store.list(&path("user-folder")).await.unwrap().is_empty());
    store.remove(&path("user-folder"), false).await.unwrap();
    assert!(!root.path().join("user-folder").exists());
}
