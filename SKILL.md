---
name: replmux
description: Keep computational state alive across turns or share one live Jupyter workspace between agents. Use Python by default, or launch language and domain kernels for repeated calculations and collaborative analysis.
---

Use Replmux as durable working memory for Python computation: create a named workspace once, then return to it from later turns or other agents.

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
- The code is untrusted; Replmux is not a sandbox.
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

## Choosing a standard kernel

Replmux can manage any compatible Jupyter kernelspec, not only Python. The
canonical discovery inventory is the [Jupyter community kernel
list](https://github.com/jupyter/jupyter/wiki/Jupyter-kernels); it is a catalog,
not a support guarantee.

For agent work, prioritize maintained kernels with persistent state,
deterministic text output, standard interrupt and inspection behavior,
noninteractive installation, and automation-compatible licensing. Recommended
capability additions are SageMath for broad computer algebra, LFortran for
modern Fortran, Maxima and GAP for specialized algebra, Octave for free
MATLAB-like computing, xeus-sqlite for local data work, and EvCxR for Rust.
Wolfram/Mathematica and MATLAB kernels require user-managed runtimes and valid
licenses. Hardware, Docker, database, and remote-service kernels require
explicit credentials and isolation; Replmux is not a sandbox.

See [docs/AGENT_KERNEL_CATALOG.md](docs/AGENT_KERNEL_CATALOG.md) for the curated
recommendations and [`tests/jupyter-kernels/kernels.toml`](tests/jupyter-kernels/kernels.toml)
for the machine-tested compatibility matrix.

## Standalone CLI

Use the Rust CLI outside Pi:

```bash
replmux kernel create analysis
replmux kernel exec analysis 'x = 40'
replmux kernel exec analysis 'x + 2'
replmux kernel list
replmux kernel delete analysis

# Launch or attach standard Jupyter kernels
replmux kernel create notebook --kernelspec python3
replmux kernel attach existing /path/to/connection.json
```

Standard Jupyter kernels use signed ZMQ execution; the custom Replmux worker retains its faster direct socket. Use `replmux serve` only when clients should share the optional local broker. Normal commands work without a running service.

## How it works

The Pi extension prefers the local Rust broker when one is running, falls back to a direct Replmux worker socket, and uses the Rust Jupyter client for standard kernels without that custom socket. Lifecycle operations use the Rust CLI. The Python worker requires Python 3 with `pyzmq`; the Rust binary bundles its own libzmq.

For transport options, runtime metadata commands, release procedures, and architecture details, see [cli/README.md](cli/README.md) and [docs/](docs/).

**Cost:** one persistent Python worker per active workspace and coordination around shared mutable state. **Benefit:** fast, exact, reusable computation across turns and agents.
