//! Real mount aliases, isolated from the caller's mount namespace.
//! Run explicitly on Linux with unshare/mount and unprivileged user namespaces.
#![cfg(target_os = "linux")]

use share_domain::Error;
use share_storage::{FileStore, FsStore};
use std::{fs, os::unix::fs::MetadataExt, path::Path, process::Command};

#[test]
#[ignore = "requires Linux unprivileged user/mount namespaces and mount(8)"]
fn public_assets_reject_bind_aliases_in_both_directions() {
    in_namespace(
        "public_assets_reject_bind_aliases_in_both_directions",
        exercise_mounts,
    );
}

#[test]
#[ignore = "requires Linux unprivileged user/mount namespaces and overlayfs"]
fn public_assets_reject_layered_aliases_with_distinct_device_ids() {
    in_namespace(
        "public_assets_reject_layered_aliases_with_distinct_device_ids",
        exercise_layers,
    );
}

fn in_namespace(test: &str, fixture: fn(&Path)) {
    const CHILD: &str = "INOUT_ASSET_MOUNT_FIXTURE";
    if let Some(directory) = std::env::var_os(CHILD) {
        fixture(Path::new(&directory));
        return;
    }
    let temporary = tempfile::tempdir().unwrap();
    let result = Command::new("unshare")
        .args(["--user", "--map-root-user", "--mount"])
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", test, "--ignored", "--nocapture"])
        .env(CHILD, temporary.path())
        .output()
        .expect("run this test with unshare installed");
    assert!(
        result.status.success(),
        "isolated mount regression failed:\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );
    // The child namespace is gone before TempDir removes any fixture paths.
}

fn overlay(lower: &Path, destination: &Path, scratch: &Path) {
    let upper = scratch.join("upper");
    let work = scratch.join("work");
    fs::create_dir_all(&upper).unwrap();
    fs::create_dir(&work).unwrap();
    let result = Command::new("mount")
        .args(["-t", "overlay", "overlay", "-o"])
        .arg(format!(
            "lowerdir={},upperdir={},workdir={},userxattr",
            lower.display(),
            upper.display(),
            work.display()
        ))
        .arg(destination)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "overlay fixture failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
}

fn exercise_layers(parent: &Path) {
    let share = parent.join("share");
    let assets = parent.join("assets");
    fs::create_dir_all(share.join("private")).unwrap();
    fs::create_dir_all(assets.join("exposed")).unwrap();
    fs::write(share.join("private/secret.html"), b"private layer bytes").unwrap();
    fs::write(assets.join("index.html"), b"trusted index").unwrap();
    overlay(
        &share.join("private"),
        &assets.join("exposed"),
        &parent.join("scratch"),
    );
    let private = fs::metadata(share.join("private/secret.html")).unwrap();
    let exposed = fs::metadata(assets.join("exposed/secret.html")).unwrap();
    assert_ne!(private.dev(), exposed.dev());
    assert_eq!(private.nlink(), 1);
    assert!(
        matches!(
            FsStore::new(&share).unwrap().web_assets(&assets),
            Err(Error::Forbidden)
        ),
        "accepted private layer bytes under a different device identity"
    );

    // Sibling paths within one overlay namespace still have comparable origins.
    let lower = parent.join("container-lower");
    let merged = parent.join("container");
    fs::create_dir_all(lower.join("share")).unwrap();
    fs::create_dir_all(lower.join("assets")).unwrap();
    fs::create_dir(&merged).unwrap();
    fs::write(lower.join("assets/index.html"), b"container index").unwrap();
    overlay(&lower, &merged, &parent.join("container-scratch"));
    let files = FsStore::new(merged.join("share"))
        .unwrap()
        .web_assets(&merged.join("assets"))
        .expect("separate sibling paths within one overlay mount remain usable");
    assert_eq!(files[0].bytes, b"container index");
}

fn bind(source: &Path, destination: &Path) {
    assert!(Command::new("mount")
        .arg("--bind")
        .arg(source)
        .arg(destination)
        .status()
        .unwrap()
        .success());
}

fn exercise_mounts(parent: &Path) {
    for case in [
        "shared-directory",
        "shared-file",
        "assets-in-share",
        "unrelated",
    ] {
        // Spaces/backslashes are escaped in mountinfo, and Unicode stays bytes.
        let fixture = parent.join(format!("{case} space\\界"));
        let share = fixture.join("share");
        let assets = fixture.join("assets");
        fs::create_dir_all(share.join("private")).unwrap();
        fs::create_dir_all(assets.join("public")).unwrap();
        fs::write(share.join("private/secret.html"), b"private bytes").unwrap();
        fs::write(assets.join("index.html"), b"trusted index").unwrap();
        fs::write(assets.join("public/app.js"), b"trusted script").unwrap();
        match case {
            "shared-directory" => {
                fs::create_dir(assets.join("exposed")).unwrap();
                bind(&share.join("private"), &assets.join("exposed"));
            }
            "shared-file" => {
                fs::write(assets.join("exposed.html"), b"placeholder").unwrap();
                bind(
                    &share.join("private/secret.html"),
                    &assets.join("exposed.html"),
                );
            }
            "assets-in-share" => {
                fs::create_dir(share.join("alias")).unwrap();
                bind(&assets.join("public"), &share.join("alias"));
            }
            "unrelated" => {
                let outside = fixture.join("outside");
                fs::create_dir(&outside).unwrap();
                fs::write(outside.join("allowed.txt"), b"public bytes").unwrap();
                fs::create_dir(assets.join("extra")).unwrap();
                bind(&outside, &assets.join("extra"));
            }
            _ => unreachable!(),
        }
        assert_eq!(
            fs::metadata(share.join("private/secret.html"))
                .unwrap()
                .nlink(),
            1
        );
        let store = FsStore::new(&share).unwrap();
        let result = store.web_assets(&assets);
        if case == "unrelated" {
            let files = result.expect("an unrelated public mount remains usable");
            assert!(files
                .iter()
                .any(|file| file.path == "extra/allowed.txt" && file.bytes == b"public bytes"));
        } else {
            assert!(matches!(result, Err(Error::Forbidden)), "accepted {case}");
        }
    }
}
