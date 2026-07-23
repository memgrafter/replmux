use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KernelSpec {
    pub argv: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub language: String,
}

#[derive(Debug, Clone)]
pub struct PreparedKernel {
    pub program: String,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
}

pub fn load(name_or_path: &str) -> Result<KernelSpec, String> {
    let supplied = PathBuf::from(name_or_path);
    if supplied.exists() {
        let path = if supplied.is_dir() {
            supplied.join("kernel.json")
        } else {
            supplied
        };
        return read_spec(&path);
    }
    for directory in search_directories() {
        let path = directory.join(name_or_path).join("kernel.json");
        if path.exists() {
            return read_spec(&path);
        }
    }
    Err(format!("Jupyter kernelspec not found: {name_or_path}"))
}

pub fn prepare(spec: &KernelSpec, connection_file: &Path) -> Result<PreparedKernel, String> {
    let mut argv = spec
        .argv
        .iter()
        .map(|value| expand_environment(value))
        .collect::<Vec<_>>();
    if argv.is_empty() {
        return Err("kernelspec argv cannot be empty".to_owned());
    }
    let connection_file = connection_file
        .to_str()
        .ok_or_else(|| "connection file path is not valid UTF-8".to_owned())?;
    for argument in &mut argv {
        *argument = argument.replace("{connection_file}", connection_file);
    }
    if !argv
        .iter()
        .any(|argument| argument.contains(connection_file))
    {
        return Err("kernelspec argv must contain {connection_file}".to_owned());
    }
    let program = argv.remove(0);
    let environment = spec
        .env
        .iter()
        .map(|(key, value)| (key.clone(), expand_environment(value)))
        .collect();
    Ok(PreparedKernel {
        program,
        arguments: argv,
        environment,
    })
}

pub fn write_connection_file(path: &Path) -> Result<serde_json::Value, String> {
    let ports = reserve_ports(5)?;
    let key = random_key()?;
    let connection = json!({
        "shell_port": ports[0],
        "iopub_port": ports[1],
        "stdin_port": ports[2],
        "control_port": ports[3],
        "hb_port": ports[4],
        "ip": "127.0.0.1",
        "key": key,
        "transport": "tcp",
        "signature_scheme": "hmac-sha256",
        "kernel_name": "",
    });
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let payload = serde_json::to_vec_pretty(&connection).map_err(|error| error.to_string())?;
    fs::write(path, payload)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(connection)
}

pub fn search_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(paths) = env::var_os("JUPYTER_PATH") {
        directories.extend(env::split_paths(&paths).map(|path| path.join("kernels")));
    }
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        directories.push(PathBuf::from(data_home).join("jupyter/kernels"));
    }
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        directories.push(home.join("Library/Jupyter/kernels"));
        directories.push(home.join(".local/share/jupyter/kernels"));
    }
    directories.push(PathBuf::from("/Library/Jupyter/kernels"));
    directories.push(PathBuf::from("/opt/homebrew/share/jupyter/kernels"));
    directories.push(PathBuf::from("/usr/local/share/jupyter/kernels"));
    directories.push(PathBuf::from("/usr/share/jupyter/kernels"));
    directories
}

fn read_spec(path: &Path) -> Result<KernelSpec, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("cannot read kernelspec {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("invalid kernelspec {}: {error}", path.display()))
}

fn reserve_ports(count: usize) -> Result<Vec<u16>, String> {
    let listeners = (0..count)
        .map(|_| TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    listeners
        .iter()
        .map(|listener| {
            listener
                .local_addr()
                .map(|address| address.port())
                .map_err(|error| error.to_string())
        })
        .collect()
}

fn random_key() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    fs::File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|error| format!("cannot generate Jupyter signing key: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn expand_environment(value: &str) -> String {
    let mut expanded = value.to_owned();
    for (key, value) in env::vars() {
        expanded = expanded.replace(&format!("${{{key}}}"), &value);
    }
    expanded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepares_kernelspec_connection_argument() {
        let spec = KernelSpec {
            argv: vec![
                "python3".to_owned(),
                "-m".to_owned(),
                "ipykernel_launcher".to_owned(),
                "-f".to_owned(),
                "{connection_file}".to_owned(),
            ],
            env: BTreeMap::new(),
            display_name: "Python".to_owned(),
            language: "python".to_owned(),
        };
        let prepared = prepare(&spec, Path::new("/tmp/kernel.json")).unwrap();
        assert_eq!(prepared.program, "python3");
        assert_eq!(prepared.arguments.last().unwrap(), "/tmp/kernel.json");
    }

    #[test]
    fn writes_standard_connection_document() {
        let path = PathBuf::from(format!(
            "/tmp/replmux-connection-test-{}.json",
            std::process::id()
        ));
        let connection = write_connection_file(&path).unwrap();
        assert!(connection["shell_port"].as_u64().unwrap() > 0);
        assert_eq!(connection["key"].as_str().unwrap().len(), 64);
        let _ = fs::remove_file(path);
    }
}
