---
id: rep-spey
status: open
deps: []
links: []
created: 2026-07-23T19:01:39Z
type: epic
priority: 1
assignee: memgrafter
---
# Design remote environment provisioning for persistent kernels


Current kernel installation is entirely local: micromamba prefixes, system kernelspecs, hardcoded search paths. No support for SSH, Docker, VMs, or any remote execution target.

Scope:
- Define an `Environment` resource in the service (beyond the current `environment` string on Runtime)
- Support provisioning via: local micromamba, Docker images, SSH hosts, microVMs (Firecracker)
- Track environment lifecycle: provisioned, ready, running, degraded, destroyed
- Enable snapshot/restore at the VM level (moves snapshot responsibility out of the kernel process)

Related: ~/code/firecracker_macos/ as a snapshottable microVM candidate. MicroVM snapshots are a natural product differentiator but introduce a "VM package distribution" problem.

Sub-tasks to create after initial design:
- Local environment manager (unify micromamba + pip + conda)
- Docker environment backend
- SSH environment backend
- MicroVM environment backend (Firecracker)
- Environment health checks and auto-recovery

