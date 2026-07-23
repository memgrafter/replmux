---
id: rep-7vcx
status: closed
deps: []
links: []
created: 2026-07-23T07:08:35Z
type: feature
priority: 1
assignee: memgrafter
---
# Add next-wave agent kernel matrix

## Notes

**2026-07-23T07:15:15Z**

1/7 SageMath: installed conda-forge sage 10.9; native sagemath kernelspec lifecycle passed after warm-up. Persistent polynomial state produced (x - 1) * (x + 1) * (x^2 - 3).

**2026-07-23T07:17:41Z**

2/7 LFortran: installed 0.64.0; native fortran kernelspec lifecycle and persistent state passed (integer answer=42, later print emitted 43).

**2026-07-23T07:21:47Z**

3/7 Maxima-Jupyter blocked on macOS arm64: conda-forge has Maxima only with ECL, while Maxima-Jupyter explicitly says ECL does not work; no conda kernel package exists, and the supported SBCL path requires building Maxima from source (prohibited by project workflow). No matrix entry committed.

**2026-07-23T07:28:55Z**

4/7 GAP blocked: conda-forge has GAP but no JupyterKernel package. Upstream JupyterKernel requires io, json, uuid, ZeroMQInterface, and crypting GAP packages; several require source builds, prohibited by project workflow. A pure pip install was tested and rejected because LoadPackage(JupyterKernel) failed. Failed environment removed; no matrix entry committed.

**2026-07-23T07:42:58Z**

5/7 Octave blocked on this macOS arm64 host: xeus-octave 0.7.0 SIGSEGVs at startup; octave_kernel with Octave 10.3.0 also fails because octave-cli segfaults in interactive mode. Tested both kernels and a direct octave-cli session. Failed environments removed; no matrix entry committed. Also fixed installer kernelspec normalization to use atomic replacement instead of mutating hard-linked conda package-cache files.

**2026-07-23T07:46:11Z**

6/7 xeus-sqlite: installed 0.10.0; lifecycle passed. Created a database, table, and row across separate calls; SELECT returned 43 as text/plain and HTML execute_result.

**2026-07-23T07:51:06Z**

7/7 EvCxR: installed 0.21.1; lifecycle and persistent Rust state passed (answer=42, later println emitted 43). Final matrix additions: SageMath, LFortran, xeus-sqlite, EvCxR. Maxima-Jupyter, GAP, and Octave remain documented host/provisioning blockers. Cleaned micromamba caches after installs; 6.0 GiB remains free.
