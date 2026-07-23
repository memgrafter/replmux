use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::broker::{KernelOperation, KernelRequest, KernelResponse, TransportMode, dispatch};

const SERVER_NAME: &str = "replmux";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_PROTOCOL_VERSION: &str = "2024-11-05";

pub struct McpServer {
    kernel_dir: Option<PathBuf>,
    python: Option<PathBuf>,
    kernel_script: Option<PathBuf>,
    transport: TransportMode,
    broker_socket: PathBuf,
}

impl McpServer {
    pub fn new(
        kernel_dir: Option<PathBuf>,
        python: Option<PathBuf>,
        kernel_script: Option<PathBuf>,
        transport: TransportMode,
        broker_socket: PathBuf,
    ) -> Self {
        Self {
            kernel_dir,
            python,
            kernel_script,
            transport,
            broker_socket,
        }
    }

    pub fn serve(self) -> Result<(), String> {
        let stdin = io::stdin();
        let mut stdout = io::stdout().lock();
        for line in stdin.lock().lines() {
            let line = line.map_err(|error| format!("cannot read MCP request: {error}"))?;
            if line.trim().is_empty() {
                continue;
            }
            let request: Value = serde_json::from_str(&line)
                .map_err(|error| format!("invalid MCP JSON: {error}"))?;
            if let Some(response) = self.handle(&request) {
                serde_json::to_writer(&mut stdout, &response).map_err(|error| error.to_string())?;
                stdout.write_all(b"\n").map_err(|error| error.to_string())?;
                stdout.flush().map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }

    fn handle(&self, request: &Value) -> Option<Value> {
        let id = request.get("id")?.clone();
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let result = match method {
            "initialize" => Ok(initialize_result(request)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tool_definitions()})),
            "tools/call" => self.call_tool(request.get("params").unwrap_or(&Value::Null)),
            _ => {
                return Some(jsonrpc_error(
                    id,
                    -32601,
                    format!("method not found: {method}"),
                ));
            }
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
            Err(error) => jsonrpc_error(id, -32602, error),
        })
    }

    fn call_tool(&self, params: &Value) -> Result<Value, String> {
        let name = required_string(params, "name")?;
        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let result = match name {
            "repl" => self.call_repl(&arguments),
            "repl-manage" => self.call_repl_manage(&arguments),
            _ => return Err(format!("unknown tool: {name}")),
        };
        Ok(match result {
            Ok(text) => tool_result(text, false),
            Err(error) => tool_result(error, true),
        })
    }

    fn call_repl(&self, arguments: &Value) -> Result<String, String> {
        let name = required_string(arguments, "name")?.to_owned();
        let code = required_string(arguments, "code")?.to_owned();
        match self.dispatch(KernelOperation::Exec { name, code })? {
            KernelResponse::Executed { response } => {
                let mut output = String::new();
                if !response.ok {
                    output.push_str(response.error.as_deref().unwrap_or("execution failed"));
                } else if let Some(result) = response.result {
                    match result {
                        Value::String(value) => output.push_str(&value),
                        value => {
                            output.push_str(&serde_json::to_string(&value).unwrap_or_default())
                        }
                    }
                }
                if !response.stdout.is_empty() {
                    append_output(&mut output, "stdout", response.stdout.trim());
                }
                if !response.stderr.is_empty() {
                    append_output(&mut output, "stderr", response.stderr.trim());
                }
                if output.is_empty() {
                    output.push_str("(ok)");
                }
                Ok(output)
            }
            _ => Err("replmux returned an unexpected execution response".to_owned()),
        }
    }

    fn call_repl_manage(&self, arguments: &Value) -> Result<String, String> {
        let action = required_string(arguments, "action")?;
        let supplied_name = arguments.get("name").and_then(Value::as_str);
        let operation = match action {
            "create" => KernelOperation::Create {
                name: supplied_name.map_or_else(generated_name, str::to_owned),
                kernelspec: arguments
                    .get("kernelspec")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
            "list" => KernelOperation::List,
            "connect" => KernelOperation::Connect {
                name: require_name(supplied_name, action)?,
            },
            "delete" => KernelOperation::Delete {
                name: require_name(supplied_name, action)?,
            },
            _ => return Err(format!("unsupported repl-manage action: {action}")),
        };
        serde_json::to_string_pretty(&self.dispatch(operation)?).map_err(|error| error.to_string())
    }

    fn dispatch(&self, operation: KernelOperation) -> Result<KernelResponse, String> {
        dispatch(
            KernelRequest {
                operation,
                kernel_dir: self.kernel_dir.clone(),
                python: self.python.clone(),
                kernel_script: self.kernel_script.clone(),
            },
            self.transport,
            &self.broker_socket,
        )
    }
}

fn initialize_result(request: &Value) -> Value {
    let requested_version = request
        .pointer("/params/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);
    json!({
        "protocolVersion": requested_version,
        "capabilities": {"tools": {}},
        "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION}
    })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "repl",
            "description": "Execute code in a named persistent Replmux kernel. State persists across calls.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Running kernel name"},
                    "code": {"type": "string", "description": "Code in the kernel's language"}
                },
                "required": ["name", "code"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "repl-manage",
            "description": "Create, list, connect to, or delete Replmux kernels.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["create", "list", "connect", "delete"]},
                    "name": {"type": "string", "description": "Kernel name; optional for create"},
                    "kernelspec": {"type": "string", "description": "Installed kernelspec name or kernel.json path for create"}
                },
                "required": ["action"],
                "additionalProperties": false
            }
        }),
    ]
}

fn required_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{field} must be a non-empty string"))
}

fn require_name(name: Option<&str>, action: &str) -> Result<String, String> {
    name.filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("name is required for {action}"))
}

fn generated_name() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("repl-{}-{timestamp}", std::process::id())
}

fn append_output(output: &mut String, label: &str, value: &str) {
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str(label);
    output.push_str(": ");
    output.push_str(value);
}

fn tool_result(text: String, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn jsonrpc_error(id: Value, code: i32, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message}
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> McpServer {
        McpServer::new(
            None,
            None,
            None,
            TransportMode::Local,
            PathBuf::from("/tmp/replmux-mcp-test.sock"),
        )
    }

    #[test]
    fn initializes_and_advertises_two_tools() {
        let initialize = server()
            .handle(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": "2025-06-18"}
            }))
            .unwrap();
        assert_eq!(initialize["result"]["protocolVersion"], "2025-06-18");

        let tools = server()
            .handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}))
            .unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn ignores_notifications_and_rejects_unknown_methods() {
        assert!(
            server()
                .handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
                .is_none()
        );
        let response = server()
            .handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "missing"}))
            .unwrap();
        assert_eq!(response["error"]["code"], -32601);
    }
}
