After inspecting `~/code/replmux`, I’d treat **replay as the canonical history** and snapshots as acceleration artifacts.

## Important architectural observation

Currently:

- `Kernel.namespace` is one persistent dictionary.
- Jupyter requests execute on the kernel’s main loop.
- Direct Unix-socket requests spawn `_handle_socket_client` threads and execute Python inside those threads.

That explains the Sage problem: `signal.signal` only works on Python’s main thread.

A Rust rewrite should separate:

```text
Rust broker
├── Jupyter/ZMQ protocol
├── Unix socket / multiplayer ordering
├── event log and snapshot catalog
└── Python worker process
    └── all Python executes on its main thread
```

The Rust broker should enqueue every execution to one Python-owner thread/process rather than executing from network threads.

---

# Snapshot techniques

## 1. Replay/event sourcing

Your existing approach:

```text
environment + ordered cells → reconstructed kernel
```

### Advantages

- Portable
- Auditable
- Naturally supports multiplayer provenance
- Works across Python versions better than heap serialization
- Can rebuild state even if snapshots become unreadable

### Limitations

Replay is not deterministic when cells involve:

- Time or randomness
- Network calls
- Filesystem mutations
- Database writes
- Package upgrades
- Native libraries
- External processes
- Cells that partially mutate state before throwing

Record a globally ordered event log containing:

```json
{
  "sequence": 42,
  "agent": "agent-id",
  "code": "...",
  "status": "ok",
  "started_at": "...",
  "environment_hash": "...",
  "cwd": "...",
  "parent_snapshot": "..."
}
```

Replay should remain the fallback and provenance record.

---

## 2. Logical Python namespace snapshots

Serialize selected objects from `Kernel.namespace`.

```python
state = {
    name: value
    for name, value in namespace.items()
    if name not in EXCLUDED_NAMES
}
```

Possible serializers:

- `pickle` — standard, limited dynamic-code support
- `cloudpickle` — functions, classes, closures
- `dill` — broad session serialization
- Sage `.sobj` — strong for Sage objects
- Arrow/Parquet/Zarr — large tabular or array data
- PEP 574 out-of-band pickle buffers — avoids copying large arrays

### Recommended format

Use a manifest plus content-addressed blobs:

```json
{
  "execution_sequence": 42,
  "python_version": "3.12.13",
  "environment_hash": "...",
  "objects": {
    "model": {
      "serializer": "cloudpickle",
      "blob": "sha256:..."
    },
    "table": {
      "serializer": "parquet",
      "blob": "sha256:..."
    }
  },
  "excluded": {
    "socket_client": "unsupported: socket",
    "worker": "unsupported: thread"
  }
}
```

### Objects that generally cannot be restored safely

- Threads
- Locks
- Sockets
- Open transactions
- Generators mid-execution
- Subprocesses
- Native pointers
- GPU contexts
- Memory maps
- Event loops

Use a serializer registry so libraries can provide custom handlers:

```text
numpy.ndarray → pickle protocol 5 / NPY
pandas.DataFrame → Parquet
torch.Tensor → safetensors
Sage object → .sobj
fallback → cloudpickle
```

Logical snapshots should be considered trusted executable content: never load untrusted pickle-like data.

---

## 3. Checkpoint plus replay tail

This is likely the best default:

```text
checkpoint at sequence 100
+ replay events 101–113
= restored sequence 113
```

Benefits:

- Faster than full replay
- More portable than process snapshots
- A failed object snapshot only loses the interval since the previous checkpoint
- Supports compaction of long histories

I would make snapshots immutable and branchable:

```text
snapshot-100
├── branch-a: events 101–120
└── branch-b: events 101–108
```

Restore should create a new kernel by default rather than mutate an existing one.

---

## 4. `fork()` copy-on-write snapshots

On Linux, a quiescent Python worker can be forked almost instantly.

```text
Rust broker
    │
    └── Python worker
          ├── active branch
          └── parked fork snapshot
```

### Advantages

- Near-instant snapshot
- Exact heap state
- Copy-on-write memory
- Excellent for branching speculative agents

### Limitations

- Not durable across reboot
- Linux-centric
- Forking a multithreaded Python process is dangerous
- ZMQ, OpenMP, CUDA and native libraries may not be fork-safe
- Parked snapshots consume process table and eventual memory

