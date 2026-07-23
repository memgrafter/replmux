use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn command(kernel_dir: &PathBuf) -> Command {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir.parent().unwrap();
    let python = repository.join(".venv/bin/python");
    let python = if python.exists() {
        python
    } else {
        PathBuf::from("python3")
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_multirepl"));
    command
        .arg("--kernel-dir")
        .arg(kernel_dir)
        .arg("--python")
        .arg(python)
        .arg("--kernel-script")
        .arg(repository.join("minimal_kernel_clean.py"));
    command
}

struct KernelCleanup {
    directory: PathBuf,
    name: String,
}

impl Drop for KernelCleanup {
    fn drop(&mut self) {
        let _ = command(&self.directory)
            .args(["delete", &self.name])
            .output();
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn manages_kernel_and_executes_persistent_code() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let kernel_dir = PathBuf::from(format!("/tmp/mr-{}-{unique}", std::process::id()));
    let name = "lifecycle".to_owned();
    let _cleanup = KernelCleanup {
        directory: kernel_dir.clone(),
        name: name.clone(),
    };

    let create = command(&kernel_dir)
        .args(["create", &name])
        .output()
        .unwrap();
    assert!(
        create.status.success(),
        "{}",
        String::from_utf8_lossy(&create.stderr)
    );

    let list = command(&kernel_dir).arg("list").output().unwrap();
    assert!(list.status.success());
    assert!(String::from_utf8_lossy(&list.stdout).contains(&name));

    let connect = command(&kernel_dir)
        .args(["connect", &name])
        .output()
        .unwrap();
    assert!(connect.status.success());
    assert!(String::from_utf8_lossy(&connect.stdout).contains("socket_path"));

    let assign = command(&kernel_dir)
        .args(["exec", &name, "answer = 42"])
        .output()
        .unwrap();
    assert!(
        assign.status.success(),
        "{}",
        String::from_utf8_lossy(&assign.stderr)
    );

    let evaluate = command(&kernel_dir)
        .args(["exec", &name, "answer"])
        .output()
        .unwrap();
    assert!(evaluate.status.success());
    assert_eq!(String::from_utf8_lossy(&evaluate.stdout).trim(), "42");

    let duplicate = command(&kernel_dir)
        .args(["create", &name])
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("already running"));

    let delete = command(&kernel_dir)
        .args(["delete", &name])
        .output()
        .unwrap();
    assert!(
        delete.status.success(),
        "{}",
        String::from_utf8_lossy(&delete.stderr)
    );

    let missing = command(&kernel_dir)
        .args(["connect", &name])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("not found"));
}
