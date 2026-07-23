#!/usr/bin/env python3
"""Full lifecycle test for the replmux project.

Usage:
    python3 lifecycle.py          # uses 'lifecycle-test' kernel
    python3 lifecycle.py myname   # uses 'myname' kernel

Requires pyzmq:  pip install pyzmq
"""
import json
import os
import socket
import subprocess
import sys
import time

TEST_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(TEST_DIR)
sys.path.insert(0, REPO)
REPL_DIR = os.path.expanduser("~/.jupyter-repl/kernels")

from jupyter_repl import KernelClient
from jupyter_repl_cli import cmd_create, cmd_list, cmd_connect, cmd_delete

KERNEL_NAME = sys.argv[1] if len(sys.argv) > 1 else "lifecycle-test"
SOCK_PATH = os.path.join(REPL_DIR, f"{KERNEL_NAME}.sock")
CONN_PATH = os.path.join(REPL_DIR, f"{KERNEL_NAME}.json")


def socket_exec(code):
    """Execute code via direct Unix socket (simulates replTool.ts path)."""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    sock.connect(SOCK_PATH)
    sock.sendall(json.dumps({"code": code}).encode())
    sock.settimeout(10)
    data = b""
    while True:
        chunk = sock.recv(65536)
        if not chunk:
            break
        data += chunk
        if len(chunk) < 65536:
            break
    sock.close()
    return json.loads(data)


def test(name, passed, detail=""):
    status = "PASS" if passed else "FAIL"
    msg = f"  [{status}] {name}"
    if detail:
        msg += f" — {detail}"
    print(msg)
    return passed


passed = 0
total = 0


def check(name, cond, detail=""):
    global passed, total
    total += 1
    if test(name, cond, detail):
        passed += 1


# ── Ensure kernel is running ──────────────────────────────────────────────

def ensure_kernel():
    """Create the test kernel if not running."""
    try:
        # Check if already running
        pid_path = os.path.join(REPL_DIR, f"{KERNEL_NAME}.pid")
        if os.path.exists(pid_path):
            pid = int(open(pid_path).read().strip())
            try:
                os.kill(pid, 0)
                return  # Already running
            except OSError:
                pass  # Stale PID, clean up
    except Exception:
        pass

    cmd_create(KERNEL_NAME)


print(f"Kernel: {KERNEL_NAME}")
ensure_kernel()

# Wait for socket to be ready
for _ in range(20):
    if os.path.exists(SOCK_PATH):
        break
    time.sleep(0.1)

# ── Socket Protocol Tests (replTool.ts path) ──────────────────────────────

print("\n=== Socket Protocol Tests ===\n")

# 1. Simple literal (eval)
r = socket_exec("42")
check("literal eval", r["mode"] == "eval" and r["result"] == "42" and r["ok"], r)

# 2. String expression (eval)
r = socket_exec("'hello'")
check("string eval", r["mode"] == "eval" and r["result"] == "'hello'" and r["ok"], r)

# 3. Variable assignment (exec)
r = socket_exec("foo = 'bar'")
check("assignment exec", r["mode"] == "exec" and r["result"] is None and r["ok"], r)

# 4. Variable readback (state persistence)
r = socket_exec("foo")
check("state persistence", r["mode"] == "eval" and r["result"] == "'bar'" and r["ok"], r)

# 5. Print stdout capture
r = socket_exec('print("stdout test")')
check("stdout capture", "stdout test" in r["stdout"] and r["ok"], r)

# 6. Stderr capture
r = socket_exec('import sys; sys.stderr.write("stderr test")')
check("stderr capture", "stderr test" in r["stderr"] and r["ok"], r)

# 7. Division error
r = socket_exec("1/0")
check("division error", not r["ok"] and "ZeroDivisionError" in (r["error"] or ""), r)

# 8. NameError
r = socket_exec("undefined_variable_xyz")
check("name error", not r["ok"] and "NameError" in (r["error"] or ""), r)

# 9. Import + expression (two statements = exec mode)
r = socket_exec("import os; os.path.sep")
check("import + expr (exec)", r["mode"] == "exec" and r["ok"], r)

# 10. Import persistence — use os without re-import
r = socket_exec("os.path.sep")
check("import persistence", r["mode"] == "eval" and r["ok"] and r["result"], r)

# 11. Multiline function + call
r = socket_exec("def double(n): return n * 2\ndouble(21)")
check("multiline function", r["ok"] and r["mode"] == "exec", r)

# 12. Function persistence — call again
r = socket_exec("double(100)")
check("function persistence", r["result"] == "200" and r["ok"], r)

# 13. List comprehension (expression)
r = socket_exec("[x**2 for x in range(5)]")
check("list comprehension", r["result"] == "[0, 1, 4, 9, 16]" and r["ok"], r)

# 14. Dict expression
r = socket_exec("{'a': 1, 'b': 2}")
check("dict expression", r["ok"] and "a" in (r["result"] or ""), r)

