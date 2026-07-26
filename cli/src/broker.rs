use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::jupyter::JupyterMessage;
use crate::kernel::{KernelManager, KernelStatus, ReplResponse};

const IO_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_REQUEST_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    Auto,
    Local,
    Socket,
}

impl std::str::FromStr for TransportMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "local" => Ok(Self::Local),
            "socket" => Ok(Self::Socket),
            _ => Err("transport must be auto, local, or socket".to_owned()),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KernelRequest {
    pub operation: KernelOperation,
    pub kernel_dir: Option<PathBuf>,
    pub python: Option<PathBuf>,
    pub kernel_script: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum KernelOperation {
    Create {
        name: String,
        kernelspec: Option<String>,
    },
    List,
    Attach {
        name: String,
        connection_file: PathBuf,
    },
    Connect {
        name: String,
    },
    Delete {
        name: String,
    },
    Exec {
        name: String,
        code: String,
    },
    Complete {
        name: String,
        code: String,
        cursor: Option<usize>,
    },
    Inspect {
        name: String,
        code: String,
        cursor: Option<usize>,
        detail_level: u8,
    },
    KernelInfo {
        name: String,
    },
    IsComplete {
        name: String,
        code: String,
    },
    Interrupt {
        name: String,
    },
    Heartbeat {
        name: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KernelResponse {
    Created { name: String, pid: u32 },
    Listed { kernels: Vec<KernelStatus> },
    Attached { name: String },
    Connected { connection: Value },
    Deleted { name: String },
    Executed { response: ReplResponse },
    JupyterReply { message: JupyterMessage },
    Heartbeat { alive: bool },
}

#[derive(Debug, Deserialize, Serialize)]
struct WireResponse {
    ok: bool,
    response: Option<KernelResponse>,
    error: Option<String>,
}

pub fn dispatch(
    request: KernelRequest,
    mode: TransportMode,
    socket_path: &Path,
) -> Result<KernelResponse, String> {
    match mode {
        TransportMode::Local => handle_request(request),
        TransportMode::Socket => send_request(socket_path, &request).map_err(|error| match error {
            BrokerClientError::Unavailable => {
                format!("broker is unavailable at {}", socket_path.display())
            }
            BrokerClientError::Failure(error) => error,
        }),
        TransportMode::Auto => match send_request(socket_path, &request) {
            Ok(response) => Ok(response),
            Err(BrokerClientError::Unavailable) => handle_request(request),
            Err(BrokerClientError::Failure(error)) => Err(error),
        },
    }
}

pub fn serve(socket_path: &Path) -> Result<(), String> {
    prepare_socket(socket_path)?;
    let listener = UnixListener::bind(socket_path).map_err(|error| {
        format!(
            "cannot bind broker socket {}: {error}",
            socket_path.display()
        )
    })?;
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600)).map_err(|error| {
        format!(
            "cannot secure broker socket {}: {error}",
            socket_path.display()
        )
    })?;
    let _cleanup = SocketCleanup(socket_path.to_path_buf());
    println!("Replmux broker listening on {}", socket_path.display());

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                std::thread::spawn(move || {
                    if let Err(error) = handle_stream(&mut stream) {
                        let _ = write_wire_response(
                            &mut stream,
                            &WireResponse {
                                ok: false,
                                response: None,
                                error: Some(error),
                            },
                        );
                    }
                });
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(format!("broker accept failed: {error}")),
        }
    }
    Ok(())
}

pub fn default_socket_path() -> PathBuf {
    std::env::var_os("REPLMUX_BROKER_SOCKET")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".replmux/b.sock"))
        })
        .unwrap_or_else(|| PathBuf::from(".replmux/b.sock"))
}

fn handle_request(request: KernelRequest) -> Result<KernelResponse, String> {
    let manager =
        KernelManager::from_options(request.kernel_dir, request.python, request.kernel_script)?;
    match request.operation {
        KernelOperation::Create { name, kernelspec } => match kernelspec {
            Some(kernelspec) => manager.create_from_kernelspec(&name, &kernelspec),
            None => manager.create(&name),
        }
        .map(|pid| KernelResponse::Created { name, pid }),
        KernelOperation::List => manager
            .list()
            .map(|kernels| KernelResponse::Listed { kernels }),
        KernelOperation::Attach {
            name,
            connection_file,
        } => manager
            .attach(&name, &connection_file)
            .map(|_| KernelResponse::Attached { name }),
        KernelOperation::Connect { name } => manager
            .connection(&name)
            .map(|connection| KernelResponse::Connected { connection }),
        KernelOperation::Delete { name } => manager
            .delete(&name)
            .map(|_| KernelResponse::Deleted { name }),
        KernelOperation::Exec { name, code } => manager
            .execute(&name, &code)
            .map(|response| KernelResponse::Executed { response }),
        KernelOperation::Complete { name, code, cursor } => manager
            .complete(&name, &code, cursor)
            .map(|message| KernelResponse::JupyterReply { message }),
        KernelOperation::Inspect {
            name,
            code,
            cursor,
            detail_level,
        } => manager
            .inspect(&name, &code, cursor, detail_level)
            .map(|message| KernelResponse::JupyterReply { message }),
        KernelOperation::KernelInfo { name } => manager
            .kernel_info(&name)
            .map(|message| KernelResponse::JupyterReply { message }),
        KernelOperation::IsComplete { name, code } => manager
            .is_complete(&name, &code)
            .map(|message| KernelResponse::JupyterReply { message }),
        KernelOperation::Interrupt { name } => manager
            .interrupt(&name)
            .map(|message| KernelResponse::JupyterReply { message }),
        KernelOperation::Heartbeat { name } => manager
            .heartbeat(&name)
            .map(|alive| KernelResponse::Heartbeat { alive }),
    }
}

