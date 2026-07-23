---
name: multirepl
description: Keep Python variables, imports, and results alive across turns, or share one live Python workspace between agents. Use for repeated calculations and collaborative analysis without rebuilding state or launching Python for every call.
---

Use Multirepl as durable working memory for Python computation: create a named workspace once, then return to it from later turns or other agents.

## Why use it

- **Preserve computational state** — keep variables, imports, functions, and intermediate results across tool calls.
- **Share work between agents** — agents using the same kernel name see the same namespace immediately.
- **Avoid repeated setup** — import libraries and load data once instead of recreating them for every calculation.
- **Keep reasoning concise** — perform exact calculations in Python rather than carrying large intermediate values in conversation context.
- **Stay fast** — repeated `repl` calls use Unix sockets instead of launching a Python subprocess each time.

## When to use

- A task needs several related Python calculations.
- Later turns will reuse earlier values, imports, or helper functions.
- Multiple agents need to collaborate on the same analytical state.
- You need exact numeric, text-processing, or data-transformation results.
- You want an inspectable scratch environment that survives between calls.

## When NOT to use

- A single `python -c` or `subprocess.run` call is simpler.
- The code is untrusted; Multirepl is not a sandbox.
- Concurrent agents may mutate the same values without coordinating.
- You require transactions, rollback, durable replay, or automatic crash restoration.
- You require sandboxing or automatic rollback. For magics, rich displays, or language-specific behavior, launch the appropriate standard Jupyter kernelspec instead of the minimal worker.

## Usage

### 1. Create a workspace

```text
repl-manage { action: "create", name: "analysis" }
```

Omit `name` to generate one automatically:

```text
repl-manage { action: "create" }
```

### 2. Build persistent state

```text
repl { name: "analysis", code: "from math import factorial\nvalues = [3, 5, 8, 13]" }
```

Statements persist state and normally return:

```text
(ok)
```

### 3. Reuse it later

```text
repl { name: "analysis", code: "sum(values)" }
```

```text
→ 29
```

### 4. Inspect and clean up

```text
repl-manage { action: "list" }
repl-manage { action: "connect", name: "analysis" }
repl-manage { action: "delete", name: "analysis" }
```

Delete temporary workspaces when they are no longer needed.

## Examples

### Multi-step calculation

```text
repl-manage { action: "create", name: "probability" }
repl { name: "probability", code: "from math import comb\ntotal = comb(52, 5)" }
repl { name: "probability", code: "royal_flushes = 4\nroyal_flushes / total" }
```

```text
→ 1.5390771693292702e-06
```

### Shared agent workspace

```text
Agent A: repl { name: "shared", code: "measurements = [10.2, 10.5, 9.9]" }
Agent B: repl { name: "shared", code: "sum(measurements) / len(measurements)" }
```

```text
→ 10.200000000000001
```

The namespace is shared mutable state. Coordinate writes and use separate kernel names when isolation matters.

## Output

- Expression: `→ <repr(value)>`
- Statements: `(ok)`
- Printed output: `stdout: ...`
- Standard error: `stderr: ...`
- Exception: `✗ <exception>`

A workspace remains available until its kernel exits or is deleted. If a kernel dies, its in-memory Python state is lost.

## Standalone CLI

Use the Rust CLI outside Pi:

```bash
multirepl kernel create analysis
multirepl kernel exec analysis 'x = 40'
multirepl kernel exec analysis 'x + 2'
multirepl kernel list
multirepl kernel delete analysis

# Launch or attach standard Jupyter kernels
multirepl kernel create notebook --kernelspec python3
multirepl kernel attach existing /path/to/connection.json
```

Standard Jupyter kernels use signed ZMQ execution; the custom Multirepl worker retains its faster direct socket. Use `multirepl serve` only when clients should share the optional local broker. Normal commands work without a running service.

## How it works

The Pi extension prefers the local Rust broker when one is running, falls back to a direct Multirepl worker socket, and uses the Rust Jupyter client for standard kernels without that custom socket. Lifecycle operations use the Rust CLI. The Python worker requires Python 3 with `pyzmq`; the Rust binary bundles its own libzmq.

For transport options, runtime metadata commands, release procedures, and architecture details, see [cli/README.md](cli/README.md) and [docs/](docs/).

**Cost:** one persistent Python worker per active workspace and coordination around shared mutable state. **Benefit:** fast, exact, reusable computation across turns and agents.
