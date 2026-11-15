use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_share-server"))
}

#[test]
fn root_is_required_and_an_explicit_blank_key_is_rejected() {
    let missing_root = cli().output().unwrap();
    assert!(!missing_root.status.success());
    let root = tempfile::tempdir().unwrap();
    let key = root.path().join("blank-key");
    std::fs::write(&key, " \r\n").unwrap();
    let mut blank_key = cli()
        .args([
            "--root",
            root.path().to_str().unwrap(),
            "--listen",
            "127.0.0.1:0",
            "--writable",
            "--key-file",
            key.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while blank_key.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            let _ = blank_key.kill();
            let _ = blank_key.wait();
            panic!("an explicitly invalid key must fail before starting sharing");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let blank_key = blank_key.wait_with_output().unwrap();
    assert!(!blank_key.status.success());
    assert!(
        blank_key.stdout.is_empty(),
        "an invalid key must not start sharing"
    );
}

#[test]
fn help_exposes_the_host_configuration_flags() {
    let output = cli().arg("--help").output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for option in [
        "--root",
        "--listen",
        "--web-dir",
        "--key-file",
        "--writable",
        "--modules",
        "--max-upload-bytes",
        "--product-file",
    ] {
        assert!(help.contains(option), "{option}");
    }
}

#[cfg(unix)]
#[test]
fn alternate_product_is_served_and_ctrl_c_releases_listener_without_logging_key() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::{
        net::TcpStream,
        process::Stdio,
        time::{Duration, Instant},
    };
    let root = tempfile::tempdir().unwrap();
    let product = root.path().join("alternate.json");
    std::fs::write(&product, r#"{"name":"Alternate CLI","slug":"alternate","description":"Test","version":"1","application_id":"test.alternate","repository":"https://example.com"}"#).unwrap();
    let key = root.path().join("key");
    std::fs::write(&key, "secret-never-printed\n").unwrap();
    let mut child = cli()
        .args([
            "--root",
            root.path().to_str().unwrap(),
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            key.to_str().unwrap(),
            "--product-file",
            product.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    assert!(line.starts_with("Listening on http://"), "{line}");
    assert!(!line.contains("secret-never-printed"));
    let address = line.trim().strip_prefix("Listening on http://").unwrap();
    let mut connection = TcpStream::connect(address).unwrap();
    connection
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    connection
        .write_all(b"GET /api/v1/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    connection.read_to_string(&mut response).unwrap();
    assert!(response.contains("Alternate CLI"));
    assert!(response.contains("\"auth_required\":true"));
    assert!(Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap()
        .success());
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("CLI did not stop on Ctrl-C");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(!stderr.contains("secret-never-printed"));
    let listener = std::net::TcpListener::bind(address).unwrap();
    drop(listener);
}
