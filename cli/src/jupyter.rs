use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::Sha256;

const DELIMITER: &[u8] = b"<IDS|MSG>";

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Deserialize)]
pub struct JupyterConnection {
    pub control_port: u16,
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_transport")]
    pub transport: String,
    #[serde(default)]
    pub key: String,
    #[serde(default = "default_signature_scheme")]
    pub signature_scheme: String,
}

pub fn shutdown(connection: &Value, timeout: Duration) -> Result<(), String> {
    let connection: JupyterConnection = serde_json::from_value(connection.clone())
        .map_err(|error| format!("invalid Jupyter connection: {error}"))?;
    if connection.signature_scheme != "hmac-sha256" {
        return Err(format!(
            "unsupported Jupyter signature scheme: {}",
            connection.signature_scheme
        ));
    }
    if connection.transport != "tcp" {
        return Err(format!(
            "unsupported Jupyter transport: {}",
            connection.transport
        ));
    }

    let context = zmq::Context::new();
    let socket = context.socket(zmq::DEALER).map_err(zmq_error)?;
    socket.set_linger(0).map_err(zmq_error)?;
    let timeout_millis = i32::try_from(timeout.as_millis()).unwrap_or(i32::MAX);
    socket.set_rcvtimeo(timeout_millis).map_err(zmq_error)?;
    socket.set_sndtimeo(timeout_millis).map_err(zmq_error)?;
    socket
        .connect(&format!(
            "tcp://{}:{}",
            connection.ip, connection.control_port
        ))
        .map_err(zmq_error)?;

    let message_id = message_id();
    let header = json!({
        "msg_id": message_id,
        "msg_type": "shutdown_request",
        "username": "multirepl",
        "session": message_id,
        "date": "1970-01-01T00:00:00Z",
        "version": "5.4"
    });
    let key = decode_key(&connection.key)?;
    let frames = serialize_message(
        &header,
        &json!({}),
        &json!({}),
        &json!({"restart": false}),
        &key,
    )?;
    socket.send_multipart(frames, 0).map_err(zmq_error)?;
    let reply = socket.recv_multipart(0).map_err(zmq_error)?;
    validate_reply(&reply, &message_id, &key)
}

pub fn bundled_zmq_version() -> (i32, i32, i32) {
    zmq::version()
}

fn serialize_message(
    header: &Value,
    parent: &Value,
    metadata: &Value,
    content: &Value,
    key: &[u8],
) -> Result<Vec<Vec<u8>>, String> {
    let parts = [header, parent, metadata, content]
        .map(|value| serde_json::to_vec(value).map_err(|error| error.to_string()))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let signature = if key.is_empty() {
        Vec::new()
    } else {
        let mut mac = HmacSha256::new_from_slice(key).map_err(|error| error.to_string())?;
        for part in &parts {
            mac.update(part);
        }
        mac.finalize()
            .into_bytes()
            .iter()
            .flat_map(|byte| format!("{byte:02x}").into_bytes())
            .collect()
    };
    let mut frames = vec![DELIMITER.to_vec(), signature];
    frames.extend(parts);
    Ok(frames)
}

fn validate_reply(frames: &[Vec<u8>], parent_message_id: &str, key: &[u8]) -> Result<(), String> {
    let delimiter = frames
        .iter()
        .position(|frame| frame == DELIMITER)
        .ok_or_else(|| "Jupyter reply is missing delimiter".to_owned())?;
    let body = &frames[delimiter + 1..];
    if body.len() < 5 {
        return Err("Jupyter reply is malformed".to_owned());
    }
    if !key.is_empty() {
        let supplied_signature = std::str::from_utf8(&body[0])
            .map_err(|error| format!("invalid Jupyter signature encoding: {error}"))?;
        let supplied_signature = decode_hex(supplied_signature)?;
        let mut mac = HmacSha256::new_from_slice(key).map_err(|error| error.to_string())?;
        for part in &body[1..5] {
            mac.update(part);
        }
        mac.verify_slice(&supplied_signature)
            .map_err(|_| "invalid Jupyter reply signature".to_owned())?;
    }
    let parent: Value = serde_json::from_slice(&body[2])
        .map_err(|error| format!("invalid Jupyter parent header: {error}"))?;
    if parent.get("msg_id").and_then(Value::as_str) != Some(parent_message_id) {
        return Err("Jupyter shutdown reply has the wrong parent message".to_owned());
    }
    let content: Value = serde_json::from_slice(&body[4])
        .map_err(|error| format!("invalid Jupyter reply content: {error}"))?;
    if content
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status != "ok")
    {
        return Err(format!("Jupyter shutdown failed: {content}"));
    }
    Ok(())
}

fn decode_key(key: &str) -> Result<Vec<u8>, String> {
    if key.is_empty() {
        return Ok(Vec::new());
    }
    if key.len().is_multiple_of(2) && key.bytes().all(|value| value.is_ascii_hexdigit()) {
        return decode_hex(key);
    }
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

fn message_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("multirepl-{}-{nanos}", std::process::id())
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
    fn decodes_hex_and_text_keys() {
        assert_eq!(decode_key("616263").unwrap(), b"abc");
        assert_eq!(decode_key("not-hex").unwrap(), b"not-hex");
    }
}
