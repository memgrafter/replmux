# jupyter_repl — Minimal Jupyter protocol client
#
# Extracted from jupyter_client (~19K LOC) to ~300 lines.
# Dependencies: pyzmq only.
#
# Connects to any running Jupyter kernel (ipykernel, xeus-cling, IRkernel,
# IJulia, etc.) and lets you execute code, get rich output, interrupt,
# tab-complete, inspect, and monitor heartbeat.

from __future__ import annotations

import hashlib
import hmac
import json
import os
import time
import typing as t
from datetime import datetime, timezone
from threading import Thread

import zmq

DELIM = b"<IDS|MSG>"
PROTOCOL_VERSION = "5.4"


def _new_id() -> str:
    buf = os.urandom(16)
    return "-".join([buf[:4].hex(), buf[4:].hex()])


def _json_default(obj: t.Any) -> str:
    if isinstance(obj, datetime):
        return obj.isoformat()
    raise TypeError(f"Object of type {type(obj)} is not JSON serializable")


def _pack(obj: t.Any) -> bytes:
    return json.dumps(obj, default=_json_default, ensure_ascii=False, allow_nan=False).encode(
        "utf8", errors="surrogateescape"
    )


def _unpack(s: str | bytes) -> t.Any:
    if isinstance(s, bytes):
        s = s.decode("utf8", "replace")
    return json.loads(s)


def _sign(msg_parts: list[bytes], auth: hmac.HMAC | None) -> bytes:
    if auth is None:
        return b""
    h = auth.copy()
    for part in msg_parts:
        h.update(part)
    return h.hexdigest().encode()


def _serialize(msg: dict, pack: t.Callable, auth: hmac.HMAC | None) -> list[bytes]:
    real_message = [
        pack(msg["header"]),
        pack(msg["parent_header"]),
        pack(msg["metadata"]),
        pack(msg["content"]),
    ]
    to_send = [DELIM, _sign(real_message, auth)]
    to_send.extend(real_message)
    return to_send


def _split_idents(msg_list: list[bytes]) -> tuple[list[bytes], list[bytes]]:
    """Split identity prefixes from the message body.

    Returns (idents, body) where body is [HMAC, p_header, p_parent, p_metadata, p_content].
    """
    try:
        idx = msg_list.index(DELIM)
    except ValueError:
        return [], msg_list
    return msg_list[:idx], msg_list[idx + 1:]


def _deserialize(msg_list: list[bytes], unpack: t.Callable, auth: hmac.HMAC | None,
                 strict: bool = True) -> dict:
    """Deserialize a zmq message into a protocol message dict.

    strict=True enforces HMAC validation; strict=False (IOPub) tolerates missing signatures.
    """
    _, body = _split_idents(msg_list)
    if auth is not None and body:
        signature = body[0]
        if signature and strict:
            check = _sign(body[1:5], auth)
            if not hmac.compare_digest(signature, check):
                raise ValueError(f"Invalid Signature: {signature!r}")
    if len(body) < 5:
        raise TypeError("malformed message")

    header = unpack(body[1])
    return {
        "header": header,
        "msg_id": header["msg_id"],
        "msg_type": header["msg_type"],
        "parent_header": unpack(body[2]),
        "metadata": unpack(body[3]),
        "content": unpack(body[4]),
    }


class _HeartbeatThread(Thread):
    """Daemon thread that pings the kernel's heartbeat channel."""

    def __init__(self, context: zmq.Context, address: str) -> None:
        super().__init__()
        self.daemon = True
        self.context = context
        self.address = address
        self._running = False
        self._beating = False

    def run(self) -> None:
        self._running = True
        sock = self.context.socket(zmq.REQ)
        sock.linger = 1000
        sock.connect(self.address)
        while self._running:
            try:
                sock.send(b"ping", zmq.NOBLOCK)
                self._beating = bool(sock.poll(1000, zmq.POLLIN))
                if self._beating:
                    sock.recv(zmq.NOBLOCK)
            except zmq.ZMQError:
                self._beating = False
            time.sleep(0.5)
        sock.close()

    def is_beating(self) -> bool:
        return self._beating

    def stop(self) -> None:
        self._running = False
        self.join(timeout=2)


