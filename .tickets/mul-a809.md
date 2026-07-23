---
id: mul-a809
status: in_progress
deps: []
links: []
created: 2026-07-23T04:59:16Z
type: feature
priority: 1
assignee: memgrafter
---
# Support arbitrary Jupyter kernels

## Notes

**2026-07-23T04:59:20Z**

Scope: kernelspec discovery and argv/environment expansion, caller-provided connection files, launch and attach flows, removal of custom socket_path requirements, and compatibility tests for minimal kernel, ipykernel, Sage, and at least one non-Python kernel.

**2026-07-23T05:15:17Z**

Implemented kernelspec discovery/path loading, argv/environment expansion, caller-created standard connection documents, kernelspec process launch with heartbeat readiness, external connection attachment, and CLI create --kernelspec / attach commands. Minimal worker now accepts conventional caller-supplied ports and signing key. Added kernelspec launch + no-socket Jupyter attachment integration test. Build/tests and external ipykernel/Sage/non-Python compatibility remain pending.

**2026-07-23T05:23:18Z**

Clean release build and automated suite now pass, including conventional kernelspec launch and attachment after removing socket_path. Ticket remains in progress only for real ipykernel, Sage, and non-Python kernel compatibility runs.