fn handle_stream(stream: &mut UnixStream) -> Result<(), String> {
    configure_stream(stream)?;
    let mut payload = Vec::new();
    (&mut *stream)
        .take(MAX_REQUEST_BYTES)
        .read_to_end(&mut payload)
        .map_err(|error| format!("cannot read broker request: {error}"))?;
    let request: KernelRequest = serde_json::from_slice(&payload)
        .map_err(|error| format!("invalid broker request: {error}"))?;
    let response = match handle_request(request) {
        Ok(response) => WireResponse {
            ok: true,
            response: Some(response),
            error: None,
        },
        Err(error) => WireResponse {
            ok: false,
            response: None,
            error: Some(error),
        },
    };
    write_wire_response(stream, &response)
}

fn write_wire_response(stream: &mut UnixStream, response: &WireResponse) -> Result<(), String> {
    let payload = serde_json::to_vec(response).map_err(|error| error.to_string())?;
    stream
        .write_all(&payload)
        .map_err(|error| format!("cannot write broker response: {error}"))
}

enum BrokerClientError {
    Unavailable,
    Failure(String),
}

fn send_request(
    socket_path: &Path,
    request: &KernelRequest,
) -> Result<KernelResponse, BrokerClientError> {
    let mut stream = UnixStream::connect(socket_path).map_err(|error| match error.kind() {
        ErrorKind::NotFound | ErrorKind::ConnectionRefused => BrokerClientError::Unavailable,
        _ => BrokerClientError::Failure(format!(
            "cannot connect to broker socket {}: {error}",
            socket_path.display()
        )),
    })?;
    configure_stream(&stream).map_err(BrokerClientError::Failure)?;
    let payload = serde_json::to_vec(request)
        .map_err(|error| BrokerClientError::Failure(error.to_string()))?;
    stream
        .write_all(&payload)
        .map_err(|error| BrokerClientError::Failure(error.to_string()))?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| BrokerClientError::Failure(error.to_string()))?;
    let mut payload = Vec::new();
    (&mut stream)
        .take(MAX_REQUEST_BYTES)
        .read_to_end(&mut payload)
        .map_err(|error| BrokerClientError::Failure(error.to_string()))?;
    let response: WireResponse = serde_json::from_slice(&payload)
        .map_err(|error| BrokerClientError::Failure(format!("invalid broker response: {error}")))?;
    if response.ok {
        response.response.ok_or_else(|| {
            BrokerClientError::Failure("broker returned no response payload".to_owned())
        })
    } else {
        Err(BrokerClientError::Failure(
            response
                .error
                .unwrap_or_else(|| "broker request failed".to_owned()),
        ))
    }
}

fn configure_stream(stream: &UnixStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|error| error.to_string())
}

fn prepare_socket(socket_path: &Path) -> Result<(), String> {
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    if socket_path.exists() {
        match UnixStream::connect(socket_path) {
            Ok(_) => {
                return Err(format!(
                    "broker is already running at {}",
                    socket_path.display()
                ));
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::ConnectionRefused | ErrorKind::NotFound
                ) =>
            {
                fs::remove_file(socket_path).map_err(|remove_error| {
                    format!(
                        "cannot remove stale broker socket {}: {remove_error}",
                        socket_path.display()
                    )
                })?;
            }
            Err(error) => {
                return Err(format!(
                    "cannot inspect broker socket {}: {error}",
                    socket_path.display()
                ));
            }
        }
    }
    Ok(())
}

struct SocketCleanup(PathBuf);

impl Drop for SocketCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_transport_modes() {
        assert_eq!("auto".parse(), Ok(TransportMode::Auto));
        assert_eq!("local".parse(), Ok(TransportMode::Local));
        assert_eq!("socket".parse(), Ok(TransportMode::Socket));
        assert!("remote".parse::<TransportMode>().is_err());
    }

    #[test]
    fn auto_mode_short_circuits_missing_socket_to_local_service() {
        let request = KernelRequest {
            operation: KernelOperation::List,
            kernel_dir: Some(PathBuf::from("/tmp/replmux-missing-local-test")),
            python: None,
            kernel_script: None,
        };
        let response = dispatch(
            request,
            TransportMode::Auto,
            Path::new("/tmp/replmux-definitely-missing/b.sock"),
        )
        .unwrap();
        assert!(matches!(response, KernelResponse::Listed { kernels } if kernels.is_empty()));
    }
}
