use std::collections::BTreeMap;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;

const DELIMITER: &[u8] = b"<IDS|MSG>";
const PROTOCOL_VERSION: &str = "5.4";

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JupyterConnection {
    pub shell_port: u16,
    pub iopub_port: u16,
    pub control_port: u16,
    pub hb_port: u16,
    #[serde(default)]
    pub stdin_port: u16,
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default)]
    pub key: String,
    #[serde(default = "default_signature_scheme")]
    pub signature_scheme: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JupyterMessage {
    pub header: Value,
    pub parent_header: Value,
    pub metadata: Value,
    pub content: Value,
    #[serde(default)]
    pub buffers: Vec<Vec<u8>>,
}

impl JupyterMessage {
    pub fn message_type(&self) -> Option<&str> {
        self.header.get("msg_type").and_then(Value::as_str)
    }

    pub fn parent_message_id(&self) -> Option<&str> {
        self.parent_header.get("msg_id").and_then(Value::as_str)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExecutionResult {
    pub reply: JupyterMessage,
    pub outputs: Vec<JupyterMessage>,
}

pub struct JupyterClient {
    _context: zmq::Context,
    shell: zmq::Socket,
    iopub: zmq::Socket,
    stdin: Option<zmq::Socket>,
    control: zmq::Socket,
    heartbeat: zmq::Socket,
    key: Vec<u8>,
    session_id: String,
    message_count: u64,
}

impl JupyterClient {
    pub fn connect(connection: &JupyterConnection) -> Result<Self, String> {
        validate_connection(connection)?;
        let context = zmq::Context::new();
        let shell = context.socket(zmq::DEALER).map_err(zmq_error)?;
        let iopub = context.socket(zmq::SUB).map_err(zmq_error)?;
        let control = context.socket(zmq::DEALER).map_err(zmq_error)?;
        let heartbeat = context.socket(zmq::REQ).map_err(zmq_error)?;
        let stdin = if connection.stdin_port == 0 {
            None
        } else {
            Some(context.socket(zmq::DEALER).map_err(zmq_error)?)
        };

        for socket in [&shell, &iopub, &control, &heartbeat] {
            socket.set_linger(0).map_err(zmq_error)?;
        }
        if let Some(socket) = &stdin {
            socket.set_linger(0).map_err(zmq_error)?;
        }
        iopub.set_subscribe(b"").map_err(zmq_error)?;

        shell
            .connect(&channel_url(connection, "shell", connection.shell_port))
            .map_err(zmq_error)?;
        iopub
            .connect(&channel_url(connection, "iopub", connection.iopub_port))
            .map_err(zmq_error)?;
        control
            .connect(&channel_url(connection, "control", connection.control_port))
            .map_err(zmq_error)?;
        heartbeat
            .connect(&channel_url(connection, "hb", connection.hb_port))
            .map_err(zmq_error)?;
        if let Some(socket) = &stdin {
            socket
                .connect(&channel_url(connection, "stdin", connection.stdin_port))
                .map_err(zmq_error)?;
        }

        Ok(Self {
            _context: context,
            shell,
            iopub,
            stdin,
            control,
            heartbeat,
            key: decode_key(&connection.key)?,
            session_id: unique_id("session"),
            message_count: 0,
        })
    }

    pub fn from_value(connection: &Value) -> Result<Self, String> {
        let connection: JupyterConnection = serde_json::from_value(connection.clone())
            .map_err(|error| format!("invalid Jupyter connection: {error}"))?;
        Self::connect(&connection)
    }

    pub fn execute(&mut self, code: &str, timeout: Duration) -> Result<ExecutionResult, String> {
        self.execute_with_options(code, ExecuteOptions::default(), timeout)
    }

    pub fn execute_with_options(
        &mut self,
        code: &str,
        options: ExecuteOptions,
        timeout: Duration,
    ) -> Result<ExecutionResult, String> {
        if self.message_count == 0 {
            self.kernel_info(timeout.min(Duration::from_secs(5)))?;
            thread::sleep(Duration::from_millis(10));
        }
        let content = json!({
            "code": code,
            "silent": options.silent,
            "store_history": options.store_history,
            "user_expressions": options.user_expressions,
            "allow_stdin": options.allow_stdin,
            "stop_on_error": options.stop_on_error,
        });
        let message_id = self.send_shell("execute_request", content)?;
        let deadline = Instant::now() + timeout;
        let mut reply = None;
        let mut outputs = Vec::new();
        let mut idle = false;

        while reply.is_none() || !idle {
            let timeout_ms = remaining_millis(deadline, "execution")?;
            let mut poll_items = [
                self.shell.as_poll_item(zmq::POLLIN),
                self.iopub.as_poll_item(zmq::POLLIN),
            ];
            zmq::poll(&mut poll_items, timeout_ms).map_err(zmq_error)?;
            if !poll_items.iter().any(zmq::PollItem::is_readable) {
                return Err("timed out waiting for execution".to_owned());
            }
            if poll_items[1].is_readable() {
                while self
                    .iopub
                    .get_events()
                    .map_err(zmq_error)?
                    .contains(zmq::POLLIN)
                {
                    let message = recv_message(&self.iopub, &self.key, true)?;
                    if message.parent_message_id() != Some(message_id.as_str()) {
                        continue;
                    }
                    idle |= message.message_type() == Some("status")
                        && message
                            .content
                            .get("execution_state")
                            .and_then(Value::as_str)
                            == Some("idle");
                    outputs.push(message);
                }
            }
            if poll_items[0].is_readable() {
                let message = recv_message(&self.shell, &self.key, false)?;
                if message.parent_message_id() == Some(message_id.as_str()) {
                    reply = Some(message);
                }
            }
        }

        Ok(ExecutionResult {
            reply: reply.expect("execution loop requires a reply"),
            outputs,
        })
    }

    pub fn complete(
        &mut self,
        code: &str,
        cursor_pos: Option<usize>,
        timeout: Duration,
    ) -> Result<JupyterMessage, String> {
        self.shell_request(
            "complete_request",
            json!({"code": code, "cursor_pos": cursor_pos.unwrap_or(code.len())}),
            timeout,
        )
    }

    pub fn inspect(
        &mut self,
        code: &str,
        cursor_pos: Option<usize>,
        detail_level: u8,
        timeout: Duration,
    ) -> Result<JupyterMessage, String> {
        self.shell_request(
            "inspect_request",
            json!({
                "code": code,
                "cursor_pos": cursor_pos.unwrap_or(code.len()),
                "detail_level": detail_level,
            }),
            timeout,
        )
    }

    pub fn kernel_info(&mut self, timeout: Duration) -> Result<JupyterMessage, String> {
        self.shell_request("kernel_info_request", json!({}), timeout)
    }

    pub fn is_complete(&mut self, code: &str, timeout: Duration) -> Result<JupyterMessage, String> {
        self.shell_request("is_complete_request", json!({"code": code}), timeout)
    }

    pub fn interrupt(&mut self, timeout: Duration) -> Result<JupyterMessage, String> {
        self.control_request("interrupt_request", json!({}), timeout)
    }

    pub fn input(&mut self, value: &str) -> Result<(), String> {
        let message = self.build_message("input_reply", json!({"value": value}));
        let socket = self
            .stdin
            .as_ref()
            .ok_or_else(|| "kernel connection has no stdin channel".to_owned())?;
        send_message(socket, &message, &self.key)
    }

    pub fn heartbeat(&self, timeout: Duration) -> Result<bool, String> {
        let timeout_ms = duration_millis(timeout);
        self.heartbeat.set_sndtimeo(timeout_ms).map_err(zmq_error)?;
        self.heartbeat.set_rcvtimeo(timeout_ms).map_err(zmq_error)?;
        self.heartbeat.send("ping", 0).map_err(zmq_error)?;
        match self.heartbeat.recv_bytes(0) {
            Ok(reply) => Ok(reply == b"ping"),
            Err(zmq::Error::EAGAIN) => Ok(false),
            Err(error) => Err(zmq_error(error)),
        }
    }

    pub fn shutdown(&mut self, restart: bool, timeout: Duration) -> Result<JupyterMessage, String> {
        self.control_request("shutdown_request", json!({"restart": restart}), timeout)
    }

    fn shell_request(
        &mut self,
        message_type: &str,
        content: Value,
        timeout: Duration,
    ) -> Result<JupyterMessage, String> {
        let message_id = self.send_shell(message_type, content)?;
        recv_matching(&self.shell, &self.key, &message_id, timeout, message_type)
    }

    fn control_request(
        &mut self,
        message_type: &str,
        content: Value,
        timeout: Duration,
    ) -> Result<JupyterMessage, String> {
        let message = self.build_message(message_type, content);
        let message_id = message
            .header
            .get("msg_id")
            .and_then(Value::as_str)
            .expect("generated headers contain msg_id")
            .to_owned();
        send_message(&self.control, &message, &self.key)?;
        recv_matching(&self.control, &self.key, &message_id, timeout, message_type)
    }

    fn send_shell(&mut self, message_type: &str, content: Value) -> Result<String, String> {
        let message = self.build_message(message_type, content);
        let message_id = message
            .header
            .get("msg_id")
            .and_then(Value::as_str)
            .expect("generated headers contain msg_id")
            .to_owned();
        send_message(&self.shell, &message, &self.key)?;
        Ok(message_id)
    }

    fn build_message(&mut self, message_type: &str, content: Value) -> JupyterMessage {
        let message_id = format!(
            "{}-{}-{}",
            self.session_id,
            std::process::id(),
            self.message_count
        );
        self.message_count += 1;
        JupyterMessage {
            header: json!({
                "msg_id": message_id,
                "msg_type": message_type,
                "username": "replmux",
                "session": self.session_id.clone(),
                "date": utc_timestamp(),
                "version": PROTOCOL_VERSION,
            }),
            parent_header: json!({}),
            metadata: json!({}),
            content,
            buffers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecuteOptions {
    pub silent: bool,
    pub store_history: bool,
    pub user_expressions: BTreeMap<String, String>,
    pub allow_stdin: bool,
    pub stop_on_error: bool,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            silent: false,
            store_history: true,
            user_expressions: BTreeMap::new(),
            allow_stdin: true,
            stop_on_error: true,
        }
    }
}

pub fn shutdown(connection: &Value, timeout: Duration) -> Result<(), String> {
    let mut client = JupyterClient::from_value(connection)?;
    client.shutdown(false, timeout).map(|_| ())
}

pub fn bundled_zmq_version() -> (i32, i32, i32) {
    zmq::version()
}

fn send_message(socket: &zmq::Socket, message: &JupyterMessage, key: &[u8]) -> Result<(), String> {
    let mut parts = vec![
        serde_json::to_vec(&message.header).map_err(json_error)?,
        serde_json::to_vec(&message.parent_header).map_err(json_error)?,
        serde_json::to_vec(&message.metadata).map_err(json_error)?,
        serde_json::to_vec(&message.content).map_err(json_error)?,
    ];
    let signature = sign(&parts, key)?;
    let mut frames = vec![DELIMITER.to_vec(), signature];
    frames.append(&mut parts);
    frames.extend(message.buffers.iter().cloned());
    socket.send_multipart(frames, 0).map_err(zmq_error)
}

fn recv_message(
    socket: &zmq::Socket,
    key: &[u8],
    allow_unsigned: bool,
) -> Result<JupyterMessage, String> {
    let frames = socket.recv_multipart(0).map_err(zmq_error)?;
    deserialize_message(&frames, key, allow_unsigned)
}

fn deserialize_message(
    frames: &[Vec<u8>],
    key: &[u8],
    allow_unsigned: bool,
) -> Result<JupyterMessage, String> {
    let delimiter = frames
        .iter()
        .position(|frame| frame == DELIMITER)
        .ok_or_else(|| "Jupyter message is missing delimiter".to_owned())?;
    let body = &frames[delimiter + 1..];
    if body.len() < 5 {
        return Err("Jupyter message is malformed".to_owned());
    }
    if !key.is_empty() {
        if body[0].is_empty() && allow_unsigned {
            // Some third-party IOPub implementations omit signatures.
        } else {
            let supplied = std::str::from_utf8(&body[0])
                .map_err(|error| format!("invalid Jupyter signature encoding: {error}"))?;
            let supplied = decode_hex(supplied)?;
            let mut mac = HmacSha256::new_from_slice(key).map_err(|error| error.to_string())?;
            for part in &body[1..5] {
                mac.update(part);
            }
            mac.verify_slice(&supplied)
                .map_err(|_| "invalid Jupyter message signature".to_owned())?;
        }
    }
    Ok(JupyterMessage {
        header: serde_json::from_slice(&body[1]).map_err(json_error)?,
        parent_header: serde_json::from_slice(&body[2]).map_err(json_error)?,
        metadata: serde_json::from_slice(&body[3]).map_err(json_error)?,
        content: serde_json::from_slice(&body[4]).map_err(json_error)?,
        buffers: body[5..].to_vec(),
    })
}

fn recv_matching(
    socket: &zmq::Socket,
    key: &[u8],
    message_id: &str,
    timeout: Duration,
    operation: &str,
) -> Result<JupyterMessage, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let timeout_ms = remaining_millis(deadline, operation)?;
        if socket.poll(zmq::POLLIN, timeout_ms).map_err(zmq_error)? == 0 {
            return Err(format!("timed out waiting for {operation}"));
        }
        let message = recv_message(socket, key, false)?;
        if message.parent_message_id() == Some(message_id) {
            return Ok(message);
        }
    }
}

fn sign(parts: &[Vec<u8>], key: &[u8]) -> Result<Vec<u8>, String> {
    if key.is_empty() {
        return Ok(Vec::new());
    }
    let mut mac = HmacSha256::new_from_slice(key).map_err(|error| error.to_string())?;
    for part in parts {
        mac.update(part);
    }
    Ok(to_hex(&mac.finalize().into_bytes()).into_bytes())
}

fn validate_connection(connection: &JupyterConnection) -> Result<(), String> {
    if connection.signature_scheme != "hmac-sha256" {
        return Err(format!(
            "unsupported Jupyter signature scheme: {}",
            connection.signature_scheme
        ));
    }
    if connection.transport.is_empty() {
        return Err("Jupyter transport cannot be empty".to_owned());
    }
    Ok(())
}

fn channel_url(connection: &JupyterConnection, _channel: &str, port: u16) -> String {
    if connection.transport == "tcp" {
        format!("tcp://{}:{port}", connection.ip)
    } else {
        format!("{}://{}-{port}", connection.transport, connection.ip)
    }
}

fn remaining_millis(deadline: Instant, operation: &str) -> Result<i64, String> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| format!("timed out waiting for {operation}"))?;
    Ok(i64::try_from(remaining.as_millis()).unwrap_or(i64::MAX))
}

