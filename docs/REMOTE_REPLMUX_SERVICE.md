The shortest path is much smaller than building a public network broker.

## Best immediate solution: MCP over SSH

The MCP server already uses stdio, so Claude Code can start it remotely through SSH:

```sh
claude mcp add --scope user replmux-linux -- \
  ssh -T user@linux-host replmux mcp
```

Architecture:

```text
Local Claude Code
  └── SSH stdio
       └── remote replmux mcp
            └── remote replmux broker/kernel
```

All Jupyter ZeroMQ ports remain on remote loopback. SSH provides authentication, encryption, and transport. No Replmux code changes should be necessary.

Ensure `replmux` is available to noninteractive SSH sessions:

```sh
ssh user@linux-host 'command -v replmux && replmux --version'
```

**Lift:** hours for documentation and end-to-end verification.

## Existing `replmux serve`

Today it provides:

- Unix socket with `0600` permissions
- JSON request/response protocol
- 1 MB request limit
- 30-second I/O timeout
- Thread per connection
- Local broker fallback
- All lifecycle and execution operations

Its security boundary is currently “processes running as this Unix user.”

## Option 2: first-class SSH transport

Add something like:

```sh
replmux --transport ssh \
  --remote user@host \
  kernel exec analysis 'x + 1'
```

Internally, send the existing serialized `KernelRequest` through:

```sh
ssh -T host replmux broker-request
```

This reuses:

- Existing wire types
- SSH authentication and host verification
- Remote kernel manager
- Existing operation dispatch

Required work:

- New `ssh` transport mode
- Remote endpoint configuration
- Quoting-free stdin framing
- Connection and command timeouts
- Clear distinction between local and remote paths
- SSH error classification
- Integration tests and documentation

**Lift:** approximately 2–4 focused days.

## Option 3: native remote HTTPS/MCP server

Expose either the broker protocol or Streamable HTTP MCP over TLS:

```text
https://host/replmux/mcp
```

Required production work:

- HTTP server and request framing
- TLS termination
- Token or mTLS authentication
- Authorization by kernel and operation
- Kernel ownership/namespaces
- Rate limits and concurrency limits
- Request IDs, audit logs, and replay protection
- Cancellation and longer execution handling
- Secret management
- Disable or redact `kernel connect`, since it exposes connection keys
- Daemon supervision and health endpoints
- Remote client configuration
- End-to-end security tests

**Lift:**
- Single-user prototype behind Tailscale: roughly 3–5 days
- Production single-tenant HTTPS service: 1–2 weeks
- Multi-user provider: 3–6 weeks

## Option 4: complete remote runtime provider

If “remote serve” includes provisioning Linux machines, selecting images, persistence, quotas, snapshots, and cleanup, it becomes a control plane:

```text
Provider API
├── provision workspace
├── install/select kernel image
├── start replmux
├── authenticate participants
├── persist runtime metadata
├── collect logs/artifacts
└── stop or hibernate workspace
```

**Lift:** roughly 1–2 months for a production first version.

## Recommendation

Start with **MCP over SSH**. It already provides the correct security and topology:

- Agent stays local
- Replmux and kernels are co-located remotely
- No raw Jupyter ports cross the network
- Linux-only kernels become available immediately
- No custom authentication system

After validating demand, implement first-class SSH transport. Build native HTTPS only if users need browser clients, shared tenancy, or providers where SSH is unavailable.
