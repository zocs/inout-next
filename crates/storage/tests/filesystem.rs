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
    let names: Vec<_> = std::fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, ["keep"]);
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
