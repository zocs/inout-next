//! Compare Linux filesystem subtrees, including bind mounts of individual files.
//! Layered origins across device identities are rejected: identity walks cannot
//! prove that their underlying content is separate. Other unavailable topology
//! requires the bounded identity walk.
use cap_std::fs::Dir;
use share_domain::Error;

pub(crate) fn isolated(root: &Dir, assets: &Dir) -> Result<bool, Error> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    return match linux::prove(root, assets) {
        Ok(()) => Ok(true),
        Err(Error::Forbidden) => Err(Error::Forbidden),
        // Restricted /proc, unknown handles or an incomplete mount table never
        // authorize the fast path. The caller keeps its bounded identity scan.
        Err(_) => Ok(false),
    };
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        let _ = (root, assets);
        Ok(false)
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod linux {
    use super::*;
    use crate::identity::Identity;
    use std::{
        collections::HashSet,
        ffi::OsString,
        fs::File,
        io::Read,
        os::{fd::AsRawFd, unix::ffi::OsStringExt},
        path::{Component, Path, PathBuf},
    };

    const MAX_TABLE_BYTES: u64 = 8 * 1024 * 1024;
    const MAX_MOUNTS: usize = 16_384;

    #[derive(Debug)]
    struct Mount {
        id: u64,
        device: (u64, u64),
        root: PathBuf,
        point: PathBuf,
        layered: bool,
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct Origin {
        device: (u64, u64),
        path: PathBuf,
        layered: bool,
    }

    fn number(bytes: &[u8]) -> Result<u64, Error> {
        std::str::from_utf8(bytes)
            .ok()
            .filter(|text| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|text| text.parse().ok())
            .ok_or(Error::Unsupported)
    }

    fn decode_path(bytes: &[u8]) -> Result<PathBuf, Error> {
        let mut decoded = Vec::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                let escaped = bytes.get(index + 1..index + 4).ok_or(Error::Unsupported)?;
                decoded.push(match escaped {
                    b"040" => b' ',
                    b"011" => b'\t',
                    b"012" => b'\n',
                    b"134" => b'\\',
                    _ => return Err(Error::Unsupported),
                });
                index += 4;
            } else {
                decoded.push(bytes[index]);
                index += 1;
            }
        }
        if decoded.contains(&0)
            || decoded
                .split(|byte| *byte == b'/')
                .any(|part| part == b"..")
        {
            return Err(Error::Unsupported);
        }
        let path = PathBuf::from(OsString::from_vec(decoded));
        if !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return Err(Error::Unsupported);
        }
        Ok(path)
    }

    fn parse(table: &[u8]) -> Result<Vec<Mount>, Error> {
        if table.len() as u64 > MAX_TABLE_BYTES || !table.ends_with(b"\n") {
            return Err(Error::Unsupported);
        }
        let mut mounts = Vec::new();
        let mut ids = HashSet::new();
        for line in table
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
        {
            let fields: Vec<_> = line.split(|byte| *byte == b' ').collect();
            let separator = fields
                .iter()
                .position(|field| *field == b"-")
                .ok_or(Error::Unsupported)?;
            if separator < 6 || fields.len() != separator + 4 || mounts.len() >= MAX_MOUNTS {
                return Err(Error::Unsupported);
            }
            let id = number(fields[0])?;
            number(fields[1])?;
            if !ids.insert(id) {
                return Err(Error::Unsupported);
            }
            let mut device = fields[2].split(|byte| *byte == b':');
            let major = number(device.next().ok_or(Error::Unsupported)?)?;
            let minor = number(device.next().ok_or(Error::Unsupported)?)?;
            if device.next().is_some() {
                return Err(Error::Unsupported);
            }
            mounts.push(Mount {
                id,
                device: (major, minor),
                root: decode_path(fields[3])?,
                point: decode_path(fields[4])?,
                // Layered filesystems can show an underlying file under a new
                // device ID. nlink/device/inode comparison cannot rule that out.
                layered: matches!(fields[separator + 1], b"overlay" | b"aufs" | b"ecryptfs")
                    || fields[separator + 1].starts_with(b"fuse"),
            });
        }
        if mounts.is_empty() {
            return Err(Error::Unsupported);
        }
        Ok(mounts)
    }

    fn handle_location(dir: &Dir) -> Result<(u64, PathBuf), Error> {
        let file = dir
            .try_clone()
            .map_err(|_| Error::Unsupported)?
            .into_std_file();
        let descriptor = file.as_raw_fd();
        let path = std::fs::read_link(format!("/proc/self/fd/{descriptor}"))
            .map_err(|_| Error::Unsupported)?;
        // Use the live capability's location, never a possibly renamed or
        // replaced configuration pathname. Deleted/unresolvable roots fall back.
        let current = Dir::open_ambient_dir(&path, cap_std::ambient_authority())
            .map_err(|_| Error::Unsupported)?;
        if Identity::of(&file)? != Identity::of(&current.into_std_file())? {
            return Err(Error::Unsupported);
        }
        let info = std::fs::read(format!("/proc/self/fdinfo/{descriptor}"))
            .map_err(|_| Error::Unsupported)?;
        let id = info
            .split(|byte| *byte == b'\n')
            .find_map(|line| line.strip_prefix(b"mnt_id:\t"))
            .ok_or(Error::Unsupported)?;
        Ok((number(id)?, path))
    }

    fn origins(mounts: &[Mount], id: u64, path: &Path) -> Result<Vec<Origin>, Error> {
        let active = mounts
            .iter()
            .find(|mount| mount.id == id)
            .ok_or(Error::Unsupported)?;
        let relative = path
            .strip_prefix(&active.point)
            .map_err(|_| Error::Unsupported)?;
        let mut result = vec![Origin {
            device: active.device,
            path: active.root.join(relative),
            layered: active.layered,
        }];
        // Include every nested mount conservatively, even a covered mount.
        // This also covers bind-mounted regular files without extra tree I/O.
        result.extend(
            mounts
                .iter()
                .filter(|mount| mount.point != path && mount.point.starts_with(path))
                .map(|mount| Origin {
                    device: mount.device,
                    path: mount.root.clone(),
                    layered: mount.layered,
                }),
        );
        result.sort_unstable();
        result.dedup();
        Ok(result)
    }

    fn overlap(left: &[Origin], right: &[Origin]) -> bool {
        let (mut first, mut second) = (0, 0);
        while let (Some(a), Some(b)) = (left.get(first), right.get(second)) {
            if a.device == b.device && (a.path.starts_with(&b.path) || b.path.starts_with(&a.path))
            {
                return true;
            }
            if a < b {
                first += 1;
            } else {
                second += 1;
            }
        }
        false
    }

    fn layered_origins_uncertain(left: &[Origin], right: &[Origin]) -> bool {
        // Origins are sorted by device. A layered origin is comparable only
        // when every opposing origin belongs to that same filesystem instance.
        let uncertain = |a: &[Origin], b: &[Origin]| {
            a.iter().any(|origin| {
                origin.layered
                    && (b.first().is_some_and(|other| other.device != origin.device)
                        || b.last().is_some_and(|other| other.device != origin.device))
            })
        };
        uncertain(left, right) || uncertain(right, left)
    }

    pub(super) fn prove(root: &Dir, assets: &Dir) -> Result<(), Error> {
        let mut table = Vec::new();
        File::open("/proc/self/mountinfo")
            .map_err(|_| Error::Unsupported)?
            .take(MAX_TABLE_BYTES + 1)
            .read_to_end(&mut table)
            .map_err(|_| Error::Unsupported)?;
        let mounts = parse(&table)?;
        let (root_id, root_path) = handle_location(root)?;
        let (asset_id, asset_path) = handle_location(assets)?;
        let shared = origins(&mounts, root_id, &root_path)?;
        let public = origins(&mounts, asset_id, &asset_path)?;
        if overlap(&shared, &public) || layered_origins_uncertain(&shared, &public) {
            return Err(Error::Forbidden);
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const TABLE: &[u8] = b"1 0 8:1 / / rw - ext4 /dev/sda rw\n2 1 8:1 /share/private /assets/exposed rw - ext4 /dev/sda rw\n3 1 8:2 / /other rw - ext4 /dev/sdb rw\n";

        #[test]
        fn origins_compare_filesystem_subtrees_instead_of_visible_paths() {
            let mounts = parse(TABLE).unwrap();
            let shared = origins(&mounts, 1, Path::new("/share")).unwrap();
            assert!(overlap(
                &shared,
                &origins(&mounts, 1, Path::new("/assets")).unwrap()
            ));
            assert!(overlap(
                &shared,
                &origins(&mounts, 2, Path::new("/assets/exposed")).unwrap()
            ));
            assert!(!overlap(
                &shared,
                &origins(&mounts, 1, Path::new("/share-other")).unwrap()
            ));
            assert!(!overlap(
                &shared,
                &origins(&mounts, 3, Path::new("/other/share")).unwrap()
            ));
            assert!(origins(&mounts, 2, Path::new("/share")).is_err());
            assert!(origins(&mounts, 99, Path::new("/assets")).is_err());
        }

        #[test]
        fn mount_paths_preserve_kernel_escapes_and_non_utf8_bytes() {
            assert_eq!(
                decode_path(b"/space\\040tab\\011line\\012slash\\134\xff").unwrap(),
                PathBuf::from(OsString::from_vec(
                    b"/space tab\tline\nslash\\\xff".to_vec()
                ))
            );
            for invalid in [
                b"/bad\\777".as_slice(),
                b"relative",
                b"/../share",
                b"/bad\0",
            ] {
                assert!(decode_path(invalid).is_err());
            }
        }

        #[test]
        fn incomplete_or_ambiguous_tables_cannot_prove_separation() {
            for invalid in [
                b"".as_slice(),
                b"1 0 8:1 / / rw - ext4 dev rw",
                b"1 0 8:1 / / rw\n",
                b"1 0 8:1 / / rw - ext4 dev rw\n1 0 8:2 / /x rw - ext4 dev rw\n",
            ] {
                assert!(parse(invalid).is_err());
            }
        }

        #[test]
        fn layered_origins_cannot_hide_underlying_content_behind_new_device_ids() {
            let table = b"1 0 8:1 / / rw - ext4 /dev/sda rw\n2 1 0:2 / /assets/exposed rw - overlay overlay rw\n3 1 0:3 / /container rw - overlay overlay rw\n";
            let mounts = parse(table).unwrap();
            let shared = origins(&mounts, 1, Path::new("/share")).unwrap();
            let public = origins(&mounts, 1, Path::new("/assets")).unwrap();
            assert!(!overlap(&shared, &public));
            assert!(layered_origins_uncertain(&shared, &public));
            assert!(layered_origins_uncertain(&public, &shared));
            let inside_share = origins(&mounts, 3, Path::new("/container/share")).unwrap();
            let inside_public = origins(&mounts, 3, Path::new("/container/assets")).unwrap();
            assert!(!layered_origins_uncertain(&inside_share, &inside_public));
            assert!(!overlap(&inside_share, &inside_public));
        }
    }
}
