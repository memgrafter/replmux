# Blocked kernel deployment workup

This report covers every kernel rejected or deferred during the macOS arm64
compatibility work. The observed machine was Apple Silicon running macOS; the
passing baseline is the fourteen entries in
[`tests/jupyter-kernels/kernels.toml`](../tests/jupyter-kernels/kernels.toml).

## Executive recommendation

| Kernel | Current problem | Best target | Projection |
|---|---|---|---|
| Maxima-Jupyter | conda Maxima uses unsupported ECL; supported route needs SBCL and a source build | Persistent x86_64 Linux VM or prebuilt Linux image | **High** |
| GAP JupyterKernel | Missing compiled GAP package dependencies | Persistent x86_64 Linux VM or prebuilt Linux image | **High** |
| Octave / xeus-octave | Native SIGSEGV on macOS arm64 | x86_64 Linux VM/container | **High** |
| SoS | Kernel process exits before opening channels | Linux image pinned to Python 3.11/3.12 with all child kernels co-located | **Medium** |
| Ark | Kernel cannot find environment-local R | Fix the kernelspec locally; Linux is optional | **High without changing OS** |
| bash_kernel | Heartbeat opens but shell requests time out | Linux image with GNU Bash 5 and Python 3.11/3.12 | **Medium-high** |
| RunMat | No conda-forge kernel package on this host | Vendor-supported x86_64 Linux image, if its Jupyter integration is distributable | **Medium-low** |
| Scilab kernel | No usable conda-forge kernel package; maintenance uncertain | Linux or Windows VM after a kernel maintenance audit | **Low-medium** |
| MATLAB / MKernel | Runtime and license absent | User-controlled licensed Linux/Windows VM | **High if licensed** |
| Wolfram Language | Runtime and license absent | User-controlled licensed Linux VM | **High if licensed** |

For the open-source native failures, the default provider should be an
**x86_64 Linux image built once and reused**. Use a VM when compilation,
persistent package stores, native debugging, or license activation is required.
Use an ephemeral remote workspace only after its image already contains the
kernel and all dependencies.

## Provider architecture constraint

Replmux currently launches local kernelspecs or attaches a Jupyter connection
file. A connection file contains raw ZeroMQ addresses, ports, and an HMAC key;
it is not an internet-safe remote transport.

For a remote Linux provider, prefer this layout:

```text
remote workspace / VM
├── agent or MCP client
├── replmux
└── Jupyter kernels on loopback
```

Do not expose Jupyter ZeroMQ ports publicly. If the agent remains local, add a
provider transport or an authenticated tunnel that carries all Jupyter channels
and rewrites the connection information. Until that exists, running Claude Code
and `replmux mcp` inside the remote workspace is the least surprising model.

## Observed failures and deployment projections

### Maxima-Jupyter

**Observed**

- conda-forge supplied Maxima 5.49 with ECL on macOS arm64.
- Maxima-Jupyter explicitly says ECL does not work.
- No conda-forge Maxima-Jupyter kernel package was available.
- The supported path uses a threaded Lisp such as SBCL or Clozure CL, Quicklisp,
  ZeroMQ development headers, and a Maxima build compatible with that Lisp.

**Best provider**

A persistent Debian/Ubuntu x86_64 VM, or a versioned OCI image built on Linux.
The upstream project documents Debian dependencies and Docker workflows, making
Linux substantially less speculative than another macOS attempt.

**Expected route**

1. Install SBCL, ZeroMQ/CZMQ development packages, Python, and Jupyter.
2. Build or install Maxima against SBCL.
3. install Quicklisp and Maxima-Jupyter.
4. Run `jupyter_install_image()` to avoid reloading dependencies on every start.
5. Execute the Replmux lifecycle test against the generated `maxima` spec.

**Projection:** high confidence on a controlled Linux image; poor fit for a
locked-down serverless provider that cannot compile or persist Quicklisp state.

### GAP JupyterKernel

**Observed**

- `gap-defaults` installed, but conda-forge had no JupyterKernel package.
- Installing the Python portion from GitHub produced a kernelspec but did not
  satisfy GAP packages `io`, `json`, `uuid`, `ZeroMQInterface`, and `crypting`.
- `LoadPackage("JupyterKernel")` failed; the generated launcher then exited.
- Several missing GAP packages contain native components.

**Best provider**

A persistent x86_64 Linux VM or prebuilt container with a complete GAP package
tree. Upstream also recommends WSL for Windows users, which is effectively the
same Linux deployment path.

**Expected route**

Install the official GAP distribution and required build libraries, build the
required GAP packages once, install the JupyterKernel Python component, and bake
the resulting GAP root into an image. Do not repeat package compilation at
kernel startup.

**Projection:** high on Linux with a curated image; medium on WSL; low on a
minimal ephemeral platform without build tools.

### Octave, xeus-octave, and free MATLAB syntax

**Observed**

