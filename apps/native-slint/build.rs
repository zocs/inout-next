use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    embed_web_export();
    #[cfg(feature = "gui")]
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new().with_style("fluent".into()),
    )
    .expect("compile the native sharing interface");
}

/// Release GUI builds require the static browser export. Only development or
/// renderer-free builds may use an empty table and directory-backed fallback.
fn embed_web_export() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let export = manifest.join("..").join("web").join("out");
    println!("cargo:rerun-if-changed={}", export.display());
    let complete = export.join("index.html").is_file() && export.join("_next").is_dir();
    let required = env::var("PROFILE").is_ok_and(|profile| profile == "release")
        && env::var_os("CARGO_FEATURE_GUI").is_some();
    assert!(complete || !required,
        "native release builds need a complete browser export; build apps/web before building apps/native-slint");
    let mut files = Vec::new();
    if complete {
        assert!(!export.is_symlink(), "the web export must not be a symlink");
        let mut bytes = 0;
        collect(&export, "", &mut files, &mut bytes, 0);
    }
    files.sort();
    let mut source = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for (relative, path) in &files {
        let path = path.to_str().expect("UTF-8 web export path");
        source.push_str(&format!("    ({relative:?}, include_bytes!({path:?})),\n"));
    }
    source.push_str("];\n");
    let output =
        Path::new(&env::var_os("OUT_DIR").expect("output directory")).join("web_bundle.rs");
    fs::write(output, source).expect("write the web bundle table");
}

fn collect(
    directory: &Path,
    prefix: &str,
    files: &mut Vec<(String, PathBuf)>,
    bytes: &mut u64,
    depth: usize,
) {
    assert!(
        depth <= 128,
        "the web export exceeds the directory depth limit"
    );
    for entry in fs::read_dir(directory).expect("read the web export") {
        let entry = entry.expect("read a web export entry");
        let name = entry
            .file_name()
            .into_string()
            .expect("UTF-8 web export name");
        let relative = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type().expect("web export entry type");
        if kind.is_dir() {
            collect(&entry.path(), &relative, files, bytes, depth + 1);
        } else if kind.is_file() {
            *bytes = bytes.saturating_add(entry.metadata().expect("web export file size").len());
            assert!(
                files.len() < 10_000 && *bytes <= 128 * 1024 * 1024,
                "the web export exceeds the file count or byte limit"
            );
            files.push((relative, entry.path()));
        } else {
            panic!("the web export must contain only regular files and directories");
        }
    }
}
