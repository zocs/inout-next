use super::*;

#[test]
fn finishing_stage_keeps_a_second_providers_acquired_container_attached() {
    for publish_a in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let destination = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).unwrap();
        let protection_a = Arc::new(ProtectedObjects::default());
        let protection_b = Arc::new(ProtectedObjects::default());
        std::fs::write(
            root.path().join("existing.bin"),
            b"original collision winner",
        )
        .unwrap();
        let mut a = Stage::new(&destination, &protection_a).unwrap();
        a.file.as_mut().unwrap().write_all(b"first upload").unwrap();
        if publish_a {
            a.publish("a.bin").unwrap();
        }

        // Force the exact setup window: B has acquired the container capability
        // but has not created its stage. A finishes or is cancelled before B's
        // create/register steps. This uses real capabilities, not production hooks.
        let acquired_b = destination.open_dir_nofollow("INOUTTMP").unwrap();
        drop(a);
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let b_file = acquired_b
            .open_with("B0000001", &options)
            .expect("A cleanup detached B's acquired container");
        let mut b = Stage {
            dir: acquired_b.try_clone().unwrap(),
            destination: destination.try_clone().unwrap(),
            name: "B0000001".into(),
            _protection: protection_b.register(&acquired_b, &b_file).unwrap(),
            file: Some(b_file),
            published: false,
        };
        b.file
            .as_mut()
            .unwrap()
            .write_all(b"second upload")
            .unwrap();
        b.publish("b.bin").unwrap();
        drop(b);

        let mut colliding = Stage::new(&destination, &protection_b).unwrap();
        colliding
            .file
            .as_mut()
            .unwrap()
            .write_all(b"must not overwrite")
            .unwrap();
        assert_eq!(colliding.publish("existing.bin"), Err(Error::TargetExists));
        drop(colliding);
        if publish_a {
            assert_eq!(
                std::fs::read(root.path().join("a.bin")).unwrap(),
                b"first upload"
            );
        } else {
            assert!(!root.path().join("a.bin").exists());
        }
        assert_eq!(
            std::fs::read(root.path().join("b.bin")).unwrap(),
            b"second upload"
        );
        assert_eq!(
            std::fs::read(root.path().join("existing.bin")).unwrap(),
            b"original collision winner"
        );
        assert_eq!(
            std::fs::read_dir(root.path().join("INOUTTMP"))
                .unwrap()
                .count(),
            0
        );
    }
}
