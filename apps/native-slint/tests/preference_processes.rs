//! Exercise publication through separate app processes, with no renderer.
use share_native_slint::{
    preferences::Preferences,
    presentation::{Access, Locale},
};
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::Duration,
};

const PROCESSES: usize = 8;
const ROUNDS: usize = 32;

fn candidate(index: usize) -> Preferences {
    Preferences {
        locale: Some([Locale::Simplified, Locale::English, Locale::Traditional][index % 3]),
        accent: (index % 4) as i32,
        appearance: (index % 3) as i32,
        folder: format!("/folder-{index}/{}", "界".repeat(1000 + index * 100)),
        port: (5000 + index).to_string(),
        network: index.is_multiple_of(2),
        access: if index.is_multiple_of(2) {
            Access::Manage
        } else {
            Access::ReadOnly
        },
    }
}

struct Writer {
    child: Child,
    input: ChildStdin,
    ready: Receiver<()>,
    reader: Option<JoinHandle<io::Result<()>>>,
}

impl Drop for Writer {
    fn drop(&mut self) {
        // Failure at any barrier must also release all owned subprocesses.
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn spawn(path: &Path, index: usize) -> io::Result<Writer> {
    let mut child = Command::new(std::env::current_exe()?)
        .arg("--writer")
        .arg(path)
        .arg(index.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    let (send, ready) = mpsc::channel();
    let reader = thread::spawn(move || {
        for _ in 0..ROUNDS {
            let mut marker = [0];
            output.read_exact(&mut marker)?;
            if marker != *b"R" {
                return Err(io::Error::other("unexpected writer readiness marker"));
            }
            send.send(())
                .map_err(|_| io::Error::other("publication coordinator closed"))?;
        }
        Ok(())
    });
    Ok(Writer {
        child,
        input,
        ready,
        reader: Some(reader),
    })
}

fn write(path: &Path, index: usize) -> io::Result<()> {
    let preferences = candidate(index);
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut failed = 0;
    let mut first = None;
    for _ in 0..ROUNDS {
        output.write_all(b"R")?;
        output.flush()?;
        let mut marker = [0];
        input.read_exact(&mut marker)?;
        if marker != *b"G" {
            return Err(io::Error::other("unexpected publication barrier marker"));
        }
        if let Err(error) = preferences.save(path) {
            failed += 1;
            first.get_or_insert(error);
        }
    }
    if failed > 0 {
        return Err(io::Error::other(format!(
            "{failed} independent-process saves failed; first: {first:?}"
        )));
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let arguments: Vec<_> = std::env::args_os().collect();
    if arguments.get(1).is_some_and(|value| value == "--writer") {
        assert_eq!(arguments.len(), 4);
        let index: usize = arguments[3].to_str().unwrap().parse().unwrap();
        assert!(index < PROCESSES);
        return write(&PathBuf::from(&arguments[2]), index);
    }
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("prefs.conf");
    let mut writers: Vec<_> = (0..PROCESSES)
        .map(|index| {
            let path = if index.is_multiple_of(2) {
                path.clone()
            } else {
                directory.path().join(".").join("prefs.conf")
            };
            spawn(&path, index)
        })
        .collect::<io::Result<_>>()?;
    for _ in 0..ROUNDS {
        for writer in &writers {
            writer
                .ready
                .recv_timeout(Duration::from_secs(10))
                .expect("a writer failed or did not reach the publication barrier");
        }
        for writer in &mut writers {
            writer.input.write_all(b"G")?;
            writer.input.flush()?;
        }
    }
    for writer in &mut writers {
        assert!(writer.child.wait()?.success(), "a preference writer failed");
        writer.reader.take().unwrap().join().unwrap()?;
    }
    drop(writers);
    let winner = Preferences::load(&path);
    assert!((0..PROCESSES).any(|index| candidate(index) == winner));
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
    assert!(!std::fs::read_to_string(path)?.contains("key"));
    println!("Independent-process preference publication: 8 processes, 32 rounds; complete winner, directory aliases and cleaned staging verified");
    Ok(())
}
