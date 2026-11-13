//! Snapshot trusted public assets only after proving isolation from the actual
//! capability root. Symlinks, identity aliases and mutable path lookups cannot
//! connect the resulting HTTP namespace to shared files.
use crate::identity::Identity;
use crate::{io_error, FsStore, WebAsset};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use share_domain::{Error, RelativePath};
use std::{collections::HashSet, io::Read, path::Path};

pub const MAX_ASSET_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_ASSET_FILES: usize = 10_000;
const MAX_SCANNED_ENTRIES: usize = 100_000;
const MAX_DEPTH: usize = 128;

fn dir_id(dir: &Dir) -> Result<Identity, Error> {
    Identity::of(&dir.try_clone().map_err(io_error)?.into_std_file())
}
fn readonly() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    options
}
fn ancestor_contains(path: &Path, identity: &Identity) -> Result<bool, Error> {
    for ancestor in path.ancestors() {
        // These are trusted configuration validation paths, never API inputs.
        let dir =
            Dir::open_ambient_dir(ancestor, cap_std::ambient_authority()).map_err(io_error)?;
        if &dir_id(&dir)? == identity {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn snapshot(store: &FsStore, web_dir: &Path) -> Result<Vec<WebAsset>, Error> {
    let path = std::fs::canonicalize(web_dir).map_err(io_error)?;
    let assets = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).map_err(io_error)?;
    let root_identity = dir_id(&store.root)?;
    let asset_identity = dir_id(&assets)?;
    if ancestor_contains(&path, &root_identity)?
        || ancestor_contains(&store.root_path, &asset_identity)?
    {
        return Err(Error::Forbidden);
    }
    let mut identities = HashSet::new();
    let mut scanned = 0;
    collect_shared(&store.root, &mut identities, &mut scanned, 0)?;
    let mut files = Vec::new();
    let mut total = 0;
    scan_assets(
        &assets,
        "",
        &identities,
        &root_identity,
        &mut files,
        &mut total,
        0,
    )?;
    Ok(files)
}

fn collect_shared(
    dir: &Dir,
    identities: &mut HashSet<Identity>,
    scanned: &mut usize,
    depth: usize,
) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::TooLarge);
    }
    for item in dir.entries().map_err(io_error)? {
        let item = item.map_err(io_error)?;
        *scanned += 1;
        if *scanned > MAX_SCANNED_ENTRIES {
            return Err(Error::TooLarge);
        }
        let name = item.file_name();
        let metadata = dir.symlink_metadata(&name).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            let child = dir.open_dir_nofollow(&name).map_err(io_error)?;
            identities.insert(dir_id(&child)?);
            collect_shared(&child, identities, scanned, depth + 1)?;
        } else if metadata.is_file() {
            let file = dir.open_with(&name, &readonly()).map_err(io_error)?;
            identities.insert(Identity::of(&file.into_std())?);
        }
    }
    Ok(())
}

fn scan_assets(
    dir: &Dir,
    prefix: &str,
    shared: &HashSet<Identity>,
    root: &Identity,
    files: &mut Vec<WebAsset>,
    total: &mut u64,
    depth: usize,
) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::TooLarge);
    }
    let identity = dir_id(dir)?;
    if &identity == root || shared.contains(&identity) {
        return Err(Error::Forbidden);
    }
    for item in dir.entries().map_err(io_error)? {
        let item = item.map_err(io_error)?;
        let name = item
            .file_name()
            .into_string()
            .map_err(|_| Error::InvalidPath)?;
        let relative = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        RelativePath::parse(&relative, false)?;
        let metadata = dir.symlink_metadata(&name).map_err(io_error)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::Forbidden);
        }
        if metadata.is_dir() {
            scan_assets(
                &dir.open_dir_nofollow(&name).map_err(io_error)?,
                &relative,
                shared,
                root,
                files,
                total,
                depth + 1,
            )?;
        } else if metadata.is_file() {
            let file = dir.open_with(&name, &readonly()).map_err(io_error)?;
            let identity = Identity::of(&file.try_clone().map_err(io_error)?.into_std())?;
            if shared.contains(&identity) {
                return Err(Error::Forbidden);
            }
            if files.len() >= MAX_ASSET_FILES {
                return Err(Error::TooLarge);
            }
            let mut bytes = Vec::new();
            file.take(MAX_ASSET_BYTES.saturating_sub(*total) + 1)
                .read_to_end(&mut bytes)
                .map_err(io_error)?;
            *total += bytes.len() as u64;
            if *total > MAX_ASSET_BYTES {
                return Err(Error::TooLarge);
            }
            files.push(WebAsset {
                path: relative,
                bytes,
            });
        } else {
            return Err(Error::Unsupported);
        }
    }
    Ok(())
}
