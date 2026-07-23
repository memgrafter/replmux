use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::jupyter::{self, JupyterClient, JupyterMessage};
use crate::kernelspec;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const EXECUTION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct KernelManager {
    directory: PathBuf,
    python: PathBuf,
    kernel_script: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct KernelStatus {
    pub name: String,
    pub pid: Option<u32>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReplResponse {
    pub ok: bool,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub outputs: Vec<JupyterMessage>,
}

impl KernelManager {
    pub fn from_options(
        directory: Option<PathBuf>,
        python: Option<PathBuf>,
        kernel_script: Option<PathBuf>,
    ) -> Result<Self, String> {
        let directory = directory.unwrap_or_else(default_kernel_directory);
        let python = python
            .or_else(|| env::var_os("REPLMUX_PYTHON").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("python3"));
        let kernel_script = kernel_script
            .or_else(|| env::var_os("REPLMUX_KERNEL_SCRIPT").map(PathBuf::from))
            .unwrap_or_else(default_kernel_script);
        Ok(Self {
            directory,
            python,
            kernel_script,
        })
    }

    pub fn create(&self, name: &str) -> Result<u32, String> {
        validate_name(name)?;
        fs::create_dir_all(&self.directory)
            .map_err(|error| format!("cannot create {}: {error}", self.directory.display()))?;

        let connection_path = self.connection_path(name);
        let pid_path = self.pid_path(name);
        if connection_path.exists() {
            if let Some(pid) = read_pid(&pid_path)? {
                if process_is_alive(pid) {
                    return Err(format!("kernel '{name}' is already running (pid {pid})"));
                }
            }
            self.remove_artifacts(name);
        }

        if !self.kernel_script.exists() {
            return Err(format!(
                "kernel script not found: {} (set --kernel-script or REPLMUX_KERNEL_SCRIPT)",
                self.kernel_script.display()
            ));
        }

        let mut child = Command::new(&self.python)
            .arg(&self.kernel_script)
            .env("KERNEL_CONNECTION_FILE", &connection_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                format!(
                    "failed to start kernel with {}: {error}",
                    self.python.display()
                )
            })?;
        let pid = child.id();
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            if connection_path.exists() {
                fs::write(&pid_path, pid.to_string())
                    .map_err(|error| format!("cannot write {}: {error}", pid_path.display()))?;
                return Ok(pid);
            }
            if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                self.remove_artifacts(name);
                return Err(format!(
                    "kernel '{name}' exited during startup with {status}"
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }

        let _ = child.kill();
        let _ = child.wait();
        self.remove_artifacts(name);
        Err(format!("kernel '{name}' failed to start within 5 seconds"))
    }

    pub fn create_from_kernelspec(&self, name: &str, kernelspec_name: &str) -> Result<u32, String> {
        validate_name(name)?;
        fs::create_dir_all(&self.directory)
            .map_err(|error| format!("cannot create {}: {error}", self.directory.display()))?;
        let connection_path = self.connection_path(name);
        let pid_path = self.pid_path(name);
        if connection_path.exists() {
            if let Some(pid) = read_pid(&pid_path)? {
                if process_is_alive(pid) {
                    return Err(format!("kernel '{name}' is already running (pid {pid})"));
                }
            }
            self.remove_artifacts(name);
        }

        let spec = kernelspec::load(kernelspec_name)?;
        let prepared = kernelspec::prepare(&spec, &connection_path)?;
        let connection = kernelspec::write_connection_file(&connection_path)?;
        let mut child = match Command::new(&prepared.program)
            .args(&prepared.arguments)
            .envs(&prepared.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                self.remove_artifacts(name);
                return Err(format!(
                    "failed to launch kernelspec '{kernelspec_name}': {error}"
                ));
            }
        };
        let pid = child.id();
        if let Err(error) = fs::write(&pid_path, pid.to_string()) {
            let _ = child.kill();
            let _ = child.wait();
            self.remove_artifacts(name);
            return Err(format!("cannot write {}: {error}", pid_path.display()));
        }

        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
                self.remove_artifacts(name);
                return Err(format!(
                    "kernel '{name}' exited during startup with {status}"
                ));
            }
            if JupyterClient::from_value(&connection)
                .and_then(|client| client.heartbeat(Duration::from_millis(100)))
                .unwrap_or(false)
            {
                return Ok(pid);
            }
            thread::sleep(POLL_INTERVAL);
        }
        let _ = child.kill();
        let _ = child.wait();
        self.remove_artifacts(name);
        Err(format!("kernel '{name}' failed to start within 5 seconds"))
    }

    pub fn attach(&self, name: &str, connection_file: &Path) -> Result<(), String> {
        validate_name(name)?;
        fs::create_dir_all(&self.directory)
            .map_err(|error| format!("cannot create {}: {error}", self.directory.display()))?;
        let connection_path = self.connection_path(name);
        if connection_path.exists() {
            return Err(format!("kernel '{name}' already exists"));
        }
        let contents = fs::read_to_string(connection_file).map_err(|error| {
            format!(
                "cannot read connection file {}: {error}",
                connection_file.display()
            )
        })?;
        let connection: Value = serde_json::from_str(&contents)
            .map_err(|error| format!("invalid Jupyter connection file: {error}"))?;
        JupyterClient::from_value(&connection)?;
        fs::write(&connection_path, contents)
            .map_err(|error| format!("cannot write {}: {error}", connection_path.display()))
    }

    pub fn list(&self) -> Result<Vec<KernelStatus>, String> {
        if !self.directory.exists() {
            return Ok(Vec::new());
        }
        let mut kernels = Vec::new();
        for entry in fs::read_dir(&self.directory)
            .map_err(|error| format!("cannot read {}: {error}", self.directory.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            let pid = read_pid(&self.pid_path(name))?;
            let status = match pid {
                Some(pid) if process_is_alive(pid) => "running",
                Some(_) => "dead",
                None => "no-pid",
            }
            .to_owned();
            kernels.push(KernelStatus {
                name: name.to_owned(),
                pid,
                status,
            });
        }
        kernels.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(kernels)
    }

    pub fn connection(&self, name: &str) -> Result<Value, String> {
        validate_name(name)?;
        let path = self.connection_path(name);
        let contents =
            fs::read_to_string(&path).map_err(|_| format!("kernel '{name}' not found"))?;
        serde_json::from_str(&contents)
            .map_err(|error| format!("invalid connection file {}: {error}", path.display()))
    }

    pub fn execute(&self, name: &str, code: &str) -> Result<ReplResponse, String> {
        let connection = self.connection(name)?;
        if let Some(socket_path) = connection.get("socket_path").and_then(Value::as_str) {
            return execute_socket(Path::new(socket_path), code);
        }
        execute_jupyter(&connection, code)
    }

    pub fn complete(
        &self,
        name: &str,
        code: &str,
        cursor: Option<usize>,
    ) -> Result<JupyterMessage, String> {
        let mut client = JupyterClient::from_value(&self.connection(name)?)?;
        client.complete(code, cursor, EXECUTION_TIMEOUT)
    }

    pub fn inspect(
        &self,
        name: &str,
        code: &str,
        cursor: Option<usize>,
        detail_level: u8,
    ) -> Result<JupyterMessage, String> {
        let mut client = JupyterClient::from_value(&self.connection(name)?)?;
        client.inspect(code, cursor, detail_level, EXECUTION_TIMEOUT)
    }

    pub fn kernel_info(&self, name: &str) -> Result<JupyterMessage, String> {
        let mut client = JupyterClient::from_value(&self.connection(name)?)?;
        client.kernel_info(EXECUTION_TIMEOUT)
    }

    pub fn is_complete(&self, name: &str, code: &str) -> Result<JupyterMessage, String> {
        let mut client = JupyterClient::from_value(&self.connection(name)?)?;
        client.is_complete(code, EXECUTION_TIMEOUT)
    }

    pub fn interrupt(&self, name: &str) -> Result<JupyterMessage, String> {
        let mut client = JupyterClient::from_value(&self.connection(name)?)?;
        client.interrupt(EXECUTION_TIMEOUT)
    }

    pub fn heartbeat(&self, name: &str) -> Result<bool, String> {
        let client = JupyterClient::from_value(&self.connection(name)?)?;
        client.heartbeat(Duration::from_secs(2))
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        validate_name(name)?;
        if !self.connection_path(name).exists() {
            return Err(format!("kernel '{name}' not found"));
        }
        if let Some(pid) = read_pid(&self.pid_path(name))? {
            if process_is_alive(pid) {
                if let Ok(connection) = self.connection(name) {
                    let _ = jupyter::shutdown(&connection, SHUTDOWN_TIMEOUT);
                }
                if process_is_alive(pid) {
                    send_signal(pid, libc::SIGTERM)?;
                }
                let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
                while Instant::now() < deadline && process_is_alive(pid) {
                    thread::sleep(POLL_INTERVAL);
                }
                if process_is_alive(pid) {
                    send_signal(pid, libc::SIGKILL)?;
                }
            }
        }
        self.remove_artifacts(name);
        Ok(())
    }

    fn connection_path(&self, name: &str) -> PathBuf {
        self.directory.join(format!("{name}.json"))
    }

    fn pid_path(&self, name: &str) -> PathBuf {
        self.directory.join(format!("{name}.pid"))
    }

    fn socket_path(&self, name: &str) -> PathBuf {
        self.directory.join(format!("{name}.sock"))
    }

    fn remove_artifacts(&self, name: &str) {
        for path in [
            self.connection_path(name),
            self.pid_path(name),
            self.socket_path(name),
        ] {
            let _ = fs::remove_file(path);
        }
    }
}

