#!/usr/bin/env python3
"""
shared_repl.py — Shared Python REPL over a Unix socket.

Connections are serialised (one exec at a time) so agents never
stomp each other.  get()/set() provide atomic cross-agent state.
"""

import io, json, os, socket, sys, threading, traceback


class SharedREPL:
    def __init__(self):
        self.shared = {}
        self.lock = threading.Lock()

    # -- connection handler (runs in a thread) -------------------------------

    def handle(self, conn):
        data = b""
        while True:
            chunk = conn.recv(65536)
            if not chunk:
                break
            data += chunk
            if len(chunk) < 65536:
                break

        code = data.decode("utf-8", errors="replace").strip()
        if not code:
            conn.sendall(b'{"ok":false,"error":"empty"}')
            conn.close()
            return

        result = self.exec(code)
        conn.sendall(json.dumps(result).encode())
        conn.close()

    # -- execution (serialised by self.lock) --------------------------------

    def exec(self, code: str) -> dict:
        """Run code in a sandbox with get()/set() for safe shared state."""
        ns = {"__builtins__": __builtins__, "get": self.get, "set": self.set}

        captured = io.StringIO()
        old = sys.stdout
        sys.stdout = captured
        error = None
        returned = None

        import ast as _ast

        # If code is a single expression, eval it so the result is captured
        try:
            tree = _ast.parse(code.strip(), mode="eval")
            returned = repr(eval(compile(tree, "<repl>", "eval"), ns))
        except (SyntaxError, ValueError):
            try:
                compiled = compile(code.strip(), "<repl>", "exec")
                exec(compiled, ns)
            except Exception:
                error = traceback.format_exc()
        finally:
            sys.stdout = old

        return {
            "ok": error is None,
            "stdout": captured.getvalue(),
            "result": returned,
            "error": error,
        }

    # -- atomic shared state ------------------------------------------------

    def get(self, key: str, default=None):
        with self.lock:
            return self.shared.get(key, default)

    def set(self, key: str, value):
        with self.lock:
            self.shared[key] = value

    # -- server loop --------------------------------------------------------

    def run(self, path: str):
        try:
            os.unlink(path)
        except OSError:
            pass

        server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        server.bind(path)
        server.listen(5)
        os.chmod(path, 0o777)
        print(f"repl ready: {path}", flush=True)

        try:
            while True:
                conn, _ = server.accept()
                threading.Thread(target=self.handle, args=(conn,), daemon=True).start()
        except KeyboardInterrupt:
            print("shutdown", flush=True)
        finally:
            server.close()
            try:
                os.unlink(path)
            except OSError:
                pass


# -- entry point ---------------------------------------------------------

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("socket", nargs="?", default="/tmp/repl.sock")
    parser.add_argument("code", nargs="*")
    args = parser.parse_args()

    # One-shot: code provided as arguments or piped on stdin
    if args.code:
        code = " ".join(args.code)
    elif not sys.stdin.isatty():
        code = sys.stdin.read()
    else:
        code = None

    if code:
        code = code.strip()
        if not code:
            print("repl-client: no code provided", file=sys.stderr)
            sys.exit(1)
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.connect(args.socket)
        sock.sendall(code.encode())
        sock.shutdown(socket.SHUT_WR)
        data = b""
        while True:
            chunk = sock.recv(65536)
            if not chunk:
                break
            data += chunk
        print(data.decode())
        sock.close()
        sys.exit(0)

    # Daemon mode
    SharedREPL().run(args.socket)
