//! Capability-based storage. Only construction opens an ambient host path.
use async_trait::async_trait;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File, Metadata, OpenOptions};
use share_domain::{Entry, EntryKind, Error, RelativePath};
use std::{io::Write, path::Path, pin::Pin, sync::Arc};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};
mod assets;
mod identity;
#[cfg(test)]
mod lifecycle_tests;
mod protected;
use protected::{ProtectedObjects, Protection};

/// A provider-validated immutable public asset; no live user-file paths remain.
pub struct WebAsset {
    pub path: String,
    pub bytes: Vec<u8>,
}

pub type Reader = Pin<Box<dyn AsyncRead + Send + Unpin>>;
pub struct OpenFile {
    pub entry: Entry,
    pub reader: Reader,
}

#[async_trait]
/// Operation futures must own their work and release resources when dropped.
/// Providers must not detach writes or publish after a cancelled future ends.
pub trait FileStore: Send + Sync {
    /// Providers must prove separation from their actual share. Unknown providers
    /// reject public assets rather than opening an unrelated config.root.
    fn web_assets(&self, _: &Path) -> Result<Vec<WebAsset>, Error> {
        Err(Error::Unsupported)
    }
    fn file_move_supported(&self) -> bool {
        false
    }
    /// Providers report support independently of mounted HTTP modules.
    fn directory_move_supported(&self) -> bool {
        false
    }
    async fn list(&self, path: &RelativePath) -> Result<Vec<Entry>, Error>;
    async fn open(&self, path: &RelativePath, offset: u64) -> Result<OpenFile, Error>;
    async fn upload(&self, path: &RelativePath, reader: Reader, limit: u64)
        -> Result<Entry, Error>;
    async fn mkdir(&self, path: &RelativePath) -> Result<Entry, Error>;
    async fn move_entry(
        &self,
        source: &RelativePath,
        destination: &RelativePath,
    ) -> Result<Entry, Error>;
    async fn copy(&self, source: &RelativePath, destination: &RelativePath)
        -> Result<Entry, Error>;
    async fn remove(&self, path: &RelativePath, recursive: bool) -> Result<(), Error>;
}

pub struct FsStore {
    root: Arc<Dir>,
    mutations: tokio::sync::Mutex<()>,
    protected: Arc<ProtectedObjects>,
    root_path: std::path::PathBuf,
}
impl FsStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, Error> {
        let root_path = std::fs::canonicalize(root.as_ref()).map_err(io_error)?;
        let root = Dir::open_ambient_dir(root_path.clone(), cap_std::ambient_authority())
            .map_err(io_error)?;
        Ok(Self {
            root: Arc::new(root),
            mutations: tokio::sync::Mutex::new(()),
            protected: Arc::new(ProtectedObjects::default()),
            root_path,
        })
    }
    fn directory(&self, path: &str) -> Result<Dir, Error> {
        let mut dir = self.root.try_clone().map_err(io_error)?;
        if !path.is_empty() {
            for part in path.split('/') {
                if !self.metadata(&dir, part)?.is_dir() {
                    return Err(Error::Unsupported);
                }
                dir = dir.open_dir_nofollow(part).map_err(io_error)?;
                self.protected.check_dir(&dir)?;
            }
        }
        Ok(dir)
    }
    fn parent(&self, path: &RelativePath) -> Result<(Dir, String), Error> {
        if path.as_str().is_empty() {
            return Err(Error::InvalidPath);
        }
        let (parent, name) = path
            .as_str()
            .rsplit_once('/')
            .unwrap_or(("", path.as_str()));
        Ok((self.directory(parent)?, name.to_owned()))
    }
    fn metadata(&self, dir: &Dir, name: &str) -> Result<Metadata, Error> {
        let metadata = dir.symlink_metadata(name).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::Forbidden);
        }
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(Error::Unsupported);
        }
        self.protected.check_entry(dir, name, &metadata)?;
        Ok(metadata)
    }
    fn absent(dir: &Dir, name: &str) -> Result<(), Error> {
        match dir.symlink_metadata(name) {
            Ok(_) => Err(Error::TargetExists),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }
}

fn io_error(error: std::io::Error) -> Error {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    ))]
    if error.raw_os_error() == Some(rustix::io::Errno::LOOP.raw_os_error()) {
        return Error::Forbidden;
    }
    use std::io::ErrorKind;
    match error.kind() {
        ErrorKind::NotFound => Error::NotFound,
        ErrorKind::AlreadyExists => Error::TargetExists,
        ErrorKind::PermissionDenied => Error::Forbidden,
        ErrorKind::DirectoryNotEmpty => Error::NotEmpty,
        ErrorKind::CrossesDevices | ErrorKind::Unsupported => Error::Unsupported,
        ErrorKind::InvalidInput => Error::InvalidPath,
        _ => Error::Io,
    }
}