fn default_kernel_script() -> PathBuf {
    let filename = "minimal_kernel_clean.py";
    if let Ok(current_directory) = env::current_dir() {
        let candidate = current_directory.join(filename);
        if candidate.exists() {
            return candidate;
        }
    }
    if let Ok(executable) = env::current_exe() {
        for ancestor in executable.ancestors().skip(1) {
            let candidate = ancestor.join(filename);
            if candidate.exists() {
                return candidate;
            }
        }
    }
    PathBuf::from(filename)
}

fn default_kernel_directory() -> PathBuf {
    env::var_os("REPLMUX_KERNEL_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME").map(|home| PathBuf::from(home).join(".jupyter-repl/kernels"))
        })
        .unwrap_or_else(|| PathBuf::from(".jupyter-repl/kernels"))
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'.' | b'_' | b'-'))
    {
        return Err("kernel name must contain only letters, numbers, '.', '_' or '-'".to_owned());
    }
    Ok(())
}

fn read_pid(path: &Path) -> Result<Option<u32>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let value = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    value
        .trim()
        .parse::<u32>()
        .map(Some)
        .map_err(|error| format!("invalid PID file {}: {error}", path.display()))
}

fn process_is_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn send_signal(pid: u32, signal: libc::c_int) -> Result<(), String> {
    let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
    if result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(format!(
            "failed to signal kernel process {pid}: {}",
            std::io::Error::last_os_error()
        ))
    }
}

