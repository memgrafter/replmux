---
id: rep-sk5z
status: open
deps: [rep-x3ek]
links: [rep-r9hy, rep-ao5x]
created: 2026-07-23T19:01:39Z
type: task
priority: 2
assignee: memgrafter
---
# Add wait tool for code-mode execution in MCP and Pi extension


Lean into "code mode" by adding a `wait` tool that lets agents poll for long-running execution results without blocking the tool call.

Changes needed:
- `cli/src/mcp.rs`: add `wait` tool (poll execution status by request ID)
- `pi/extension/replTool.ts`: add corresponding `wait` tool in Pi's MCP client

Design: `exec` returns immediately with a request ID. `wait` polls that ID for completion, result, or timeout. Enables fire-and-forget execution patterns in code mode.

Acceptance:
- `exec` returns {request_id, status: "pending"} for slow executions
- `wait {request_id}` returns result when ready, or "still running"
- Timeout configurable, default 30s