Your current kernel should **not** be forked safely: it already has heartbeat, socket-client and ZMQ activity across threads.

A broker/worker split makes this viable because the worker can be:

- Single-threaded at snapshot barriers
- Free of network sockets except one reconnectable IPC endpoint
- Forked only while quiescent

A useful pattern is:

1. Pause execution queue.
2. Wait for current cell to finish.
3. Flush output.
4. Fork worker.
5. Park one branch.
6. Reconnect the active branch to the Rust broker.
7. Resume queue.

---

## 5. Full process checkpointing

### CRIU

On Linux, CRIU can checkpoint:

- Address space
- Process tree
- File descriptors
- Signals
- Some socket state

But it is sensitive to:

- Kernel version
- Network connections
- ZMQ state
- Containers/namespaces
- Native drivers

Separating the broker from the Python worker again helps: checkpoint only the worker and reconstruct its broker IPC connection after restore.

CRIU is unavailable on macOS.

### DMTCP

DMTCP is another userspace process-checkpoint option. It can handle distributed processes through plugins, but adds operational complexity and is still platform-sensitive.

### VM/microVM snapshots

Firecracker/QEMU snapshots capture memory plus disks with the highest fidelity. They are suitable for untrusted or extremely stateful kernels but are much heavier than logical snapshots.

---

## 6. Filesystem/environment snapshots

Kernel state is not just memory. Pair every snapshot with:

- Python executable identity
- Exact package lock
- `sys.path`
- Working directory
- Selected environment variables
- Filesystem layer ID
- Git commit and dirty diff
- RNG states

Possible filesystem techniques:

- OverlayFS layers
- Btrfs/ZFS snapshots
- APFS volume snapshots
- OCI image + writable layer
- Content-addressed artifact directory

For the Sage kernel, for example:

```text
Sage 10.9 package environment
+ namespace snapshot
+ execution-log offset
+ project filesystem snapshot
```

would form a reproducible restore point.

---

# Consistency for multiplayer snapshots

A snapshot must correspond to an exact global sequence number.

Current `RLock` serialization helps, but the Rust design should formalize this with an execution queue:

```text
request accepted
→ assigned sequence
→ executed by Python main thread
→ output committed
→ sequence marked durable
```

Snapshot protocol:

```text
freeze new executions
wait for active execution
flush stdout/stderr/display events
record final sequence number
capture state
unfreeze queue
```

Use optimistic checks:

```json
{
  "operation": "snapshot",
  "expected_sequence": 42
}
```

If the kernel has advanced to 43, reject or snapshot at 43 explicitly.

---

# Recommended architecture

## Rust broker

Owns:

- Jupyter protocol
- Unix socket protocol
- Authentication and multiplayer identity
- Global execution ordering
- Append-only event log
- Snapshot metadata/catalog
- Blob storage
- Environment fingerprints
- Worker lifecycle

## Python worker

Owns:

- CPython interpreter
- Namespace
- Main-thread execution
- Logical serialization plugins
- Library-specific restore hooks

Communication can be a framed Unix socket or pipe:

```json
{"op":"execute","sequence":42,"code":"..."}
{"op":"snapshot","id":"...","sequence":42}
{"op":"restore","snapshot":"..."}
```

Do not embed snapshot operations into standard Jupyter semantics. Keep them as replmux lifecycle extensions while retaining Jupyter compatibility.

---

# Suggested snapshot hierarchy

Implement these in order:

1. **Execution WAL and replay** — canonical truth
2. **Logical namespace checkpoint + replay tail** — portable default
3. **Content-addressed external buffers** — large-object efficiency
4. **Linux fork snapshots** — fast local branching
5. **Optional CRIU/microVM provider** — maximal fidelity

Define a provider interface in Rust:

```rust
trait SnapshotProvider {
    fn prepare(&self, kernel: KernelId) -> Result<Barrier>;
    fn capture(&self, barrier: Barrier) -> Result<Snapshot>;
    fn restore(&self, snapshot: SnapshotId) -> Result<Worker>;
    fn validate(&self, worker: WorkerId) -> Result<Validation>;
}
```

This lets logical, fork, CRIU and VM snapshots coexist.

## Guiding principle

**The event log is the durable truth; snapshots are disposable caches.**

That gives replmux fast restoration and branching without making correctness depend on successfully serializing every possible Python object.