- xeus-octave 0.7.0 received SIGSEGV during startup.
- octave_kernel could establish its Python-side heartbeat, but Octave 10.3.0
  crashed in interactive `octave-cli` execution.
- The crash reproduced outside Replmux, so it is not a Jupyter framing defect.
- RunMat and Scilab had no usable conda-forge Jupyter kernel package for this
  host.

**Best provider**

An x86_64 Linux VM/container using headless `octave-cli`. Prefer octave_kernel
first for maturity, then test xeus-octave as a native-protocol alternative.
Include gnuplot and fonts if plots are required.

**Projection:** high for Octave on mainstream Linux; medium for rich graphics in
headless remote environments; medium-low for RunMat until its packaging and
Jupyter installation are verified; low-medium for the currently listed Scilab
kernel because maintenance is uncertain. A Windows VM is a reasonable Scilab
fallback, but not the first Octave target.

### SoS

**Observed**

- conda-forge `sos-notebook` installed and emitted a `sos` kernelspec.
- The Python kernel process exited silently before opening Jupyter channels.
- Replmux never reached execution, so multi-kernel routing was not tested.

**Best provider**

A Linux image pinned to a conservative Python version, preferably 3.11 or 3.12,
with SoS and every delegated kernel installed in the same image. SoS gains
little from a provider where child kernels live on unrelated machines.

**Projection:** medium. The failure is likely dependency/version or package
initialization rather than an OS-level impossibility. A controlled VM is better
for diagnosis; a remote workspace is suitable after the image passes startup.

### Ark R kernel

**Observed**

- The Ark binary and user kernelspec installed.
- Startup aborted because Ark could not find `R` or `R_HOME`.
- Adding `r-base` to the micromamba environment did not help because the
  user-level kernelspec did not activate that environment or publish `R_HOME`.

**Best provider**

No different OS is required. Generate an environment-local Ark kernelspec with:

- an absolute Ark executable,
- `R_HOME=<prefix>/lib/R`, and
- the environment's `bin` directory prepended to `PATH`.

Linux may make R discovery more conventional, but it only masks the packaging
boundary. Fixing the kernelspec is the correct solution.

**Projection:** high locally or on Linux after kernelspec normalization.

### bash_kernel

**Observed**

- The Python kernel process opened heartbeat.
- Repeated `kernel_info_request` and shell requests timed out.
- No code reached Bash, so persistence was not tested.

**Best provider**

A small x86_64 Linux image with GNU Bash 5, Python 3.11/3.12, pexpect, and
bash_kernel. Linux is the natural semantic target for this kernel and avoids
macOS's old system Bash and platform-specific process behavior.

**Projection:** medium-high. If it still fails on Linux, capture kernel stderr
and compare its Jupyter protocol behavior before adding workarounds. Because a
persistent shell grants broad host access, run it only in an isolated workspace.

## Licensed kernels

### MATLAB / MKernel

Use a user-controlled Linux or Windows VM with an existing MATLAB installation,
the MATLAB Engine for Python, and an automation-compatible network or named-user
license. A generic public provider is inappropriate unless license terms and
secret storage are resolved. Linux is preferable for headless automation;
Windows is preferable where institutional MATLAB tooling is Windows-specific.

**Projection:** high technically, conditional on licensing. Octave remains the
preferred free target once moved to Linux.

### Wolfram Language for Jupyter

Use a user-controlled Linux VM with Wolfram Engine or Mathematica already
activated. Bake only the open-source Jupyter integration into reusable images;
do not redistribute proprietary runtime files or activation material. Confirm
whether automated and remote use is allowed by the applicable license.

**Projection:** high technically, conditional on licensing and activation.

## Recommended execution plan

1. Build one x86_64 Ubuntu image containing Octave, bash_kernel, and pinned
   Python 3.11; test those inexpensive kernels first.
2. Add SoS to that image only after its standalone kernel starts, then install
   the child kernels it should orchestrate.
3. Build a separate computer-algebra image for SBCL Maxima-Jupyter and GAP. Keep
   compilers and Quicklisp/GAP package stores out of the general agent image.
4. Fix Ark locally by generating an environment-aware kernelspec; do not migrate
   it merely to solve `R_HOME` discovery.
5. Keep MATLAB and Wolfram in separate licensed, user-owned VM profiles.
6. Add a secure remote provider transport before attempting to control kernels
   across hosts; until then, co-locate the agent, MCP server, and kernels.

## Acceptance test for every provider

A provider is ready only when all of these pass:

1. Create from its kernelspec within the startup deadline.
2. Return `kernel_info_reply` and heartbeat.
3. Assign state in one execution and read or print it in another.
4. Return textual stdout or `text/plain`, not only browser-only rich output.
5. Report syntax/runtime errors without killing the kernel.
6. Interrupt bounded long-running code.
7. Delete the kernel and its connection artifacts.
8. Repeat from a fresh image without interactive installation or credentials in
   the kernelspec.