class KernelClient:
    """Minimal Jupyter protocol client.

    Connects to a running kernel and provides execute, complete, inspect,
    interrupt, and heartbeat methods.

    Parameters
    ----------
    conn_info : dict
        Connection info from a kernel's JSON connection file.  Keys:
        shell_port, iopub_port, stdin_port, control_port, hb_port,
        ip, key, transport, signature_scheme.
    """

    def __init__(self, conn_info: dict) -> None:
        self.conn_info = conn_info
        self.context = zmq.Context()

        # Session state
        self.session_id = _new_id()
        self.username = os.environ.get("USER", "username")
        self._msg_count = 0

        # HMAC signing — key may be hex-encoded (minimal_kernel) or raw string (ipykernel)
        key = conn_info.get("key", b"")
        if isinstance(key, str):
            try:
                key = bytes.fromhex(key)
            except ValueError:
                key = key.encode()
        scheme = conn_info.get("signature_scheme", "hmac-sha256")
        self.auth: hmac.HMAC | None = None
        if key:
            hash_name = scheme.split("-", 1)[1] if "-" in scheme else "sha256"
            digest_mod = getattr(hashlib, hash_name, hashlib.sha256)
            self.auth = hmac.HMAC(key, digestmod=digest_mod)

        # Transport
        transport = conn_info.get("transport", "tcp")
        ip = conn_info.get("ip", "127.0.0.1")

        def _url(channel: str) -> str:
            port = conn_info[f"{channel}_port"]
            return f"tcp://{ip}:{port}" if transport == "tcp" else f"{transport}://{ip}-{port}"

        # Sockets
        self.shell_socket = self.context.socket(zmq.DEALER)
        self.shell_socket.linger = 1000
        self.shell_socket.connect(_url("shell"))

        self.iopub_socket = self.context.socket(zmq.SUB)
        self.iopub_socket.linger = 1000
        self.iopub_socket.setsockopt(zmq.SUBSCRIBE, b"")
        self.iopub_socket.connect(_url("iopub"))

        self.stdin_socket = self.context.socket(zmq.DEALER)
        self.stdin_socket.linger = 1000
        self.stdin_socket.connect(_url("stdin"))

        self.control_socket = self.context.socket(zmq.DEALER)
        self.control_socket.linger = 1000
        self.control_socket.connect(_url("control"))

        # Heartbeat
        self._hb_thread: _HeartbeatThread | None = None

    def _next_msg_id(self) -> str:
        msg_id = f"{self.session_id}_{os.getpid()}_{self._msg_count}"
        self._msg_count += 1
        return msg_id

    def _build_message(self, msg_type: str, content: dict | None = None,
                       parent: dict | None = None) -> dict:
        msg_id = self._next_msg_id()
        header = {
            "msg_id": msg_id,
            "msg_type": msg_type,
            "username": self.username,
            "session": self.session_id,
            "date": datetime.now(timezone.utc),
            "version": PROTOCOL_VERSION,
        }
        return {
            "header": header,
            "msg_id": msg_id,
            "msg_type": msg_type,
            "parent_header": {} if parent is None else parent.get("header", {}),
            "content": content or {},
            "metadata": {},
        }

    def _send(self, socket: zmq.Socket, msg: dict) -> None:
        to_send = _serialize(msg, _pack, self.auth)
        socket.send_multipart(to_send)

    def _recv_signed(self, socket: zmq.Socket) -> dict:
        return _deserialize(socket.recv_multipart(), _unpack, self.auth, strict=True)

    def _recv_iopub(self, socket: zmq.Socket) -> dict:
        return _deserialize(socket.recv_multipart(), _unpack, self.auth, strict=False)

    # ------------------------------------------------------------------
    # Execute
    # ------------------------------------------------------------------

    def execute(
        self,
        code: str,
        *,
        silent: bool = False,
        store_history: bool = True,
        user_expressions: dict | None = None,
        allow_stdin: bool = True,
        stop_on_error: bool = True,
        timeout: float | None = 30,
    ) -> tuple[dict, list[dict]]:
        """Execute code in the kernel. Blocks until status: idle.

        Returns (reply, iopub_messages).
        """
        if user_expressions is None:
            user_expressions = {}

        content = {
            "code": code,
            "silent": silent,
            "store_history": store_history,
            "user_expressions": user_expressions,
            "allow_stdin": allow_stdin,
            "stop_on_error": stop_on_error,
        }
        msg = self._build_message("execute_request", content)
        self._send(self.shell_socket, msg)
        msg_id = msg["header"]["msg_id"]

        outputs: list[dict] = []
        deadline = time.monotonic() + timeout if timeout is not None else None

        poller = zmq.Poller()
        poller.register(self.iopub_socket, zmq.POLLIN)

        while True:
            remaining = 100
            if deadline is not None:
                remaining = max(0, (deadline - time.monotonic()) * 1000)

            events = dict(poller.poll(int(remaining)))
            if not events and deadline is not None:
                raise TimeoutError("Timeout waiting for execution to complete")

            while self.iopub_socket.poll(timeout=0):
                iopub_msg = self._recv_iopub(self.iopub_socket)
                if iopub_msg["parent_header"].get("msg_id") != msg_id:
                    continue
                outputs.append(iopub_msg)
                if (
                    iopub_msg["header"]["msg_type"] == "status"
                    and iopub_msg["content"].get("execution_state") == "idle"
                ):
                    break

            if self.shell_socket.poll(timeout=0):
                reply = self._recv_signed(self.shell_socket)
                if reply["parent_header"].get("msg_id") == msg_id:
                    return reply, outputs

    # ------------------------------------------------------------------
    # Shell channel requests (request/reply pattern)
    # ------------------------------------------------------------------

    def _send_recv(
        self, msg_type: str, content: dict | None = None, timeout: float = 30
    ) -> dict:
        """Send a request on the shell channel and wait for reply."""
        msg = self._build_message(msg_type, content)
        self._send(self.shell_socket, msg)
        msg_id = msg["header"]["msg_id"]

        deadline = time.monotonic() + timeout
        while True:
            remaining = max(0, (deadline - time.monotonic()) * 1000)
            if self.shell_socket.poll(int(remaining)):
                reply = self._recv_signed(self.shell_socket)
                if reply["parent_header"].get("msg_id") == msg_id:
                    return reply
            else:
                raise TimeoutError(f"Timeout waiting for {msg_type} reply")

    def complete(
        self, code: str, cursor_pos: int | None = None, timeout: float = 30
    ) -> dict:
        """Tab completion request."""
        if cursor_pos is None:
            cursor_pos = len(code)
        return self._send_recv("complete_request", {"code": code, "cursor_pos": cursor_pos}, timeout)

    def inspect(
        self,
        code: str,
        cursor_pos: int | None = None,
        detail_level: int = 0,
        timeout: float = 30,
    ) -> dict:
        """Object inspection request."""
        if cursor_pos is None:
            cursor_pos = len(code)
        return self._send_recv(
            "inspect_request",
            {"code": code, "cursor_pos": cursor_pos, "detail_level": detail_level},
            timeout,
        )

    def kernel_info(self, timeout: float = 30) -> dict:
        """Kernel info request."""
        return self._send_recv("kernel_info_request", None, timeout)

    def is_complete(self, code: str, timeout: float = 5) -> dict:
        """Check if code is syntactically complete."""
        return self._send_recv("is_complete_request", {"code": code}, timeout)

    # ------------------------------------------------------------------
    # Control channel requests
    # ------------------------------------------------------------------

    def interrupt(self) -> None:
        """Interrupt the kernel's current execution."""
        msg = self._build_message("interrupt_request")
        self._send(self.control_socket, msg)

    def shutdown(self, restart: bool = False, timeout: float = 10) -> dict:
        """Graceful shutdown via control channel."""
        msg = self._build_message("shutdown_request", {"restart": restart})
        self._send(self.control_socket, msg)

        deadline = time.monotonic() + timeout
        while True:
            remaining = max(0, (deadline - time.monotonic()) * 1000)
            if self.control_socket.poll(int(remaining)):
                return self._recv_signed(self.control_socket)
            raise TimeoutError("Timeout waiting for shutdown reply")

    # ------------------------------------------------------------------
    # Stdin
    # ------------------------------------------------------------------

    def input(self, value: str) -> None:
        """Send raw input to the kernel (in response to an input_request)."""
        msg = self._build_message("input_reply", {"value": value})
        self._send(self.stdin_socket, msg)

    # ------------------------------------------------------------------
    # Heartbeat
    # ------------------------------------------------------------------

    def start_heartbeat(self) -> None:
        """Start the heartbeat monitoring thread."""
        if self._hb_thread is not None and self._hb_thread.is_alive():
            return
        conn = self.conn_info
        ip = conn.get("ip", "127.0.0.1")
        transport = conn.get("transport", "tcp")
        hb_port = conn["hb_port"]
        address = f"tcp://{ip}:{hb_port}" if transport == "tcp" else f"{transport}://{ip}-{hb_port}"
        self._hb_thread = _HeartbeatThread(self.context, address)
        self._hb_thread.start()

    def stop_heartbeat(self) -> None:
        """Stop the heartbeat monitoring thread."""
        if self._hb_thread is not None:
            self._hb_thread.stop()
            self._hb_thread = None

    def is_alive(self) -> bool:
        """Check if the kernel's heartbeat is responding."""
        if self._hb_thread is None or not self._hb_thread.is_alive():
            return False
        return self._hb_thread.is_beating()

    # ------------------------------------------------------------------
    # Lifecycle
    # ------------------------------------------------------------------

    def close(self) -> None:
        """Close all sockets and destroy the zmq context."""
        self.stop_heartbeat()
        for sock in (self.shell_socket, self.iopub_socket, self.stdin_socket, self.control_socket):
            try:
                sock.close(linger=0)
            except Exception:
                pass
        self.context.destroy(linger=100)