fn duration_millis(duration: Duration) -> i32 {
    i32::try_from(duration.as_millis()).unwrap_or(i32::MAX)
}

fn decode_key(key: &str) -> Result<Vec<u8>, String> {
    Ok(key.as_bytes().to_vec())
}

fn decode_hex(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("hex value has an odd length".to_owned());
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).map_err(|error| error.to_string())?;
            u8::from_str_radix(pair, 16).map_err(|error| error.to_string())
        })
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unique_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{}-{nanos}", std::process::id())
}

fn utc_timestamp() -> String {
    let mut now: libc::time_t = 0;
    let mut utc = std::mem::MaybeUninit::<libc::tm>::uninit();
    let result = unsafe {
        libc::time(&mut now);
        libc::gmtime_r(&now, utc.as_mut_ptr())
    };
    if result.is_null() {
        return "1970-01-01T00:00:00Z".to_owned();
    }
    let utc = unsafe { utc.assume_init() };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        utc.tm_year + 1900,
        utc.tm_mon + 1,
        utc.tm_mday,
        utc.tm_hour,
        utc.tm_min,
        utc.tm_sec
    )
}

fn json_error(error: serde_json::Error) -> String {
    format!("invalid Jupyter JSON: {error}")
}

fn zmq_error(error: zmq::Error) -> String {
    format!("ZeroMQ error: {error}")
}

