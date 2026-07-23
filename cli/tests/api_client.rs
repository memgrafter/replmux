use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use multirepl_runtime_cli::{ApiClient, ApiError, RuntimeCreate};

const RUNTIME_JSON: &str = r#"{
  "id":"rt_123",
  "name":"analysis",
  "language":"python",
  "environment":{"kind":"python","executable":"python3","digest":null},
  "snapshot_policy":{"interval_executions":25,"mode":"logical"},
  "status":"idle",
  "worker_generation":0,
  "revision":1,
  "created_at":"2026-07-23T00:00:00+00:00",
  "updated_at":"2026-07-23T00:00:00+00:00"
}"#;

#[test]
fn get_runtime_uses_object_endpoint() {
    let (base_url, request) = serve_once(200, RUNTIME_JSON);
    let client = ApiClient::new(&base_url).unwrap();

    let runtime = client.get_runtime("rt_123").unwrap();

    assert_eq!(runtime.id, "rt_123");
    assert_eq!(runtime.name, "analysis");
    assert!(
        request
            .recv()
            .unwrap()
            .starts_with("GET /v1/runtimes/rt_123 ")
    );
}

#[test]
fn create_runtime_sends_openapi_shape() {
    let (base_url, request) = serve_once(201, RUNTIME_JSON);
    let client = ApiClient::new(&base_url).unwrap();

    client
        .create_runtime(&RuntimeCreate::new("analysis"))
        .unwrap();

    let request = request.recv().unwrap();
    assert!(request.starts_with("POST /v1/runtimes "));
    assert!(request.contains("\"name\":\"analysis\""));
    assert!(request.contains("\"snapshot_policy\""));
}

#[test]
fn api_error_includes_fastapi_detail() {
    let (base_url, _) = serve_once(404, r#"{"detail":"Runtime not found: rt_missing"}"#);
    let client = ApiClient::new(&base_url).unwrap();

    let error = client.get_runtime("rt_missing").unwrap_err();

    match error {
        ApiError::Response { status, detail } => {
            assert_eq!(status.as_u16(), 404);
            assert_eq!(detail, "Runtime not found: rt_missing");
        }
        other => panic!("unexpected error: {other}"),
    }
}

fn serve_once(status: u16, response_body: &'static str) -> (String, Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sender, receiver) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream);
        let _ = sender.send(request);
        let reason = match status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            404 => "Not Found",
            _ => "Response",
        };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        )
        .unwrap();
    });

    (format!("http://{address}"), receiver)
}

fn read_request(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        if request_complete(&request) {
            break;
        }
    }
    String::from_utf8(request).unwrap()
}

fn request_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    request.len() >= header_end + 4 + content_length
}
