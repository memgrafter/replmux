use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
        .arg(repository.join("minimal_kernel_clean.py"))
        .arg("--broker-socket")
        .arg(kernel_dir.join("broker.sock"));
    command
}

struct KernelCleanup {
    directory: PathBuf,
    name: String,
}

struct BrokerCleanup {
    child: Child,
    directory: PathBuf,
}

impl Drop for KernelCleanup {
    fn drop(&mut self) {
        let _ = command(&self.directory)
            .args(["delete", &self.name])
            .output();
        let _ = fs::remove_dir_all(&self.directory);
    }
}

impl Drop for BrokerCleanup {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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

#[test]
fn serves_kernel_lifecycle_over_unix_socket() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let kernel_dir = PathBuf::from(format!("/tmp/mr-broker-{}-{unique}", std::process::id()));
    let socket_path = kernel_dir.join("broker.sock");
    let child = command(&kernel_dir)
        .arg("serve")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut cleanup = BrokerCleanup {
        child,
        directory: kernel_dir.clone(),
    };

    for _ in 0..50 {
        if socket_path.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(socket_path.exists(), "broker socket was not ready");

    let name = "broker-lifecycle";
    let create = command(&kernel_dir)
        .args(["--transport", "socket", "create", name])
        .output()
        .unwrap();
    assert!(
        create.status.success(),
        "{}",
        String::from_utf8_lossy(&create.stderr)
    );

    let assign = command(&kernel_dir)
        .args(["--transport", "socket", "exec", name, "broker_value = 21"])
        .output()
        .unwrap();
    assert!(assign.status.success());
    let evaluate = command(&kernel_dir)
        .args(["--transport", "socket", "exec", name, "broker_value * 2"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&evaluate.stdout).trim(), "42");

    let delete = command(&kernel_dir)
        .args(["--transport", "socket", "delete", name])
        .output()
        .unwrap();
    assert!(delete.status.success());

    cleanup.child.kill().unwrap();
    cleanup.child.wait().unwrap();
}
