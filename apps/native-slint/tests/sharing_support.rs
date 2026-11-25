use share_native_slint::{
    folders::{common, expand, parse_user_dirs, CommonFolder},
    preferences::Preferences,
    presentation::{format_bytes, Access, Locale},
    qr::{Module, Symbol, QUIET_ZONE},
};
use std::path::{Path, PathBuf};

#[test]
fn share_addresses_encode_to_a_small_symbol_with_intact_finders() {
    for address in [
        "http://127.0.0.1:5000",
        "http://192.168.1.173:5000",
        "http://192.168.100.200:65535",
    ] {
        let symbol = Symbol::encode(address).unwrap();
        assert_eq!(symbol.size(), 29, "{address} fits version 3 at quartile");
        assert_eq!(symbol, Symbol::encode(address).unwrap(), "deterministic");
        let far = symbol.size() - 7;
        assert_eq!(
            symbol.finders(),
            [
                Module { x: 0, y: 0 },
                Module { x: far, y: 0 },
                Module { x: 0, y: far }
            ]
        );
        for corner in symbol.finders() {
            for offset in 0..7 {
                assert!(symbol.is_dark(corner.x + offset, corner.y), "outer ring");
                assert!(symbol.is_dark(corner.x, corner.y + offset), "outer ring");
                assert!(symbol.is_dark(corner.x + offset, corner.y + 6));
                assert!(symbol.is_dark(corner.x + 6, corner.y + offset));
            }
            assert!(!symbol.is_dark(corner.x + 1, corner.y + 1), "light ring");
            assert!(symbol.is_dark(corner.x + 3, corner.y + 3), "centre");
        }
        let data = symbol.data_modules();
        assert!(!data.is_empty());
        assert!(data.iter().all(|module| symbol.is_dark(module.x, module.y)));
        assert!(data.iter().all(|module| {
            symbol.finders().iter().all(|corner| {
                !((corner.x..corner.x + 7).contains(&module.x)
                    && (corner.y..corner.y + 7).contains(&module.y))
            })
        }));
        let all_dark = (0..symbol.size())
            .flat_map(|y| (0..symbol.size()).map(move |x| (x, y)))
            .filter(|(x, y)| symbol.is_dark(*x, *y))
            .count();
        assert_eq!(
            all_dark,
            data.len() + 3 * 33,
            "finders hold 33 dark modules each"
        );
    }
    assert!(!Symbol::encode("x").unwrap().is_dark(-1, 0));
    assert!(Symbol::encode(&"a".repeat(4000)).is_none());
    assert_eq!(QUIET_ZONE, 4);
}

#[test]
fn a_typed_tilde_means_the_home_folder_and_nothing_else_is_rewritten() {
    let home = Path::new("/home/someone");
    assert_eq!(expand("~", Some(home)), home);
    assert_eq!(expand("  ~  ", Some(home)), home);
    assert_eq!(expand("~/Downloads", Some(home)), home.join("Downloads"));
    assert_eq!(expand("~//Downloads", Some(home)), home.join("Downloads"));
    assert_eq!(expand("~\\Desktop", Some(home)), home.join("Desktop"));
    assert_eq!(
        expand("~other/files", Some(home)),
        PathBuf::from("~other/files")
    );
    assert_eq!(
        expand("/srv/~/files", Some(home)),
        PathBuf::from("/srv/~/files")
    );
    assert_eq!(expand("~", None), PathBuf::from("~"));
}

#[test]
fn common_folders_follow_localized_desktop_configuration_and_must_exist() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    std::fs::create_dir(home.join("下载")).unwrap();
    std::fs::create_dir(home.join("Documents")).unwrap();
    std::fs::create_dir(home.join("Downloads")).unwrap();
    let configuration = "# comment\nXDG_DESKTOP_DIR=\"$HOME/\"\nXDG_DOWNLOAD_DIR=\"$HOME/下载\"\n\
        XDG_DOCUMENTS_DIR=\"$HOME/文档\"\nXDG_MUSIC_DIR=\"$HOME/Music\"\nXDG_PICTURES_DIR=relative\n";
    assert_eq!(
        parse_user_dirs(configuration, home),
        [
            (CommonFolder::Downloads, home.join("下载")),
            (CommonFolder::Documents, home.join("文档")),
        ]
    );
    assert_eq!(
        common(home, Some(configuration)),
        [
            (CommonFolder::Home, home.to_path_buf()),
            // The configured 文档 does not exist, so the conventional name is used.
            (CommonFolder::Documents, home.join("Documents")),
            (CommonFolder::Downloads, home.join("下载")),
        ]
    );
    assert_eq!(
        common(home, None),
        [
            (CommonFolder::Home, home.to_path_buf()),
            (CommonFolder::Documents, home.join("Documents")),
            (CommonFolder::Downloads, home.join("Downloads")),
        ]
    );
}

