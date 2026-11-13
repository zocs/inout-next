//! Capability-based storage. Only construction opens an ambient host path.
use async_trait::async_trait;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, File, Metadata, OpenOptions};
use share_domain::{Entry, EntryKind, Error, RelativePath};
use std::{io::Write, path::Path, pin::Pin, sync::Arc};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};

pub type Reader = Pin<Box<dyn AsyncRead + Send + Unpin>>;
pub struct OpenFile {
    pub entry: Entry,
    pub reader: Reader,
}

#[async_trait]
pub trait FileStore: Send + Sync {
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
}
impl FsStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, Error> {
        let root = Dir::open_ambient_dir(root, cap_std::ambient_authority()).map_err(io_error)?;
        Ok(Self {
            root: Arc::new(root),
            mutations: tokio::sync::Mutex::new(()),
        })
    }
    fn directory(&self, path: &str) -> Result<Dir, Error> {
        let mut dir = self.root.try_clone().map_err(io_error)?;
        if !path.is_empty() {
            for part in path.split('/') {
                if !Self::metadata(&dir, part)?.is_dir() {
                    return Err(Error::Unsupported);
                }
                dir = dir.open_dir_nofollow(part).map_err(io_error)?;
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
    fn metadata(dir: &Dir, name: &str) -> Result<Metadata, Error> {
        let metadata = dir.symlink_metadata(name).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::Forbidden);
        }
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(Error::Unsupported);
        }
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
    name: String,
    file: Option<File>,
}
impl Stage {
    fn new(dir: &Dir) -> Result<Self, Error> {
        let cleanup_dir = dir.try_clone().map_err(io_error)?;
        for _ in 0..8 {
            let name = format!(".inout-stage-{:032x}", rand::random::<u128>());
            let mut options = OpenOptions::new();
            options
                .write(true)
                .create_new(true)
                .follow(FollowSymlinks::No);
            match dir.open_with(&name, &options) {
                Ok(file) => {
                    return Ok(Self {
                        dir: cleanup_dir,
                        name,
                        file: Some(file),
                    })
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
        // link creation is exclusive even when another process creates the destination.
        self.dir
            .hard_link(&self.name, &self.dir, destination)
            .map_err(io_error)
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        self.file.take();
        let _ = self.dir.remove_file(&self.name);
    }
}

#[async_trait]
impl FileStore for FsStore {
    fn directory_move_supported(&self) -> bool {
        cfg!(any(
            target_os = "linux",
            target_os = "android",
            target_os = "macos",
            target_os = "ios"
        ))
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
            match Self::metadata(&dir, &name) {
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
        if !Self::metadata(&dir, &name)?.is_file() {
            return Err(Error::Unsupported);
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = dir.open_with(&name, &options).map_err(io_error)?;
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
        let mut stage = Stage::new(&dir)?;
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
        Ok(entry(path, &Self::metadata(&dir, &name)?))
    }
    async fn mkdir(&self, path: &RelativePath) -> Result<Entry, Error> {
        let _lock = self.mutations.lock().await;
        let (dir, name) = self.parent(path)?;
        Self::absent(&dir, &name)?;
        dir.create_dir(&name).map_err(io_error)?;
        Ok(entry(path, &Self::metadata(&dir, &name)?))
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
        let metadata = Self::metadata(&source_dir, &source_name)?;
        Self::absent(&destination_dir, &destination_name)?;
        if metadata.is_dir() {
            move_directory(
                &source_dir,
                &source_name,
                &destination_dir,
                &destination_name,
            )?;
        } else {
            source_dir
                .hard_link(&source_name, &destination_dir, &destination_name)
                .map_err(io_error)?;
            if let Err(error) = source_dir.remove_file(&source_name) {
                let _ = destination_dir.remove_file(&destination_name);
                return Err(io_error(error));
            }
        }
        Ok(entry(destination, &metadata))
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
        let metadata = Self::metadata(&dir, &name)?;
        if metadata.is_dir() {
            if recursive {
                dir.remove_dir_all(&name).map_err(io_error)
            } else {
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
fn move_directory(
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
fn move_directory(_: &Dir, _: &str, _: &Dir, _: &str) -> Result<(), Error> {
    Err(Error::Unsupported)
}
