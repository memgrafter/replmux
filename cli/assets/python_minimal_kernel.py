#!/usr/bin/env python3
"""
Minimal Jupyter kernel. Single dep: pyzmq.

Subclass Kernel and override do_execute() to make your own.
"""

import hashlib
import hmac
import io
import json
import os
import socket as socket_module
import sys
import threading
import time
import traceback
import uuid
from dataclasses import dataclass, field
from typing import Any

import zmq


# ---------------------------------------------------------------------------
# Wire protocol
# ---------------------------------------------------------------------------


def send_message(
    socket: zmq.Socket,
    key: bytes,
    msg_type: str,
    content: dict,
    parent_header: dict | None = None,
    identity: bytes | None = None,
):
    """Build, HMAC-sign, and send a Jupyter protocol message."""
    header = {
        "msg_id": uuid.uuid4().hex,
        "msg_type": msg_type,
        "username": "kernel",
        "session": uuid.uuid4().hex,
        "date": time.strftime("%Y-%m-%dT%H:%M:%S.000000Z", time.gmtime()),
        "version": "5.3",
    }

    parent = parent_header or {}
    metadata: dict = {}

    serialized_header = json.dumps(header, separators=(",", ":")).encode()
    serialized_parent = json.dumps(parent, separators=(",", ":")).encode()
    serialized_metadata = json.dumps(metadata, separators=(",", ":")).encode()
    serialized_content = json.dumps(content, separators=(",", ":")).encode()

    signature = hmac.new(
        key,
        b"".join([serialized_header, serialized_parent, serialized_metadata, serialized_content]),
        hashlib.sha256,
    ).hexdigest()

    frames: list[bytes] = []
    if identity:
        frames.append(identity)
    frames += [
        b"<IDS|MSG>",
        signature.encode(),
        serialized_header,
        serialized_parent,
        serialized_metadata,
        serialized_content,
    ]
    socket.send_multipart(frames)


def parse_message(frames: list[bytes]) -> dict | None:
    """Parse a received multipart message into a dict with header + content."""
    try:
        delim = frames.index(b"<IDS|MSG>")
    except ValueError:
        return None

    body = frames[delim + 1:]
    if len(body) < 5:
        return None

    try:
        return {
            "header": json.loads(body[1]),
            "content": json.loads(body[4]),
        }
    except (json.JSONDecodeError, IndexError):
        return None


# ---------------------------------------------------------------------------
# Kernel
# ---------------------------------------------------------------------------


@dataclass
class ConnectionInfo:
    shell_port: int
    iopub_port: int
    control_port: int
    hb_port: int
    ip: str = "127.0.0.1"
    transport: str = "tcp"
    signature_scheme: str = "hmac-sha256"
    kernel_name: str = "python3"


