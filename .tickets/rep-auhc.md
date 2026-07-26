---
id: rep-auhc
status: open
deps: []
links: []
created: 2026-07-23T19:01:39Z
type: task
priority: 1
assignee: memgrafter
---
# Register python_minimal_kernel as an installable kernelspec


The minimal Python kernel ships as a stray `cli/assets/python_minimal_kernel.py` bundled in the release tarball. It is discovered by walking CWD and binary ancestors, not through any registration mechanism.

Goals:
- Register it as a proper Jupyter kernelspec (kernel.json + argv)
- Make it discoverable via `jupyter kernelspec list` or the existing `kernelspec::load()` path
- Support installation alongside standard kernelspecs
- Remove the ad-hoc search path (CWD → binary ancestors → bare filename)

Consider: bake a kernelspec directory into the release archive, or emit one at install time.