fn entry(path: &RelativePath, metadata: &Metadata) -> Entry {
    Entry {
        name: path
            .as_str()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned(),
        path: path.as_str().to_owned(),
        kind: if metadata.is_dir() {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        size: if metadata.is_dir() {
            None
        } else {
            Some(metadata.len())
        },
        modified: metadata
            .modified()
            .ok()
            .and_then(|time| time.into_std().duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
    }
}

/// Removes unpublished files on errors and future cancellation. The file is closed
/// before unlinking, including on Windows. No asynchronous writes outlive the guard.
struct Stage {
    dir: Dir,
    destination: Dir,
    name: String,
    file: Option<File>,
    _protection: Protection,
    published: bool,
}
impl Stage {
    fn new(destination: &Dir, protected: &Arc<ProtectedObjects>) -> Result<Self, Error> {
        const CONTAINER: &str = "INOUTTMP";
        // This is an exact 8.3 directory name, so native short-name generation
        // cannot introduce a second spelling for the staging namespace.
        match destination.create_dir(CONTAINER) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let canonical_name_exists = destination.entries().map_err(io_error)?.any(|item| {
                    item.ok().is_some_and(|entry| {
                        entry
                            .file_name()
                            .to_str()
                            .is_some_and(|name| name.eq_ignore_ascii_case(CONTAINER))
                    })
                });
                if !canonical_name_exists {
                    return Err(Error::Forbidden);
                }
            }
            Err(error) => return Err(io_error(error)),
        }
        let dir = destination.open_dir_nofollow(CONTAINER).map_err(io_error)?;
        let cleanup_dir = dir.try_clone().map_err(io_error)?;
        let destination = destination.try_clone().map_err(io_error)?;
        for _ in 0..8 {
            let name = format!("{:08x}", rand::random::<u32>());
            let mut options = OpenOptions::new();
            options
                .write(true)
                .create_new(true)
                .follow(FollowSymlinks::No);
            match dir.open_with(&name, &options) {
                Ok(file) => {
                    let protection = match protected.register(&dir, &file) {
                        Ok(protection) => protection,
                        Err(error) => {
                            drop(file);
                            let _ = cleanup_dir.remove_file(&name);
                            return Err(error);
                        }
                    };
                    return Ok(Self {
                        dir: cleanup_dir,
                        destination,
                        name,
                        file: Some(file),
                        _protection: protection,
                        published: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(io_error(error)),
            }
        }
        Err(Error::Conflict)
    }
    fn publish(&mut self, destination: &str) -> Result<(), Error> {
        self.file
            .as_mut()
            .ok_or(Error::Conflict)?
            .flush()
            .map_err(io_error)?;
        self.file
            .as_ref()
            .ok_or(Error::Conflict)?
            .sync_all()
            .map_err(io_error)?;
        self.file.take();
        #[cfg(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios"
        ))]
        {
            move_exclusively(&self.dir, &self.name, &self.destination, destination)?;
            self.published = true;
            Ok(())
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios"
        )))]
        {
            // Publication of app-owned internal data remains exclusive. This is
            // never used to emulate a user-file move or roll back user entries.
            self.dir
                .hard_link(&self.name, &self.destination, destination)
                .map_err(io_error)
        }
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        self.file.take();
        if !self.published {
            let _ = self.dir.remove_file(&self.name);
        }
        // Keep the empty reserved container attached. Another provider/process
        // may have opened it for setup but not yet created a staging file.
        // Eager removal here would invalidate that provider's capability.
    }
}

