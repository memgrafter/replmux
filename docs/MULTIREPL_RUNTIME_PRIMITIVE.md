The central REST resource should not be a **kernel**. A kernel is an ephemeral process. Make the durable aggregate a **Runtime**.

```text
Runtime
├── Branches
├── Executions
├── Events
├── Snapshots
├── Transactions
├── Participants
├── Approvals
└── Attachments
```

A runtime survives kernel crashes, hibernation, JupyterLab disconnects, and process replacement.

# 1. Runtime resource

```http
POST /v1/runtimes
```

```json
{
  "name": "jacobian-analysis",
  "language": "python",
  "environment": {
    "kind": "python",
    "executable": "/path/to/python",
    "digest": "sha256:..."
  },
  "snapshot_policy": {
    "interval_executions": 25,
    "mode": "logical"
  }
}
```

Response:

```json
{
  "id": "rt_01J...",
  "name": "jacobian-analysis",
  "status": "running",
  "default_branch_id": "br_01J...",
  "worker_generation": 1,
  "created_at": "...",
  "etag": "runtime:4"
}
```

States:

```text
provisioning
running
idle
hibernated
restoring
failed
deleted
```

Important distinction:

```text
Runtime = durable identity and history
Worker  = replaceable Python process
Branch  = one lineage of runtime state
```

# 2. Branch resource

Every execution occurs against a branch.

```json
{
  "id": "br_main",
  "runtime_id": "rt_01J...",
  "name": "main",
  "parent_snapshot_id": null,
  "head_execution_id": "ex_42",
  "head_snapshot_id": "snap_40",
  "revision": 42,
  "status": "ready",
  "worker_generation": 3
}
```

Create a branch:

```http
POST /v1/runtimes/{runtime_id}/branches
```

```json
{
  "name": "try-alternative-proof",
  "from": {
    "snapshot_id": "snap_40"
  }
}
```

Branches form a state DAG, while the runtime event log remains totally ordered.

# 3. Execution resource

```http
POST /v1/branches/{branch_id}/executions
Idempotency-Key: tool-call-abc
If-Match: "branch:42"
Prefer: wait=30
```

```json
{
  "code": "det_F = J.det()",
  "mode": "auto",
  "atomicity": "on_error",
  "provenance": {
    "client": "pi",
    "tool_call_id": "call_123",
    "notebook_path": "analysis.ipynb",
    "cell_id": "cell-abcd"
  }
}
```

The authenticated identity supplies the actor; do not trust a body field such as `"agent_id"`.

Response if it finishes during the wait:

```json
{
  "id": "ex_43",
  "runtime_sequence": 109,
  "branch_revision_before": 42,
  "branch_revision_after": 43,
  "actor": {
    "type": "agent",
    "id": "agent-7"
  },
  "status": "succeeded",
  "result": {
    "repr": "-2",
    "mime": {
      "text/plain": "-2"
    }
  },
  "stdout": "",
  "stderr": "",
  "snapshot_before": "snap_42",
  "snapshot_after": null
}
```

For a long execution:

```http
HTTP/1.1 202 Accepted
Location: /v1/executions/ex_43
```

## Execution states

```text
proposed
awaiting_approval
queued
running
succeeded
failed
cancelled
rolled_back
```

## Concurrency

Each branch has exactly one state-mutating execution at a time.

Use:

- `Idempotency-Key` to prevent duplicate tool calls
- `If-Match` to prevent execution against stale state
- `409 Conflict` when branch revision changed

Different branches may execute concurrently.

# 4. Canonical event log

The event log is append-only and is the durable truth.

```json
{
  "sequence": 109,
  "event_id": "evt_01J...",
  "runtime_id": "rt_01J...",
  "branch_id": "br_main",
  "type": "execution.succeeded",
  "subject_id": "ex_43",
  "actor": {
    "type": "agent",
    "id": "agent-7",
    "session_id": "agent-session-12"
  },
  "causation_id": "evt_108",
  "correlation_id": "tool-call-abc",
  "created_at": "...",
  "payload": {}
}
```

Consume it through SSE:

```http
GET /v1/runtimes/{runtime_id}/events?after=108
Accept: text/event-stream
```

Events might include:

```text
runtime.created
worker.started
participant.joined
execution.proposed
approval.requested
approval.granted
execution.started
execution.stdout
execution.displayed
execution.succeeded
snapshot.created
branch.created
transaction.committed
worker.replaced
```

Use one monotonic `runtime_sequence` across all branches. Branch causality is represented separately by revision and parent links.

# 5. Snapshot resource

```http
POST /v1/branches/{branch_id}/snapshots
If-Match: "branch:43"
```

```json
{
  "mode": "logical",
  "label": "before-elimination"
}
```

Response:

```json
{
  "id": "snap_43",
  "runtime_id": "rt_01J...",
  "branch_id": "br_main",
  "execution_id": "ex_43",
  "branch_revision": 43,
  "environment_digest": "sha256:...",
  "mode": "logical",
  "manifest_blob": "sha256:...",
  "replay_offset": 109,
  "status": "ready"
}
```

Snapshot modes can share one API:

```text
logical      serialized namespace
replay       environment + event offset
fork         parked COW worker
process      CRIU/process image
filesystem  environment/filesystem layer
```

Snapshots should be immutable.

# 6. Restore resource

Prefer creating a restore operation rather than mutating silently:

```http
POST /v1/branches/{branch_id}/restores
If-Match: "branch:43"
```

```json
{
  "snapshot_id": "snap_30",
  "strategy": "replace_worker"
}
```

Restore procedure:

1. Freeze branch queue.
2. Start a new worker generation.
3. Load snapshot.
4. Replay tail if needed.
5. Run validation probes.
6. Atomically switch the branch to the new worker.
7. Fence and stop the old worker.
8. Append `branch.restored`.

Every worker request carries its generation. Late responses from an old worker are rejected.

# 7. Transaction resource

Arbitrary Python cannot be rolled back by undoing dictionary assignments. Transactionality should be implemented through isolated runtime state.

```http
POST /v1/branches/{branch_id}/transactions
If-Match: "branch:43"
```

```json
{
  "isolation": "snapshot",
  "on_error": "rollback"
}
```

Response:

```json
{
  "id": "tx_01J...",
  "base_revision": 43,
  "status": "open",
  "working_branch_id": "br_tx_01J..."
}
```

Execute within it:

```http
POST /v1/transactions/{transaction_id}/executions
```

Commit:

```http
POST /v1/transactions/{transaction_id}/commit
If-Match: "branch:43"
```

Commit succeeds only if the original branch still has revision 43. Otherwise:

```http
409 Conflict
```

Rollback:

```http
DELETE /v1/transactions/{transaction_id}
```

Implementation options:

- Clone from a logical snapshot
- Fork a quiescent worker
- Restore checkpoint and replay
- Create an ephemeral branch

Only managed kernel and filesystem state is transactional. Network calls, external databases and emails cannot be rolled back automatically.

# 8. Approval resource

A policy may stop an execution before it enters the queue:

```json
{
  "id": "appr_01J...",
  "execution_id": "ex_44",
  "status": "pending",
  "policy": "external-network-access",
  "requested_by": {
    "type": "agent",
    "id": "agent-7"
  },
  "required": {
    "count": 1,
    "roles": ["owner", "approver"]
  },
  "expires_at": "..."
}
```

Decision:

```http
POST /v1/approvals/{approval_id}/decisions
```

```json
{
  "decision": "approve",
  "comment": "Network lookup is allowed."
}
```

The authenticated user becomes the decision actor.

States:

```text
pending
approved
rejected
expired
cancelled
```

All decisions enter the canonical event log.

# 9. Participant and coordination resources

Attach a headless agent:

```http
POST /v1/runtimes/{runtime_id}/participants
```

```json
{
  "client_type": "agent",
  "capabilities": [
    "execute",
    "observe",
    "request_approval"
  ]
}
```

Participant records use leases:

```json
{
  "id": "part_01J...",
  "actor_id": "agent-7",
  "status": "active",
  "lease_expires_at": "...",
  "last_seen_sequence": 108
}
```

For multi-step exclusive work:

```http
POST /v1/branches/{branch_id}/leases
```

```json
{
  "scope": "write",
  "ttl_seconds": 60,
  "reason": "three-step symbolic transformation"
}
```

Do not require a lease for ordinary executions—the queue already serializes them. Leases are for coordinating a multi-operation sequence.

# 10. Attachment resource

Notebooks and tools attach to the Runtime, not directly to a PID.

```http
POST /v1/branches/{branch_id}/attachments
```

```json
{
  "client_type": "jupyter",
  "notebook_path": "analysis.ipynb"
}
```

Response:

```json
{
  "id": "attach_01J...",
  "branch_id": "br_main",
  "worker_generation": 3,
  "protocol": "jupyter-zmq",
  "connection_token": "short-lived-token",
  "expires_at": "..."
}
```

Notebook metadata stores stable identifiers:

```json
{
  "multirepl": {
    "runtime_id": "rt_01J...",
    "branch_id": "br_main",
    "attachment_id": "attach_01J..."
  }
}
```

After JupyterLab exits, the Runtime remains. Reopening the notebook requests a fresh attachment.

# 11. Fast tool API

REST should be the semantic API, but local agents can use HTTP over a Unix socket:

```text
~/.multirepl/multirepl.sock
```

The same request model can be exposed through:

- HTTP/1.1 over Unix socket
- HTTP/2 locally or remotely
- Rust CLI
- Pi tool
- Jupyter Server extension

Avoid maintaining separate execution semantics for each transport.

A fast synchronous call:

```http
POST /v1/branches/br_main/executions
Prefer: wait=30
Idempotency-Key: pi-call-123
```

returns immediately when possible, while SSE streams longer output.

# 12. Storage model

For a prototype:

```text
SQLite in WAL mode
├── runtimes
├── branches
├── workers
├── actors
├── participants
├── executions
├── events
├── snapshots
├── transactions
├── approvals
├── leases
└── attachments

Content-addressed blob directory
├── namespace snapshots
├── large outputs
├── MIME bundles
├── replay artifacts
└── environment manifests
```

Important invariants:

1. Snapshots are immutable.
2. Events are append-only.
3. One active worker generation per branch.
4. One state-mutating execution at a time per branch.
5. Branch-head updates use compare-and-swap.
6. Every request is idempotent.
7. Every state transition and its event commit in one database transaction.
8. Old worker generations cannot publish accepted results.

# 13. Jupyter mapping

Every Jupyter `execute_request` becomes an Execution resource.

Every REST/tool execution emits standard Jupyter events:

```text
execute_input
status busy
stream/display/error
execute_result
status idle
```

Therefore:

- Notebook users see agent work.
- Agents see notebook executions.
- Both share provenance and ordering.
- Yjs owns notebook text.
- Multirepl owns runtime state.

# Recommended MVP

Start with only five resources:

```text
Runtime
Branch
Execution
Event
Snapshot
```

Then add:

```text
Transaction
Approval
Participant/Lease
Attachment
```

The key design decision is that **Runtime is the durable REST object, Branch is the state lineage, Execution is the mutation, and Event is the canonical record**.
