---
id: mul-d57l
status: open
deps: []
links: []
created: 2026-07-20T05:21:28Z
type: bug
priority: 3
assignee: memgrafter
---
# Add status field to kernel_info_reply

kernel_info_reply content dict is missing status: ok per Jupyter protocol spec. The shell and control channel replies include status but kernel_info_reply does not. Most clients tolerate it but it is strictly non-compliant.