class Kernel:
    """
    A Jupyter kernel backed by a persistent Python namespace.

    Subclass and override do_execute() to customise execution behaviour.
    """

    implementation: str = "minimal_kernel"
    implementation_version: str = "0.1"
    language: str = "python"
    language_version: str = sys.version.split()[0]

    def __init__(self, connection_file: str):
        self.connection_file = connection_file
        self.namespace: dict = {"__builtins__": __builtins__}
        self.execution_count: int = 0
        self.running: bool = True
        self._lock = threading.RLock()  # reentrant: socket handler calls do_execute

        # Wire secrets
        supplied_connection = self._read_connection_file()
        supplied_key = supplied_connection.get("key", "") if supplied_connection else ""
        if isinstance(supplied_key, str) and supplied_key:
            self.connection_key = supplied_key
        else:
            self.connection_key = os.urandom(32).hex()
        self.key = self.connection_key.encode()
        self.session_id: str = uuid.uuid4().hex

        # ZMQ
        context = zmq.Context()
        self.shell_socket: zmq.Socket = context.socket(zmq.ROUTER)
        self.iopub_socket: zmq.Socket = context.socket(zmq.PUB)
        self.control_socket: zmq.Socket = context.socket(zmq.ROUTER)
        self.heartbeat_socket: zmq.Socket = context.socket(zmq.REP)

        # Bind to caller-supplied Jupyter ports when present; otherwise allocate them.
        if supplied_connection:
            if supplied_connection.get("transport", "tcp") != "tcp":
                raise ValueError("minimal kernel supports only tcp transport")
            ip = supplied_connection.get("ip", "127.0.0.1")
            self.shell_socket.bind(f"tcp://{ip}:{supplied_connection['shell_port']}")
            self.iopub_socket.bind(f"tcp://{ip}:{supplied_connection['iopub_port']}")
            self.control_socket.bind(f"tcp://{ip}:{supplied_connection['control_port']}")
            self.heartbeat_socket.bind(f"tcp://{ip}:{supplied_connection['hb_port']}")
        else:
            self.shell_socket.bind_to_random_port("tcp://127.0.0.1")
            self.iopub_socket.bind_to_random_port("tcp://127.0.0.1")
            self.control_socket.bind_to_random_port("tcp://127.0.0.1")
            self.heartbeat_socket.bind_to_random_port("tcp://127.0.0.1")

        # Persist connection info (includes socket path)
        self._write_connection_file()

        # Background threads
        threading.Thread(target=self._heartbeat_loop, daemon=True).start()
        threading.Thread(target=self._socket_loop, daemon=True).start()

    # -- connection file ---------------------------------------------------

    def _read_connection_file(self) -> dict[str, Any] | None:
        try:
            with open(self.connection_file) as connection_file:
                connection = json.load(connection_file)
        except (OSError, json.JSONDecodeError):
            return None
        required_ports = ("shell_port", "iopub_port", "control_port", "hb_port")
        return connection if all(connection.get(port) for port in required_ports) else None

    def _write_connection_file(self):
        # Socket path for direct extension access (no subprocess)
        self.socket_path = self.connection_file.replace(".json", ".sock")

        info = ConnectionInfo(
            shell_port=self.shell_socket.getsockopt(zmq.LAST_ENDPOINT).decode().split(":")[-1],
            iopub_port=self.iopub_socket.getsockopt(zmq.LAST_ENDPOINT).decode().split(":")[-1],
            control_port=self.control_socket.getsockopt(zmq.LAST_ENDPOINT).decode().split(":")[-1],
            hb_port=self.heartbeat_socket.getsockopt(zmq.LAST_ENDPOINT).decode().split(":")[-1],
        )
        with open(self.connection_file, "w") as f:
            json.dump({
                "shell_port": int(info.shell_port),
                "iopub_port": int(info.iopub_port),
                "control_port": int(info.control_port),
                "hb_port": int(info.hb_port),
                "stdin_port": 0,
                "ip": info.ip,
                "key": self.connection_key,
                "transport": info.transport,
                "signature_scheme": info.signature_scheme,
                "kernel_name": info.kernel_name,
                "socket_path": self.socket_path,
            }, f)

    # -- iopub helpers -----------------------------------------------------

    def publish(self, msg_type: str, content: dict, parent: dict | None = None):
        send_message(self.iopub_socket, self.key, msg_type, content, parent)

    def reply(self, msg_type: str, content: dict, parent_msg: dict, identity: bytes):
        send_message(self.shell_socket, self.key, msg_type, content, parent_msg["header"], identity)

    # -- heartbeat ---------------------------------------------------------

    def _heartbeat_loop(self):
        while self.running:
            try:
                self.heartbeat_socket.send(self.heartbeat_socket.recv())
            except zmq.ZMQError:
                break

    # -- message handlers --------------------------------------------------

    def handle_execute_request(self, msg: dict, identity: bytes):
        content = msg["content"]
        code = content.get("code", "")
        silent = content.get("silent", False)

        self.execution_count += 1

        self.publish("status", {"execution_state": "busy"}, msg["header"])

        if not silent:
            self.publish(
                "execute_input",
                {"code": code, "execution_count": self.execution_count},
                msg["header"],
            )

        # Run
        stdout_capture = io.StringIO()
        stderr_capture = io.StringIO()
        old_stdout, old_stderr = sys.stdout, sys.stderr
        error = None
        expression_result = None

        try:
            sys.stdout = stdout_capture
            sys.stderr = stderr_capture
            self.do_execute(code)
        except Exception:
            error = {
                "status": "error",
                "execution_count": self.execution_count,
                "ename": sys.exc_info()[0].__name__,
                "evalue": str(sys.exc_info()[1]),
                "traceback": traceback.format_exc().split("\n"),
            }
        finally:
            sys.stdout, sys.stderr = old_stdout, old_stderr

        # IOPub: stdout / stderr
        stdout_text = stdout_capture.getvalue()
        stderr_text = stderr_capture.getvalue()

        if not silent:
            if stdout_text:
                self.publish("stream", {"name": "stdout", "text": stdout_text}, msg["header"])
            if stderr_text:
                self.publish("stream", {"name": "stderr", "text": stderr_text}, msg["header"])

            # IOPub: execution result (last expression in namespace)
            last = self.namespace.get("_")
            if last is not None:
                expression_result = {"text/plain": repr(last)}
                self.publish(
                    "execute_result",
                    {
                        "execution_count": self.execution_count,
                        "data": expression_result,
                        "metadata": {},
                    },
                    msg["header"],
                )

        # Shell reply
        if error:
            self.reply("execute_reply", error, msg, identity)
        else:
            self.reply(
                "execute_reply",
                {
                    "status": "ok",
                    "execution_count": self.execution_count,
                    "payload": [],
                    "user_expressions": {},
                },
                msg,
                identity,
            )

        self.publish("status", {"execution_state": "idle"}, msg["header"])

    def handle_kernel_info_request(self, msg: dict, identity: bytes):
        self.reply(
            "kernel_info_reply",
            {
                "protocol_version": "5.3",
                "implementation": self.implementation,
                "implementation_version": self.implementation_version,
                "language_info": {
                    "name": self.language,
                    "version": self.language_version,
                    "mimetype": "text/x-python",
                    "file_extension": ".py",
                    "pygments_lexer": "ipython3",
                    "codemirror_mode": {"name": "ipython", "version": 3},
                },
                "banner": f"{self.implementation} v{self.implementation_version}",
                "help_links": [],
            },
            msg,
            identity,
        )

    def handle_complete_request(self, msg: dict, identity: bytes):
        import re
        import rlcompleter

        code = msg["content"].get("code", "")
        cursor_pos = msg["content"].get("cursor_pos", len(code))
        before_cursor = code[:cursor_pos]
        match = re.search(r"[A-Za-z_][A-Za-z0-9_.]*$", before_cursor)
        token = match.group(0) if match else ""
        completer = rlcompleter.Completer(self.namespace)
        matches: list[str] = []
        index = 0
        while True:
            completion = completer.complete(token, index)
            if completion is None:
                break
            if completion not in matches:
                matches.append(completion)
            index += 1
        self.reply(
            "complete_reply",
            {
                "status": "ok",
                "matches": matches,
                "cursor_start": cursor_pos - len(token),
                "cursor_end": cursor_pos,
                "metadata": {},
            },
            msg,
            identity,
        )

    def handle_inspect_request(self, msg: dict, identity: bytes):
        import inspect
        import re

        code = msg["content"].get("code", "")
        cursor_pos = msg["content"].get("cursor_pos", len(code))
        match = re.search(r"[A-Za-z_][A-Za-z0-9_.]*$", code[:cursor_pos])
        token = match.group(0) if match else ""
        found = False
        data: dict[str, str] = {}
        if token:
            try:
                value = eval(token, self.namespace)
                documentation = inspect.getdoc(value) or repr(value)
                data = {"text/plain": documentation}
                found = True
            except Exception:
                pass
        self.reply(
            "inspect_reply",
            {"status": "ok", "found": found, "data": data, "metadata": {}},
            msg,
            identity,
        )

    def handle_is_complete_request(self, msg: dict, identity: bytes):
        import codeop

        code = msg["content"].get("code", "")
        try:
            compiled = codeop.compile_command(code, symbol="exec")
            content = {"status": "complete" if compiled is not None else "incomplete"}
            if compiled is None:
                content["indent"] = "    "
        except (SyntaxError, OverflowError, ValueError):
            content = {"status": "invalid"}
        self.reply("is_complete_reply", content, msg, identity)

    def handle_interrupt_request(self, msg: dict, identity: bytes):
        self.reply("interrupt_reply", {"status": "ok"}, msg, identity)

    def handle_shutdown_request(self, msg: dict, identity: bytes):
        restart = msg["content"].get("restart", False)
        self.reply("shutdown_reply", {"restart": restart, "status": "ok"}, msg, identity)
        self.running = False

    # -- direct JSON socket (extension ↔ kernel, no subprocess) -----------

    def _socket_loop(self):
        try:
            os.unlink(self.socket_path)
        except OSError:
            pass

        server = socket_module.socket(socket_module.AF_UNIX, socket_module.SOCK_STREAM)
        server.bind(self.socket_path)
        server.listen(5)
        os.chmod(self.socket_path, 0o777)

        try:
            while self.running:
                server.settimeout(0.5)
                try:
                    conn, _ = server.accept()
                except socket_module.timeout:
                    continue
                threading.Thread(target=self._handle_socket_client, args=(conn,), daemon=True).start()
        finally:
            server.close()
            try:
                os.unlink(self.socket_path)
            except OSError:
                pass

    def _handle_socket_client(self, conn):
        try:
            data = b""
            while True:
                chunk = conn.recv(65536)
                if not chunk:
                    break
                data += chunk
                if len(chunk) < 65536:
                    break

            req = json.loads(data.decode())
            code = req.get("code", "")
            result = self._execute_direct(code)
            conn.sendall(json.dumps(result).encode())
        except Exception as e:
            conn.sendall(json.dumps({"ok": False, "error": str(e)}).encode())
        finally:
            conn.close()

    def _execute_direct(self, code: str) -> dict:
        """Execute code directly (from socket), return JSON result."""
        with self._lock:
            stdout_capture = io.StringIO()
            stderr_capture = io.StringIO()
            old_stdout, old_stderr = sys.stdout, sys.stderr
            error = None
            expression_result = None

            # Determine mode from AST before execution so errors report correctly
            import ast as _ast
            stripped = code.strip()
            try:
                tree = _ast.parse(stripped, mode="eval")
                mode = "eval"
            except (SyntaxError, ValueError):
                tree = None
                mode = "exec"

            try:
                sys.stdout = stdout_capture
                sys.stderr = stderr_capture
                if mode == "eval" and tree is not None:
                    self.namespace["_"] = eval(compile(tree, "<repl>", "eval"), self.namespace)
                    expression_result = repr(self.namespace["_"])
                else:
                    self.namespace.pop("_", None)
                    exec(compile(stripped, "<repl>", "exec"), self.namespace)
            except Exception:
                error = f"{sys.exc_info()[0].__name__}: {sys.exc_info()[1]}"
            finally:
                sys.stdout, sys.stderr = old_stdout, old_stderr

            return {
                "ok": error is None,
                "mode": mode,
                "code": code.strip()[:200],
                "stdout": stdout_capture.getvalue(),
                "stderr": stderr_capture.getvalue(),
                "result": expression_result,
                "error": error,
            }

    # -- override point ----------------------------------------------------

    def do_execute(self, code: str):
        """Execute Python code in the shared namespace. Override to customise."""
        with self._lock:
            import ast as _ast
            code = code.strip()
            try:
                tree = _ast.parse(code, mode="eval")
                self.namespace["_"] = eval(compile(tree, "<repl>", "eval"), self.namespace)
            except (SyntaxError, ValueError):
                self.namespace.pop("_", None)
                exec(compile(code, "<repl>", "exec"), self.namespace)

    # -- main loop ---------------------------------------------------------

    def run(self):
        shell_handlers = {
            "execute_request": self.handle_execute_request,
            "kernel_info_request": self.handle_kernel_info_request,
            "complete_request": self.handle_complete_request,
            "inspect_request": self.handle_inspect_request,
            "is_complete_request": self.handle_is_complete_request,
            "interrupt_request": self.handle_interrupt_request,
            "shutdown_request": self.handle_shutdown_request,
        }

        poller = zmq.Poller()
        poller.register(self.shell_socket, zmq.POLLIN)
        poller.register(self.control_socket, zmq.POLLIN)

        print(f"Kernel ready: {self.connection_file}", flush=True)

        while self.running:
            try:
                events = dict(poller.poll(timeout=250))
            except zmq.ZMQError:
                break

            for socket in (self.shell_socket, self.control_socket):
                if socket not in events:
                    continue

                try:
                    frames = socket.recv_multipart()
                except zmq.ZMQError:
                    continue

                parsed = parse_message(frames)
                if parsed is None:
                    continue

                identity = frames[0]
                handler = shell_handlers.get(parsed["header"].get("msg_type", ""))
                if handler:
                    handler(parsed, identity)

        # Cleanup
        for s in (self.shell_socket, self.iopub_socket, self.control_socket, self.heartbeat_socket):
            s.close(linger=0)

        for path in (self.connection_file, self.socket_path):
            try:
                os.remove(path)
            except OSError:
                pass

        print("Kernel shut down.", flush=True)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import signal
    import tempfile

    connection_file = os.environ.get("KERNEL_CONNECTION_FILE")
    if not connection_file:
        connection_file = os.path.join(tempfile.mkdtemp(), "kernel.json")

    kernel = Kernel(connection_file)

    def cleanup(*_):
        kernel.running = False

    signal.signal(signal.SIGTERM, cleanup)
    signal.signal(signal.SIGINT, cleanup)

    kernel.run()
