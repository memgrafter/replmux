---
id: rep-k8vb
status: closed
deps: []
links: []
created: 2026-07-23T08:02:33Z
type: feature
priority: 1
assignee: memgrafter
---
# Add overlapping Jupyter kernel coverage

## Notes

**2026-07-23T08:13:15Z**

Installed seven candidates while monitoring disk. Lifecycle passed for xeus-lua (43), xeus-r (43), xeus-python (43), and xeus-sql (SQLite query 43); added these four. SoS exited before channels, Ark could not discover environment R, and bash_kernel heartbeat never answered shell requests; removed failed environments and stale Ark user spec. Octave/xeus-octave was already tested as the free MATLAB alternative and crashes on this macOS arm64 host; RunMat and Scilab lack conda-forge kernel packages here. Cleaned caches; 4.7 GiB free.
