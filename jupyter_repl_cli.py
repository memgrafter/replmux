#!/usr/bin/env python3
"""CLI for managing Jupyter REPL kernel lifecycles.

Usage:
    jupyter-repl create <name>      Start a named kernel
    jupyter-repl list               Show running kernels
    jupyter-repl connect <name>     Print connection JSON for the client
    jupyter-repl delete <name>      Shut down a named kernel
"""

import json
import os
import signal
import subprocess
import sys

REPL_DIR = os.path.expanduser("~/.jupyter-repl/kernels")


def _kernel_path() -> str:
    """Path to the minimal kernel script."""
    return os.path.join(os.path.dirname(__file__), "minimal_kernel_clean.py")


def _conn_path(name: str) -> str:
    """Connection file path for a named kernel."""
    return os.path.join(REPL_DIR, f"{name}.json")


def _pid_path(name: str) -> str:
    """PID file path for a named kernel."""
    return os.path.join(REPL_DIR, f"{name}.pid")


def _is_alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False


def cmd_create(name: str) -> None:
    """Start a new kernel with the given name."""
    conn_path = _conn_path(name)
    pid_path = _pid_path(name)

    if os.path.exists(conn_path):
        existing_pid = int(open(pid_path).read().strip())
        if _is_alive(existing_pid):
            print(f"Error: kernel '{name}' is already running (pid {existing_pid})", file=sys.stderr)
            sys.exit(1)
        # Stale conn file — clean up
        os.remove(conn_path)
        if os.path.exists(pid_path):
            os.remove(pid_path)

    os.makedirs(REPL_DIR, exist_ok=True)

    python = sys.executable or "python3"
    proc = subprocess.Popen(
        [python, _kernel_path()],
        env={**os.environ, "KERNEL_CONNECTION_FILE": conn_path},
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    # Wait briefly for the kernel to write its connection file
    import time
    for _ in range(20):  # up to 2s
        if os.path.exists(conn_path):
            break
        time.sleep(0.1)
    else:
        proc.kill()
        print(f"Error: kernel '{name}' failed to start", file=sys.stderr)
        sys.exit(1)

    with open(pid_path, "w") as f:
        f.write(str(proc.pid))

    print(f"Kernel '{name}' started (pid {proc.pid})")


def cmd_list() -> None:
    """Show all kernels and their status."""
    if not os.path.exists(REPL_DIR):
        print("No kernels found.")
        return

    names = sorted(n.replace(".json", "") for n in os.listdir(REPL_DIR) if n.endswith(".json"))
    if not names:
        print("No kernels found.")
        return

    print(f"{'NAME':<20} {'PID':<10} {'STATUS':<10}")
    for name in names:
        pid_path = _pid_path(name)
        if os.path.exists(pid_path):
            pid = int(open(pid_path).read().strip())
            status = "running" if _is_alive(pid) else "dead"
        else:
            pid, status = "?", "no-pid"
        print(f"{name:<20} {str(pid):<10} {status:<10}")


def cmd_connect(name: str) -> None:
    """Print connection info as JSON."""
    conn_path = _conn_path(name)
    if not os.path.exists(conn_path):
        print(f"Error: kernel '{name}' not found", file=sys.stderr)
        sys.exit(1)

    with open(conn_path) as f:
        data = json.load(f)
    print(json.dumps(data, indent=2))


def cmd_delete(name: str) -> None:
    """Shut down a named kernel."""
    conn_path = _conn_path(name)
    pid_path = _pid_path(name)

    if not os.path.exists(conn_path):
        print(f"Error: kernel '{name}' not found", file=sys.stderr)
        sys.exit(1)

    # Try graceful shutdown via the control channel
    try:
        from jupyter_repl import KernelClient
        with open(conn_path) as f:
            conn = json.load(f)
        client = KernelClient(conn)
        client.shutdown(timeout=5)
        client.close()
    except Exception:
        pass  # fall back to kill

    # Kill the process if it's still alive
    if os.path.exists(pid_path):
        pid = int(open(pid_path).read().strip())
        if _is_alive(pid):
            os.kill(pid, signal.SIGTERM)
            try:
                import time
                for _ in range(10):  # up to 1s
                    if not _is_alive(pid):
                        break
                    time.sleep(0.1)
            except OSError:
                pass
            else:
                os.kill(pid, signal.SIGKILL)

    # Clean up files
    for path in (conn_path, pid_path):
        try:
            os.remove(path)
        except OSError:
            pass

    print(f"Kernel '{name}' shut down.")


def main() -> None:
    if len(sys.argv) < 2:
        print("Usage: jupyter-repl <command> [args]", file=sys.stderr)
        print("Commands: create, list, connect, delete", file=sys.stderr)
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == "create":
        if len(sys.argv) < 3:
            print("Usage: jupyter-repl create <name>", file=sys.stderr)
            sys.exit(1)
        cmd_create(sys.argv[2])

    elif cmd == "list":
        cmd_list()

    elif cmd == "connect":
        if len(sys.argv) < 3:
            print("Usage: jupyter-repl connect <name>", file=sys.stderr)
            sys.exit(1)
        cmd_connect(sys.argv[2])

    elif cmd == "delete":
        if len(sys.argv) < 3:
            print("Usage: jupyter-repl delete <name>", file=sys.stderr)
            sys.exit(1)
        cmd_delete(sys.argv[2])

    else:
        print(f"Unknown command: {cmd}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