#[test]
fn preferences_round_trip_without_a_key_and_ignore_invalid_values() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("nested/native-preferences.conf");
    assert_eq!(Preferences::load(&path), Preferences::default());
    let saved = Preferences {
        locale: Some(Locale::Traditional),
        accent: 2,
        appearance: 2,
        folder: "/home/someone/简体 = files".into(),
        port: "0".into(),
        network: false,
        access: Access::Manage,
    };
    saved.save(&path).unwrap();
    assert_eq!(Preferences::load(&path), saved);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("key"), "no credential field exists");
    assert!(!path.with_extension("conf.tmp").exists());

    let hostile = Preferences::parse(
        "accent=9\nappearance=-1\nport=99999\nlocale=fr\naccess=root\nunknown=1\nfolder=/a=b\nnetwork=maybe\n",
    );
    assert_eq!(
        hostile,
        Preferences {
            folder: "/a=b".into(),
            ..Preferences::default()
        }
    );
    let multiline = Preferences {
        folder: "line\nbreak".into(),
        ..Preferences::default()
    };
    assert_eq!(
        Preferences::parse(&multiline.serialize()),
        Preferences::default()
    );
    std::fs::write(&path, vec![b'a'; 20_000]).unwrap();
    assert_eq!(Preferences::load(&path), Preferences::default());
}

#[test]
fn preference_saves_preserve_unrelated_siblings() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("prefs.conf");
    let sibling = temp.path().join("prefs.conf.tmp");
    std::fs::write(&sibling, b"unrelated existing file").unwrap();
    Preferences::default().save(&path).unwrap();
    assert_eq!(std::fs::read(&sibling).unwrap(), b"unrelated existing file");
    assert_eq!(Preferences::load(&path), Preferences::default());
}

#[test]
fn concurrent_instances_publish_complete_preferences_without_stage_collisions() {
    use std::sync::{Arc, Barrier};
    let temp = tempfile::tempdir().unwrap();
    let path = Arc::new(temp.path().join("prefs.conf"));
    let barrier = Arc::new(Barrier::new(8));
    let candidates: Vec<_> = (0..8)
        .map(|index| Preferences {
            folder: format!("/folder-{index}/{}", "界".repeat(1000 + index * 100)),
            network: index % 2 == 0,
            access: if index % 2 == 0 {
                Access::Manage
            } else {
                Access::ReadOnly
            },
            ..Preferences::default()
        })
        .collect();
    let workers: Vec<_> = candidates
        .iter()
        .cloned()
        .map(|preferences| {
            let path = path.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut failures = Vec::new();
                for _ in 0..32 {
                    barrier.wait();
                    if let Err(error) = preferences.save(&path) {
                        failures.push(error);
                    }
                }
                failures
            })
        })
        .collect();
    let failures: Vec<_> = workers
        .into_iter()
        .flat_map(|worker| worker.join().unwrap())
        .collect();
    assert!(
        failures.is_empty(),
        "{} save errors; first: {:?}",
        failures.len(),
        failures.first()
    );
    assert!(
        candidates.contains(&Preferences::load(&path)),
        "the winner must be one whole submitted setup"
    );
    assert_eq!(
        std::fs::read_dir(temp.path()).unwrap().count(),
        1,
        "owned temporary files are cleaned up"
    );
}

#[test]
fn access_presets_map_to_service_modules_and_sizes_read_like_a_file_manager() {
    assert_eq!(
        [Access::ReadOnly, Access::Upload, Access::Manage]
            .map(|access| (access.uploads(), access.manage())),
        [(false, false), (true, false), (true, true)]
    );
    for index in 0..3 {
        assert_eq!(Access::from_index(index).index(), index);
    }
    assert_eq!(Access::from_index(7), Access::ReadOnly);
    assert_eq!(
        [
            0,
            1023,
            1024,
            1536,
            104_857,
            1_048_576,
            2_147_483_648,
            1_048_575,
            107_374_182_400
        ]
        .map(format_bytes),
        ["0 B", "1023 B", "1.0 KB", "1.5 KB", "102 KB", "1.0 MB", "2.0 GB", "1.0 MB", "100 GB"]
    );
}

#[test]
fn the_browser_manager_is_compiled_into_the_executable_when_it_has_been_built() {
    use share_native_slint::web;
    let export = Path::new(env!("CARGO_MANIFEST_DIR")).join("../web/out/index.html");
    if export.is_file() {
        let bundle = web::bundle().expect("a built export is embedded");
        assert_eq!(bundle.files(), web::FILES.len());
        assert!(web::FILES.iter().any(|(path, _)| *path == "index.html"));
        assert!(web::FILES
            .iter()
            .any(|(path, _)| path.starts_with("_next/")));
        assert!(web::FILES.iter().all(|(path, _)| !path.contains('\\')));
        assert_eq!(
            web::FILES
                .iter()
                .find(|(path, _)| *path == "index.html")
                .unwrap()
                .1,
            std::fs::read(export).unwrap()
        );
    } else {
        assert!(web::bundle().is_none());
    }
}
