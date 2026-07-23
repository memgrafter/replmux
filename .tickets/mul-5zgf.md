---
id: mul-5zgf
status: open
deps: [mul-r5oy, mul-jcif, mul-yxmc, mul-hfdn, mul-c2z5]
links: []
created: 2026-07-23T03:37:52Z
type: epic
priority: 2
assignee: memgrafter
---
# Add REPL manager functionality to Rust CLI

## Current status

The immediate Python CLI and Pi tool parity milestone is complete:

- Rust `kernel create`, `list`, `connect`, `exec`, and `delete`
- Top-level compatibility aliases
- Persistent Unix-socket execution with stdout, stderr, results, and errors
- Statically bundled ZeroMQ for signed Jupyter shutdown
- Rust lifecycle integration tests
- Pi `repl-manage` migrated to the Rust binary while `repl` retains direct socket transport

The epic remains open because the durable broker-managed runtime design below is not implemented. Remaining child tickets:

- `mul-jcif` — broker-managed runtime lifecycle
- `mul-yxmc` — unified Jupyter execution protocol
- `mul-hfdn` — runtime branches and snapshots
- `mul-c2z5` — runtime events and diagnostics

The ideal `cli/` REPL manager should manage **durable runtimes through the broker**, not directly treat kernel PIDs as the primary object.

## Recommended command surface

```text
multirepl runtime create <name>
multirepl runtime list
multirepl runtime get <runtime>
multirepl runtime start <runtime>
multirepl runtime stop <runtime>
multirepl runtime restart <runtime>
multirepl runtime hibernate <runtime>
multirepl runtime delete <runtime>

multirepl runtime connect <runtime>
multirepl runtime exec <runtime> <code>
multirepl runtime status <runtime>
multirepl runtime logs <runtime>
```

### Lifecycle semantics

- `create`: create durable metadata, default branch, and optionally start a worker.
- `start`: provision or restore a worker.
- `stop`: gracefully stop the worker while retaining runtime history.
- `restart`: replace the worker and increment `worker_generation`.
- `hibernate`: snapshot state, then stop the worker.
- `delete`: tombstone the runtime and safely terminate its worker.
- `connect`: return attachment/connection details, not expose permanent raw secrets.
- `status`: report runtime, branch, worker generation, PID/health, queue depth, and latest sequence.

## Execution

```bash
multirepl runtime exec analysis 'x = 42'
multirepl runtime exec analysis --file script.py
multirepl runtime exec analysis --stdin
multirepl runtime exec analysis 'x' --json
multirepl runtime exec analysis 'slow_call()' --wait 30
```

It should support:

- Persistent namespace
- stdout, stderr, result, displays, and errors
- Timeouts and cancellation
- Execution IDs and global sequence numbers
- Idempotency keys
- Branch revision checks
- Actor/tool-call provenance
- Streaming long-running output

Every execution—CLI, Pi, or notebook—must enter the same ordered broker queue and emit standard Jupyter events.

## Branches and snapshots

```text
multirepl branch list <runtime>
multirepl branch create <runtime> <name> --from <snapshot>
multirepl branch switch <runtime> <branch>
multirepl snapshot create <runtime> --label <label>
multirepl snapshot list <runtime>
multirepl snapshot restore <runtime> <snapshot>
```

These are essential to the documented product direction, though they can follow basic lifecycle parity.

## Events and observability

```text
multirepl events <runtime> --follow
multirepl execution get <execution>
multirepl execution cancel <execution>
multirepl doctor
```

`doctor` should validate:

- Broker reachability
- Database and blob storage
- Worker process health
- Stale connection/PID files
- Environment availability
- Snapshot compatibility

## Immediate parity milestone

Before the complete broker exists, Rust should replace the current Python manager with:

```text
multirepl kernel create <name>
multirepl kernel list
multirepl kernel connect <name>
multirepl kernel delete <name>
multirepl kernel exec <name> <code>
```

That implementation needs:

- Connection and PID file handling
- Stale-state cleanup
- Startup readiness timeout
- Duplicate-name detection
- Unix-socket JSON execution
- Graceful shutdown followed by bounded TERM/KILL fallback
- Machine-readable `--json` output
- The existing lifecycle suite ported to Rust integration tests

Treat `kernel` as a low-level/debug surface. The durable user-facing API should remain `runtime`, because:

```text
Runtime = durable identity and history
Worker  = replaceable process
Branch  = state lineage
```

The existing `repl-manage` actions can then become compatibility aliases, while Pi eventually calls the same broker API as the Rust CLI.