# 15. Empty code
r = socket_exec("")
check("empty code", r["ok"], r)

# 16. Lambda
r = socket_exec("(lambda x: x + 1)(99)")
check("lambda expression", r["result"] == "100" and r["ok"], r)

# 17. f-string
r = socket_exec('f"pi is {3.14159:.2f}"')
check("f-string", r["result"] == "'pi is 3.14'" and r["ok"], r)

# 18. Boolean ops
r = socket_exec("True and False or True")
check("boolean ops", r["result"] == "True" and r["ok"], r)

# 19. None literal
r = socket_exec("None")
check("None literal", r["result"] == "None" and r["ok"], r)

# 20. Tuple
r = socket_exec("(1, 2, 3)")
check("tuple", r["result"] == "(1, 2, 3)" and r["ok"], r)

# 21. Class definition + instantiation
r = socket_exec("class C:\n  def __init__(self):\n    self.v = 10\nc = C()\nc.v")
check("class + instance", r["ok"] and r["mode"] == "exec", r)

# 22. Arithmetic expression
r = socket_exec("(2 + 3) * 4")
check("arithmetic", r["result"] == "20" and r["ok"], r)

# 23. Complex number
r = socket_exec("complex(1, 2)")
check("complex number", r["result"] == "(1+2j)" and r["ok"], r)

# 24. Set expression
r = socket_exec("{1, 2, 3}")
check("set expression", r["ok"] and "1" in (r["result"] or ""), r)

# ── Jupyter Protocol Client Tests ─────────────────────────────────────────

print("\n=== Jupyter Protocol Client Tests ===\n")

with open(CONN_PATH) as f:
    conn = json.load(f)

client = KernelClient(conn)

# 25. Execute via Jupyter protocol
reply, outputs = client.execute("jp_test = 42; jp_test")
check("jupyter execute", reply["content"]["status"] == "ok", reply["content"])

# 26. Kernel info
info = client.kernel_info()
check("kernel info", info["content"]["implementation"] == "minimal_kernel", info["content"])

# 27. State: Jupyter vars visible via socket
r = socket_exec("jp_test")
check("jupyter state in socket", r["result"] == "42" and r["ok"], r)

# 28. State: socket vars visible via Jupyter
reply, outputs = client.execute("foo")
check("socket state in jupyter", reply["content"]["status"] == "ok", reply["content"])

# 29. IOPub execute_result present
has_result = any(m["header"]["msg_type"] == "execute_result" for m in outputs)
check("iopub execute_result", has_result)

# 30. Heartbeat
client.start_heartbeat()
time.sleep(1.5)
check("heartbeat alive", client.is_alive())
client.stop_heartbeat()

client.close()

# ── CLI Lifecycle Tests ───────────────────────────────────────────────────

print("\n=== CLI Lifecycle Tests ===\n")

# 31. List shows our kernel
output = subprocess.check_output(
    [sys.executable, os.path.join(REPO, "jupyter_repl_cli.py"), "list"],
    text=True
)
check("cli list shows kernel", KERNEL_NAME in output, output.strip())

# 32. Connect returns JSON
output = subprocess.check_output(
    [sys.executable, os.path.join(REPO, "jupyter_repl_cli.py"), "connect", KERNEL_NAME],
    text=True
)
conn_data = json.loads(output)
check("cli connect returns socket_path", "socket_path" in conn_data, list(conn_data.keys()))

# 33. Duplicate create fails
result = subprocess.run(
    [sys.executable, os.path.join(REPO, "jupyter_repl_cli.py"), "create", KERNEL_NAME],
    capture_output=True, text=True
)
check("duplicate create fails", result.returncode != 0 and "already running" in result.stderr, result.stderr)

# 34. Delete
cmd_delete(KERNEL_NAME)
time.sleep(0.5)
check("delete removes kernel", not os.path.exists(CONN_PATH))

# 35. List shows empty after delete
output = subprocess.check_output(
    [sys.executable, os.path.join(REPO, "jupyter_repl_cli.py"), "list"],
    text=True
)
check("list empty after delete", KERNEL_NAME not in output, output.strip())

# 36. Connect to deleted kernel fails
result = subprocess.run(
    [sys.executable, os.path.join(REPO, "jupyter_repl_cli.py"), "connect", KERNEL_NAME],
    capture_output=True, text=True
)
check("connect to deleted fails", result.returncode != 0 and "not found" in result.stderr, result.stderr)

# 37. Stale PID cleanup — create again should succeed
cmd_create(KERNEL_NAME)
check("recreate after delete succeeds", os.path.exists(CONN_PATH))

# Final cleanup
cmd_delete(KERNEL_NAME)

# ── Summary ───────────────────────────────────────────────────────────────

print(f"\n{'='*40}")
print(f"Results: {passed}/{total} passed")
if passed == total:
    print("All tests passed!")
    sys.exit(0)
else:
    print(f"{total - passed} test(s) failed.")
    sys.exit(1)