//! Snapshot trusted public assets only after proving isolation from the actual
//! capability root. Symlinks, identity aliases and mutable path lookups cannot
//! connect the resulting HTTP namespace to shared files.
//!
//! A mount-origin proof and singly linked assets avoid opening every shared
//! file. Without that proof, retain the bounded identity walk and fail closed.
use crate::identity::{link_count, Identity};
use crate::{io_error, FsStore, WebAsset};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use share_domain::{Error, RelativePath};
use std::{collections::HashSet, io::Read, path::Path};

pub const MAX_ASSET_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_ASSET_FILES: usize = 10_000;
const MAX_SCANNED_ENTRIES: usize = 100_000;
const MAX_DEPTH: usize = 128;

#[derive(Default)]
struct AssetIdentities {
    objects: HashSet<Identity>,
    hard_links: bool,
}

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
    snapshot_bounded(store, web_dir, MAX_SCANNED_ENTRIES)
}

fn snapshot_bounded(
    store: &FsStore,
    web_dir: &Path,
    scan_limit: usize,
) -> Result<Vec<WebAsset>, Error> {
    let path = std::fs::canonicalize(web_dir).map_err(io_error)?;
    let assets = Dir::open_ambient_dir(&path, cap_std::ambient_authority()).map_err(io_error)?;
    let root_identity = dir_id(&store.root)?;
    let asset_identity = dir_id(&assets)?;
    if ancestor_contains(&path, &root_identity)?
        || ancestor_contains(&store.root_path, &asset_identity)?
    {
        return Err(Error::Forbidden);
    }
    let mut files = Vec::new();
    let mount_proof = crate::mounts::isolated(&store.root, &assets)?;
    let mut asset_identities = AssetIdentities::default();
    let mut total = 0;
    scan_assets(
        &assets,
        "",
        &root_identity,
        &mut files,
        &mut asset_identities,
        &mut total,
        0,
    )?;
    if asset_identities.hard_links || !mount_proof {
        let mut identities = HashSet::new();
        let mut scanned = 0;
        collect_shared(&store.root, &mut identities, &mut scanned, scan_limit, 0)?;
        if !asset_identities.objects.is_disjoint(&identities) {
            return Err(Error::Forbidden);
        }
    }
    Ok(files)
}

fn collect_shared(
    dir: &Dir,
    identities: &mut HashSet<Identity>,
    scanned: &mut usize,
    limit: usize,
    depth: usize,
) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::TooLarge);
    }
    for item in dir.entries().map_err(io_error)? {
        let item = item.map_err(io_error)?;
        *scanned += 1;
        if *scanned > limit {
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
            collect_shared(&child, identities, scanned, limit, depth + 1)?;
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
    root: &Identity,
    files: &mut Vec<WebAsset>,
    identities: &mut AssetIdentities,
    total: &mut u64,
    depth: usize,
) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(Error::TooLarge);
    }
    let identity = dir_id(dir)?;
    if &identity == root {
        return Err(Error::Forbidden);
    }
    identities.objects.insert(identity);
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
                root,
                files,
                identities,
                total,
                depth + 1,
            )?;
        } else if metadata.is_file() {
            let file = dir.open_with(&name, &readonly()).map_err(io_error)?;
            let handle = file.try_clone().map_err(io_error)?.into_std();
            identities.objects.insert(Identity::of(&handle)?);
            identities.hard_links |= link_count(&handle)? != 1;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture(shared_files: usize) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let parent = tempfile::tempdir().unwrap();
        let share = parent.path().join("share");
        let assets = parent.path().join("assets");
        fs::create_dir(&share).unwrap();
        fs::create_dir_all(assets.join("_next")).unwrap();
        fs::write(assets.join("index.html"), b"trusted").unwrap();
        fs::write(assets.join("_next/app.js"), b"script").unwrap();
        for index in 0..shared_files {
            fs::write(share.join(format!("file-{index}.txt")), b"shared").unwrap();
        }
        (parent, share, assets)
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn singly_linked_assets_need_no_scan_of_a_share_beyond_the_bound() {
        let (_parent, share, assets) = fixture(8);
        let store = FsStore::new(&share).unwrap();
        let files = snapshot_bounded(&store, &assets, 2).unwrap();
        let mut paths: Vec<_> = files.iter().map(|file| file.path.as_str()).collect();
        paths.sort_unstable();
        assert_eq!(paths, ["_next/app.js", "index.html"]);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn an_unreadable_shared_entry_does_not_block_singly_linked_assets() {
        use std::os::unix::fs::PermissionsExt;
        let (_parent, share, assets) = fixture(1);
        let locked = share.join("locked");
        fs::create_dir(&locked).unwrap();
        fs::write(share.join("secret.bin"), b"private").unwrap();
        fs::set_permissions(share.join("secret.bin"), fs::Permissions::from_mode(0o000)).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let store = FsStore::new(&share).unwrap();
        let result = snapshot_bounded(&store, &assets, MAX_SCANNED_ENTRIES);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(result.unwrap().len(), 2);
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    #[test]
    fn an_unavailable_mount_proof_retains_the_bounded_identity_walk() {
        let (_parent, share, assets) = fixture(8);
        let store = FsStore::new(&share).unwrap();
        assert!(matches!(
            snapshot_bounded(&store, &assets, 2),
            Err(Error::TooLarge)
        ));
    }

    #[test]
    fn multiply_linked_assets_still_need_a_bounded_proof_against_the_share() {
        let (parent, share, assets) = fixture(8);
        fs::hard_link(
            assets.join("index.html"),
            parent.path().join("elsewhere.html"),
        )
        .unwrap();
        let store = FsStore::new(&share).unwrap();
        assert!(matches!(
            snapshot_bounded(&store, &assets, 2),
            Err(Error::TooLarge)
        ));
        assert_eq!(snapshot_bounded(&store, &assets, 100).unwrap().len(), 2);
        fs::hard_link(share.join("file-0.txt"), assets.join("alias.txt")).unwrap();
        assert!(matches!(
            snapshot_bounded(&store, &assets, 100),
            Err(Error::Forbidden)
        ));
    }
}
