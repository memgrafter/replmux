---
id: rep-ewrv
status: closed
deps: []
links: []
created: 2026-07-23T06:43:28Z
type: bug
priority: 1
assignee: memgrafter
---
# Repair relocated R kernel environment

## Notes

**2026-07-23T06:45:52Z**

Reinstalled r-irkernel at its current prefix, normalized its kernelspec executable to the environment R binary, and verified persistent state through replmux: answer <- 42 followed by cat(answer + 1) emitted 43. Updated installer to normalize environment-local bare kernelspec executables after provisioning.
