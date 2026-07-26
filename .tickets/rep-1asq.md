---
id: rep-1asq
status: closed
deps: []
links: []
created: 2026-07-24T17:05:12Z
type: bug
priority: 1
assignee: memgrafter
---
# Update Pi extension kernel script filename

## Notes

**2026-07-24T17:06:15Z**

Root cause is the installed ~/.local/bin/replmux binary predating commit 31d2251, not the Pi extension. The binary still embeds minimal_kernel_clean.py; current source uses python_minimal_kernel.py. Rebuild/reinstall the binary and install cli/assets/python_minimal_kernel.py beside it (or set REPLMUX_KERNEL_SCRIPT). No extension code change needed.