fn default_ip() -> String {
    "127.0.0.1".to_owned()
}

fn default_transport() -> String {
    "tcp".to_owned()
}

fn default_signature_scheme() -> String {
    "hmac-sha256".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_libzmq_is_available() {
        assert!(bundled_zmq_version() >= (4, 3, 0));
    }

    #[test]
    fn preserves_connection_key_bytes() {
        assert_eq!(decode_key("616263").unwrap(), b"616263");
        assert_eq!(decode_key("not-hex").unwrap(), b"not-hex");
        assert_eq!(decode_key("").unwrap(), b"");
    }

    #[test]
    fn signed_messages_round_trip() {
        let key = b"secret";
        let message = JupyterMessage {
            header: json!({"msg_id": "1", "msg_type": "kernel_info_request"}),
            parent_header: json!({}),
            metadata: json!({}),
            content: json!({}),
            buffers: vec![b"buffer".to_vec()],
        };
        let parts = [
            serde_json::to_vec(&message.header).unwrap(),
            serde_json::to_vec(&message.parent_header).unwrap(),
            serde_json::to_vec(&message.metadata).unwrap(),
            serde_json::to_vec(&message.content).unwrap(),
        ];
        let mut frames = vec![DELIMITER.to_vec(), sign(&parts, key).unwrap()];
        frames.extend(parts);
        frames.extend(message.buffers.clone());
        let decoded = deserialize_message(&frames, key, false).unwrap();
        assert_eq!(decoded.content, message.content);
        assert_eq!(decoded.buffers, message.buffers);
    }

    #[test]
    fn rejects_tampered_signatures() {
        let frames = vec![
            DELIMITER.to_vec(),
            vec![b'0'; 64],
            br#"{"msg_id":"1"}"#.to_vec(),
            b"{}".to_vec(),
            b"{}".to_vec(),
            b"{}".to_vec(),
        ];
        assert!(deserialize_message(&frames, b"secret", false).is_err());
    }
}