fn execute_jupyter(connection: &Value, code: &str) -> Result<ReplResponse, String> {
    let mut client = JupyterClient::from_value(connection)?;
    let execution = client.execute(code, EXECUTION_TIMEOUT)?;
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut result = None;
    let mut error = None;
    for output in &execution.outputs {
        match output.message_type() {
            Some("stream") => {
                let text = output
                    .content
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                match output.content.get("name").and_then(Value::as_str) {
                    Some("stderr") => stderr.push_str(text),
                    _ => stdout.push_str(text),
                }
            }
            Some("execute_result") => {
                result = output
                    .content
                    .get("data")
                    .and_then(|data| data.get("text/plain"))
                    .cloned();
            }
            Some("error") => {
                let name = output
                    .content
                    .get("ename")
                    .and_then(Value::as_str)
                    .unwrap_or("Error");
                let value = output
                    .content
                    .get("evalue")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                error = Some(format!("{name}: {value}"));
            }
            _ => {}
        }
    }
    if error.is_none()
        && execution
            .reply
            .content
            .get("status")
            .and_then(Value::as_str)
            == Some("error")
    {
        let name = execution
            .reply
            .content
            .get("ename")
            .and_then(Value::as_str)
            .unwrap_or("Error");
        let value = execution
            .reply
            .content
            .get("evalue")
            .and_then(Value::as_str)
            .unwrap_or_default();
        error = Some(format!("{name}: {value}"));
    }
    Ok(ReplResponse {
        ok: error.is_none(),
        mode: Some(if result.is_some() { "eval" } else { "exec" }.to_owned()),
        code: Some(code.to_owned()),
        result,
        stdout,
        stderr,
        error,
        outputs: execution.outputs,
    })
}

fn execute_socket(socket_path: &Path, code: &str) -> Result<ReplResponse, String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| format!("cannot connect to {}: {error}", socket_path.display()))?;
    stream
        .set_read_timeout(Some(EXECUTION_TIMEOUT))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(EXECUTION_TIMEOUT))
        .map_err(|error| error.to_string())?;
    let request = serde_json::to_vec(&serde_json::json!({ "code": code }))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(&request)
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| error.to_string())?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|error| {
        if matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ) {
            "timed out waiting for REPL execution".to_owned()
        } else {
            error.to_string()
        }
    })?;
    serde_json::from_slice(&response)
        .map_err(|error| format!("invalid response from kernel: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_safe_kernel_names() {
        assert!(validate_name("analysis-1_test.py").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("../escape").is_err());
        assert!(validate_name("with space").is_err());
    }

    #[test]
    fn deserializes_success_and_error_responses() {
        let success: ReplResponse = serde_json::from_str(
            r#"{"ok":true,"mode":"eval","code":"40+2","result":"42","stdout":"","stderr":"","error":null}"#,
        )
        .unwrap();
        assert!(success.ok);
        assert_eq!(success.result, Some(Value::String("42".to_owned())));

        let failure: ReplResponse = serde_json::from_str(
            r#"{"ok":false,"mode":"exec","code":"1/0","result":null,"stdout":"","stderr":"","error":"ZeroDivisionError"}"#,
        )
        .unwrap();
        assert!(!failure.ok);
        assert_eq!(failure.error.as_deref(), Some("ZeroDivisionError"));
    }
}
