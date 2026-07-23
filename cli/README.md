# Replmux Rust CLI

The CLI manages runtime metadata through `service/` and provides local kernel lifecycle and persistent REPL execution.

## Runtime commands

```bash
replmux runtime create analysis
replmux runtime list
replmux runtime get rt_ID
replmux runtime update rt_ID --status running
replmux runtime delete rt_ID
```

## Kernel and REPL commands

```bash
replmux kernel create analysis
replmux kernel list
replmux kernel connect analysis
replmux kernel exec analysis 'x = 42'
replmux kernel exec analysis 'x'
replmux kernel info analysis
replmux kernel complete analysis 'value.bi'
replmux kernel inspect analysis 'value.bit_length'
replmux kernel is-complete analysis 'for item in values:'
replmux kernel heartbeat analysis
replmux kernel interrupt analysis
replmux kernel delete analysis
```

Launch any installed Jupyter kernelspec or attach an existing connection file:

```bash
replmux kernel create science --kernelspec python3
replmux kernel create algebra --kernelspec /path/to/sage/kernel.json
replmux kernel attach existing /path/to/kernel-connection.json
```

Kernelspec discovery follows `JUPYTER_PATH`, macOS and user data directories, then system Jupyter data directories. Standard kernels execute through their signed Jupyter ZMQ channels; Replmux's custom worker retains its direct Unix socket as a local optimization. The [Jupyter community list](https://github.com/jupyter/jupyter/wiki/Jupyter-kernels) is the broad discovery catalog; see [`docs/AGENT_KERNEL_CATALOG.md`](../docs/AGENT_KERNEL_CATALOG.md) for agent-oriented recommendations and licensing and isolation constraints.

For compatibility with `jupyter_repl_cli.py`, lifecycle commands are also accepted at the top level:

```bash
replmux create analysis
replmux list
replmux connect analysis
replmux exec analysis 'x + 1'
replmux delete analysis
```

Kernel configuration can be supplied globally or through environment variables:

```text
--kernel-dir       REPLMUX_KERNEL_DIR
--python           REPLMUX_PYTHON
--kernel-script    REPLMUX_KERNEL_SCRIPT
--broker-socket    REPLMUX_BROKER_SOCKET
```

The defaults are `~/.jupyter-repl/kernels`, `python3`, `minimal_kernel_clean.py`, and the short broker path `~/.replmux/b.sock`. Use `--json` for stable machine-readable lifecycle and execution responses.

### Claude Code MCP

The binary includes a stdio MCP server exposing the same two agent tools:
`repl` executes code in persistent kernels, and `repl-manage` creates, lists,
connects to, and deletes kernels.

Register it for the current user:

```bash
claude mcp add --scope user replmux -- replmux mcp
claude mcp get replmux
```

Or commit this project-scoped `.mcp.json` in another repository:

```json
{
  "mcpServers": {
    "replmux": {
      "type": "stdio",
      "command": "replmux",
      "args": ["mcp"]
    }
  }
}
```

The `replmux` binary must be on the environment `PATH` inherited by Claude
Code. Existing `REPLMUX_KERNEL_DIR`, `REPLMUX_PYTHON`,
`REPLMUX_KERNEL_SCRIPT`, and `REPLMUX_BROKER_SOCKET` configuration applies to
the MCP server. Kernel execution is unsandboxed and has the user's permissions.

## Local and served modes

Kernel commands default to `--transport auto`. Each command attempts the broker Unix socket once. An active broker receives the real request; `ENOENT` or `ECONNREFUSED` immediately short-circuits to the same in-process service implementation. Permission, protocol, and other broker failures are returned rather than bypassed.

```bash
replmux kernel list                    # auto: socket or in-process
replmux --transport local kernel list  # require in-process handling
replmux --transport socket kernel list # require a running broker
replmux serve                          # explicit persistent broker
```

The broker socket is created with mode `0600`, stale sockets are replaced on startup, and requests use bounded 30-second I/O timeouts.

The default API URL is `http://127.0.0.1:8000`. Override it with either:

```bash
replmux --api-url http://server:8000 runtime list
REPLMUX_API_URL=http://server:8000 replmux runtime list
```

Use `--json` for machine-readable output:

```bash
replmux --json runtime list
```

## Release package

From the repository root:

```bash
./scripts/release.sh
```

The script cleans previous Rust build artifacts, runs the locked service and CLI test suite, builds the optimized binary, verifies that libzmq is statically bundled rather than dynamically linked, and creates a target-specific archive plus SHA-256 checksum under `dist/`. Override the destination with `REPLMUX_RELEASE_DIR`.

For a local packaging-only iteration, reuse the existing release binary without cleaning, building, or testing:

```bash
./scripts/release.sh --fast
```

Fast mode still verifies static ZeroMQ linkage and creates the archive and checksum. It fails if `cli/target/release/replmux` does not already exist; use the default mode for production releases.

The archive includes the CLI, `minimal_kernel_clean.py`, and this README. The Rust CLI uses its bundled libzmq for Jupyter control messages and does not require a system ZeroMQ installation. Python 3 with `pyzmq` is still required by the Python kernel worker.

GitHub Actions builds and tests four native release targets on version tags or manual dispatch:

```text
aarch64-apple-darwin
x86_64-apple-darwin
aarch64-unknown-linux-gnu
x86_64-unknown-linux-gnu
```

Each target is uploaded as a separate workflow artifact with its SHA-256 checksum. Musl targets are tracked separately because bundled libzmq also requires a compatible static C++ cross-toolchain.

## Dependency security

Run all checks with UTC start/finish timestamps and elapsed seconds:

```bash
./scripts/audit-rust-deps.sh
```

The script checks for `cargo-audit`, `cargo-deny`, and `cargo-vet`, installing only missing tools with `cargo install --locked`. Existing installations are reused. It initializes missing cargo-deny and cargo-vet policy stores, runs every security check even if an earlier check fails, then exits nonzero if any check failed.

Security policy is committed in `cli/deny.toml` and `cli/supply-chain/`. The initial cargo-vet exemptions establish a reproducible baseline; they remain review debt to replace with imported or project audits over time.

## Development

Format and inspect metadata without compiling:

```bash
cargo fmt --check
cargo metadata --no-deps
```

When builds are permitted, run:

```bash
cargo test
```
