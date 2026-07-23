use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use replmux_runtime_cli::jupyter::{JupyterClient, JupyterConnection};

fn command(kernel_dir: &PathBuf) -> Command {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest_dir.parent().unwrap();
    let python = manifest_dir.join("tests/.venv/bin/python");
    let python = if python.exists() {
        python
    } else {
        PathBuf::from("python3")
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_replmux"));
    command
        .arg("--kernel-dir")
        .arg(kernel_dir)
        .arg("--python")
        .arg(python)
        .arg("--kernel-script")
        .arg(manifest_dir.join("assets/minimal_kernel_clean.py"))
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

    let connection: JupyterConnection =
        serde_json::from_slice(&fs::read(kernel_dir.join(format!("{name}.json"))).unwrap())
            .unwrap();
    let mut jupyter = JupyterClient::connect(&connection).unwrap();
    assert!(jupyter.heartbeat(Duration::from_secs(2)).unwrap());
    let info = jupyter.kernel_info(Duration::from_secs(2)).unwrap();
    assert_eq!(info.content["implementation"], "minimal_kernel");
    jupyter
        .execute("jupyter_value = 6", Duration::from_secs(5))
        .unwrap();
    let execution = jupyter
        .execute("jupyter_value * 7", Duration::from_secs(5))
        .unwrap();
    assert_eq!(execution.reply.content["status"], "ok");
    assert!(execution.outputs.iter().any(|message| {
        message.message_type() == Some("execute_result")
            && message.content["data"]["text/plain"] == "42"
    }));
    let completion = jupyter
        .complete("jupyter_value.bi", None, Duration::from_secs(2))
        .unwrap();
    assert!(
        completion.content["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("jupyter_value.bit_length()"))
    );
    let inspection = jupyter
        .inspect("jupyter_value.bit_length", None, 0, Duration::from_secs(2))
        .unwrap();
    assert_eq!(inspection.content["found"], true);
    let completeness = jupyter
        .is_complete("for value in values:", Duration::from_secs(2))
        .unwrap();
    assert_eq!(completeness.content["status"], "incomplete");

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
fn launches_kernelspec_and_attaches_standard_connection() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let kernel_dir = PathBuf::from(format!("/tmp/mr-spec-{}-{unique}", std::process::id()));
    let name = "kernelspec-lifecycle".to_owned();
    let _cleanup = KernelCleanup {
        directory: kernel_dir.clone(),
        name: name.clone(),
    };
    fs::create_dir_all(&kernel_dir).unwrap();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let python = manifest_dir.join("tests/.venv/bin/python");
    let kernel_script = manifest_dir.join("assets/minimal_kernel_clean.py");
    let launcher = "import os,runpy,sys; os.environ['KERNEL_CONNECTION_FILE']=sys.argv[1]; runpy.run_path(sys.argv[2],run_name='__main__')";
    let spec_path = kernel_dir.join("kernel.json");
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "argv": [python, "-c", launcher, "{connection_file}", kernel_script],
            "display_name": "Replmux test kernel",
            "language": "python"
        }))
        .unwrap(),
    )
    .unwrap();

    let create = command(&kernel_dir)
        .args(["kernel", "create", &name, "--kernelspec"])
        .arg(&spec_path)
        .output()
        .unwrap();
    assert!(
        create.status.success(),
        "{}",
        String::from_utf8_lossy(&create.stderr)
    );

    let connection_path = kernel_dir.join(format!("{name}.json"));
    let mut connection: serde_json::Value =
        serde_json::from_slice(&fs::read(&connection_path).unwrap()).unwrap();
    connection.as_object_mut().unwrap().remove("socket_path");
    let external_path = kernel_dir.join("external.json");
    fs::write(
        &external_path,
        serde_json::to_vec_pretty(&connection).unwrap(),
    )
    .unwrap();

    let attach = command(&kernel_dir)
        .args(["kernel", "attach", "attached"])
        .arg(&external_path)
        .output()
        .unwrap();
    assert!(attach.status.success());
    let execute = command(&kernel_dir)
        .args(["kernel", "exec", "attached", "21 * 2"])
        .output()
        .unwrap();
    assert!(execute.status.success());
    assert_eq!(String::from_utf8_lossy(&execute.stdout).trim(), "42");
    assert!(
        command(&kernel_dir)
            .args(["kernel", "delete", "attached"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        command(&kernel_dir)
            .args(["kernel", "delete", &name])
            .output()
            .unwrap()
            .status
            .success()
    );
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
