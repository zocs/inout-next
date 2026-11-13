use share_domain::{Error, ModuleRegistry, Product, RelativePath};

#[test]
fn portable_paths_preserve_unicode_and_reject_escape_and_windows_special_names() {
    assert_eq!(
        RelativePath::parse("文档/a.txt", false).unwrap().as_str(),
        "文档/a.txt"
    );
    assert_eq!(RelativePath::parse("", true).unwrap().as_str(), "");
    for path in [
        "",
        "/x",
        "../x",
        "a/../x",
        "a/./x",
        "a//x",
        "a/",
        "a\\b",
        "C:x",
        "a\0b",
        "CON",
        "a/NUL.txt",
        "a. ",
        ".inout-stage-hidden",
        ".INOUT-STAGE-hidden",
        "dir/.InOuT-StAgE-hidden",
        "INOUTTMP",
        "dir/inouttmp/secret",
        "INOUTS~1.BIN",
    ] {
        assert_eq!(
            RelativePath::parse(path, false),
            Err(Error::InvalidPath),
            "{path:?}"
        );
    }
}

#[test]
fn portable_paths_reject_all_superscript_com_lpt_devices_in_each_component() {
    for prefix in ["COM", "LPT", "cOm", "lPt"] {
        for digit in ['¹', '²', '³'] {
            for path in [
                format!("{prefix}{digit}"),
                format!("{prefix}{digit}.txt"),
                format!("目录/{prefix}{digit}.txt/child"),
            ] {
                assert_eq!(
                    RelativePath::parse(&path, false),
                    Err(Error::InvalidPath),
                    "{path:?}"
                );
            }
        }
    }
    for path in [
        "文档/数字¹²³.txt",
        "COM10",
        "LPT10.txt",
        "目录/cOm10/lPt10.txt",
    ] {
        assert_eq!(RelativePath::parse(path, false).unwrap().as_str(), path);
    }
}

#[test]
fn module_dependencies_and_permissions_are_enforced() {
    let modules = ModuleRegistry::new(&["files".into(), "uploads".into()]).unwrap();
    assert!(modules.permissions(true).upload);
    assert!(!modules.permissions(false).upload);
    assert!(!modules.permissions(true).manage);
    assert!(ModuleRegistry::new(&["uploads".into()]).is_err());
    assert!(ModuleRegistry::new(&["unknown".into()]).is_err());
    assert!(ModuleRegistry::new(&[]).unwrap().enabled().is_empty());
}

#[test]
fn alternate_product_manifest_changes_public_identity() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("product.json");
    std::fs::write(&path, r#"{"name":"Alternate","slug":"test","description":"A test","version":"1","application_id":"test.app","repository":"https://example.com"}"#).unwrap();
    let product = Product::load(path).unwrap();
    assert_eq!(product.name, "Alternate");
    assert_eq!(product.application_id, "test.app");
}