#[async_trait]
impl FileStore for FsStore {
    fn web_assets(&self, web_dir: &Path) -> Result<Vec<WebAsset>, Error> {
        assets::snapshot(self, web_dir)
    }
    fn file_move_supported(&self) -> bool {
        native_move_supported()
    }
    fn directory_move_supported(&self) -> bool {
        native_move_supported()
    }
    async fn list(&self, path: &RelativePath) -> Result<Vec<Entry>, Error> {
        let dir = self.directory(path.as_str())?;
        let mut result = Vec::new();
        for item in dir.entries().map_err(io_error)? {
            let item = item.map_err(io_error)?;
            let Ok(name) = item.file_name().into_string() else {
                continue;
            };
            let joined = if path.as_str().is_empty() {
                name.clone()
            } else {
                format!("{}/{name}", path.as_str())
            };
            let Ok(relative) = RelativePath::parse(&joined, false) else {
                continue;
            };
            match self.metadata(&dir, &name) {
                Ok(metadata) => result.push(entry(&relative, &metadata)),
                Err(Error::Forbidden | Error::Unsupported | Error::NotFound) => continue,
                Err(error) => return Err(error),
            }
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }
    async fn open(&self, path: &RelativePath, offset: u64) -> Result<OpenFile, Error> {
        let (dir, name) = self.parent(path)?;
        if !self.metadata(&dir, &name)?.is_file() {
            return Err(Error::Unsupported);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = dir.open_with(&name, &options).map_err(io_error)?;
        self.protected.check_file(&file)?;
        let metadata = file.metadata().map_err(io_error)?;
        if !metadata.is_file() {
            return Err(Error::Unsupported);
        }
        let mut reader = tokio::fs::File::from_std(file.into_std());
        if offset != 0 {
            reader
                .seek(std::io::SeekFrom::Start(offset))
                .await
                .map_err(io_error)?;
        }
        Ok(OpenFile {
            entry: entry(path, &metadata),
            reader: Box::pin(reader),
        })
    }
    async fn upload(
        &self,
        path: &RelativePath,
        mut reader: Reader,
        limit: u64,
    ) -> Result<Entry, Error> {
        let (dir, name) = self.parent(path)?;
        Self::absent(&dir, &name)?;
        let mut stage = Stage::new(&dir, &self.protected)?;
        let mut buffer = vec![0_u8; 64 * 1024];
        let mut written = 0_u64;
        loop {
            let count = reader.read(&mut buffer).await.map_err(io_error)?;
            if count == 0 {
                break;
            }
            written = written.checked_add(count as u64).ok_or(Error::TooLarge)?;
            if written > limit {
                return Err(Error::TooLarge);
            }
            stage
                .file
                .as_mut()
                .ok_or(Error::Conflict)?
                .write_all(&buffer[..count])
                .map_err(io_error)?;
        }
        stage.publish(&name)?;
        // Published inode remains protected until the guard leaves this scope.
        drop(stage);
        Ok(entry(path, &self.metadata(&dir, &name)?))
    }
    async fn mkdir(&self, path: &RelativePath) -> Result<Entry, Error> {
        let _lock = self.mutations.lock().await;
        let (dir, name) = self.parent(path)?;
        Self::absent(&dir, &name)?;
        dir.create_dir(&name).map_err(io_error)?;
        Ok(entry(path, &self.metadata(&dir, &name)?))
    }
    async fn move_entry(
        &self,
        source: &RelativePath,
        destination: &RelativePath,
    ) -> Result<Entry, Error> {
        let _lock = self.mutations.lock().await;
        if destination
            .as_str()
            .starts_with(&format!("{}/", source.as_str()))
        {
            return Err(Error::InvalidPath);
        }
        let (source_dir, source_name) = self.parent(source)?;
        let (destination_dir, destination_name) = self.parent(destination)?;
        self.metadata(&source_dir, &source_name)?;
        Self::absent(&destination_dir, &destination_name)?;
        move_exclusively(
            &source_dir,
            &source_name,
            &destination_dir,
            &destination_name,
        )?;
        Ok(entry(
            destination,
            &self.metadata(&destination_dir, &destination_name)?,
        ))
    }
    async fn copy(
        &self,
        source: &RelativePath,
        destination: &RelativePath,
    ) -> Result<Entry, Error> {
        let opened = self.open(source, 0).await?;
        self.upload(destination, opened.reader, u64::MAX).await
    }
    async fn remove(&self, path: &RelativePath, recursive: bool) -> Result<(), Error> {
        let _lock = self.mutations.lock().await;
        let (dir, name) = self.parent(path)?;
        let metadata = self.metadata(&dir, &name)?;
        if metadata.is_dir() {
            if recursive {
                dir.remove_dir_all(&name).map_err(io_error)
            } else {
                let user_dir = dir.open_dir_nofollow(&name).map_err(io_error)?;
                let entries: Vec<_> = user_dir
                    .entries()
                    .map_err(io_error)?
                    .take(2)
                    .collect::<Result<_, _>>()
                    .map_err(io_error)?;
                if let Some(internal) = entries.iter().find(|item| {
                    item.file_name()
                        .to_str()
                        .is_some_and(|name| name.eq_ignore_ascii_case("INOUTTMP"))
                }) {
                    // Never remove the container while another public child
                    // makes this a nonempty user folder. Empty retained staging
                    // alone must not make an empty folder undeletable.
                    if entries.len() != 1 {
                        return Err(Error::NotEmpty);
                    }
                    user_dir
                        .remove_dir(internal.file_name())
                        .map_err(io_error)?;
                }
                dir.remove_dir(&name).map_err(io_error)
            }
        } else {
            dir.remove_file(&name).map_err(io_error)
        }
    }
}

#[cfg(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
))]
fn move_exclusively(
    source: &Dir,
    source_name: &str,
    destination: &Dir,
    destination_name: &str,
) -> Result<(), Error> {
    rustix::fs::renameat_with(
        source,
        source_name,
        destination,
        destination_name,
        rustix::fs::RenameFlags::NOREPLACE,
    )
    .map_err(|error| match error {
        rustix::io::Errno::NOSYS | rustix::io::Errno::INVAL | rustix::io::Errno::OPNOTSUPP => {
            Error::Unsupported
        }
        _ => io_error(error.into()),
    })
}
#[cfg(not(any(
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
)))]
fn move_exclusively(_: &Dir, _: &str, _: &Dir, _: &str) -> Result<(), Error> {
    Err(Error::Unsupported)
}

fn native_move_supported() -> bool {
    cfg!(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    ))
}
