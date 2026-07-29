#!/usr/bin/env python3
"""Regression tests for kernel socket request framing.

Guards against the truncation bug where a short read was treated as
end-of-message. Payloads that span multiple segments were silently truncated
mid-JSON, surfacing as "Unterminated string" parse errors and EPIPE on the
client. See ticket rep-ip1m.

Run: python3 cli/tests/test_socket_framing.py
"""

import importlib.util
import json
import os
import socket
import sys
import tempfile
import threading
import time
import types
import unittest
from pathlib import Path

KERNEL_PATH = Path(__file__).resolve().parents[1] / "assets" / "python_minimal_kernel.py"


def load_kernel_module():
    """Import the kernel module, stubbing pyzmq only if it is unavailable.

    The kernel annotates parameters with zmq types that are evaluated at import
    time, so a stub must expose those names.
    """
    try:
        import zmq  # noqa: F401
    except ImportError:
        stub = types.ModuleType("zmq")
        for name in ("Socket", "Context"):
            setattr(stub, name, type(name, (), {}))
        sys.modules["zmq"] = stub
    spec = importlib.util.spec_from_file_location("_replmux_kernel", KERNEL_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


recv_until_eof = load_kernel_module().recv_until_eof


class RecvUntilEofTest(unittest.TestCase):
    def round_trip(self, payload, segments):
        """Send payload in the given segment sizes; return what the server read."""
        directory = tempfile.mkdtemp()
        socket_path = os.path.join(directory, "s")
        received = {}

        server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        server.bind(socket_path)
        server.listen(1)

        def serve():
            conn, _ = server.accept()
            try:
                received["data"] = recv_until_eof(conn)
            finally:
                conn.close()
                server.close()

        thread = threading.Thread(target=serve)
        thread.start()

        client = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        client.connect(socket_path)
        try:
            offset = 0
            for size in segments:
                client.sendall(payload[offset:offset + size])
                offset += size
                time.sleep(0.05)  # force a separate segment
            client.sendall(payload[offset:])
            client.shutdown(socket.SHUT_WR)  # half-close, as real clients do
        finally:
            client.close()

        thread.join(timeout=10)
        return received.get("data", b"")

    def test_multi_segment_payload_is_not_truncated(self):
        payload = json.dumps({"code": "x = '" + "A" * 9000 + "'"}).encode()
        data = self.round_trip(payload, [4000])
        self.assertEqual(data, payload)
        self.assertEqual(json.loads(data.decode())["code"][:5], "x = '")

    def test_many_small_segments(self):
        payload = json.dumps({"code": "y = " + str(list(range(3000)))}).encode()
        data = self.round_trip(payload, [10, 100, 1000, 50])
        self.assertEqual(data, payload)

    def test_single_segment_still_works(self):
        payload = json.dumps({"code": "1 + 1"}).encode()
        data = self.round_trip(payload, [])
        self.assertEqual(data, payload)

    def test_empty_request(self):
        self.assertEqual(self.round_trip(b"", []), b"")

    def test_payload_larger_than_buffer(self):
        payload = json.dumps({"code": "z = '" + "B" * 200000 + "'"}).encode()
        self.assertGreater(len(payload), 65536)
        data = self.round_trip(payload, [])
        self.assertEqual(data, payload)


if __name__ == "__main__":
    unittest.main(verbosity=2)
