# Agent-oriented Jupyter kernel catalog

The canonical community inventory is the [Jupyter kernels
list](https://github.com/jupyter/jupyter/wiki/Jupyter-kernels). It includes
maintained, experimental, deprecated, hardware-bound, service-backed, and
commercial kernels, so inclusion there does not imply that a kernel is suitable
for unattended agents or supported by Replmux.

Replmux's executable compatibility matrix remains
[`tests/jupyter-kernels/kernels.toml`](../tests/jupyter-kernels/kernels.toml).
This document is the candidate backlog and selection policy.

## What makes a kernel useful to agents

Prefer kernels that provide:

1. Persistent definitions across separate execution requests.
2. Deterministic textual output in addition to rich notebook displays.
3. Standard `kernel_info`, execute, interrupt, completion, and inspection
   behavior.
4. Active maintenance and a reproducible, noninteractive installer.
5. A useful capability boundary that is difficult to replace with ordinary
   Python libraries or command-line tools.
6. Licensing that permits automated local or CI use.

Treat arbitrary kernel execution as arbitrary local code execution. Replmux is
not a sandbox. Hardware, Docker, database, and remote-service kernels require
additional credential and isolation policies before agent use.

## Recommended expansion order

### Priority 1: differentiated computational systems

| Kernel | Agent value | Recommendation |
|---|---|---|
| **SageMath** | Unified symbolic algebra, exact arithmetic, number theory, combinatorics, algebraic geometry, and access to systems such as GAP, Maxima, PARI/GP, and Singular. | **Add first.** It is open source and has the broadest capability gain, despite its large environment. Test its native Sage kernelspec rather than treating it as ordinary Python. |
| **LFortran** | Interactive modern Fortran with compiler diagnostics and numerical-code prototyping. | **Add.** Prefer it over the older Coarray-Fortran kernel for general use; keep Coarray-Fortran for explicit MPI/coarray testing. |
| **Maxima-Jupyter** | Focused open-source computer algebra with transparent symbolic transformations. | **Add after Sage.** Smaller and useful as an independent symbolic cross-check. |
| **GAP kernel** | Computational discrete algebra and group theory. | **Add for algebra agents.** Sage exposes GAP, but a native kernel gives direct syntax and documentation behavior. |
| **Octave or xeus-octave** | Mature, free MATLAB-like numerical computing. | **Add.** Prefer a maintained native kernel; use it as the default MATLAB-syntax target when proprietary MATLAB is unavailable. |
| **Scilab** | Free matrix-oriented numerical computing with a distinct scientific ecosystem. | **Evaluate after Octave.** Add only if its kernel is maintained and automatable on supported hosts. |
| **xeus-sqlite / xeus-sql** | Stateful SQL, schema exploration, and tabular output without embedding database control logic in Python. | **Add xeus-sqlite first.** It is local, deterministic, and credential-free; gate network database kernels separately. |

### Priority 2: strong general-purpose additions

| Kernel | Agent value | Recommendation |
|---|---|---|
| **EvCxR (Rust)** | Stateful Rust experimentation with compiler feedback. | Add for systems and performance work. |
| **GoNB** | Maintained Go kernel with current Go support. | Prefer over deprecated IGo, gopherlab, and older alternatives. |
| **kotlin-jupyter** | JVM libraries, Kotlin scripting, and rich integrations. | Add when JVM workloads matter. |
| **DFLib JJava or Rapaio** | Active JShell-based Java environments. | Prefer these maintained implementations over old IJava deployments. |
| **xeus-lua** | Lightweight embeddable-language experimentation using the xeus protocol. | Useful low-cost protocol coverage. |
| **SoS** | Coordinates multiple kernels in one workflow. | Investigate, but do not make foundational: it overlaps Replmux orchestration and adds another state-routing layer. |
| **Dot/Graphviz** | Fast graph and architecture rendering from generated DOT. | Useful as a narrow rendering kernel, though invoking Graphviz directly may be simpler. |

### Licensed or externally constrained systems

| Kernel | Constraint | Recommendation |
|---|---|---|
| **Wolfram Language for Jupyter** | Requires Wolfram Engine, Mathematica, or another appropriately licensed Wolfram installation. Wolfram Engine availability does not make every use case unrestricted or redistributable. | **High capability, conditional adoption.** Detect an existing licensed installation; never silently download, activate, or assume CI redistribution rights. Prefer the official Wolfram kernel over IWolfram. |
| **MATLAB kernel / MKernel** | Requires MATLAB and usually its Python engine plus a valid license. | Support by attaching to user-managed installations. Prefer **MKernel** for a new evaluation, and use Octave or RunMat where MATLAB compatibility is sufficient. |
| **RunMat** | Open-source MATLAB-syntax runtime, but compatibility differs from MATLAB. | Evaluate as a license-free accelerator; label it MATLAB-syntax, not MATLAB-compatible by assumption. |
| **Stata, SAS, IDL, kdb+/q** | Commercial runtime, license, and often site-specific configuration. | Attach-only by default. Do not include in an automatic public test matrix. |
| **Spark kernels / sparkmagic** | Requires Spark or Livy infrastructure and credentials. | Maintain as a separate integration profile, not a local kernel smoke test. |

### Assembly, hardware, and infrastructure kernels

Assembly kernels such as Emu86 and MIPS/SPIM are useful for teaching and ISA
experiments, but are lower priority for general agents. Their limited runtime
models, older dependencies, and architecture-specific behavior provide less
value than driving a maintained assembler, compiler, emulator, or debugger from
a systems-language kernel. Add one only with a concrete education or firmware
workflow.

MicroPython, Home Assistant, Dockerfile, SystemTap, Golem, Hive, database, and
similar kernels cross hardware, daemon, credential, or privilege boundaries.
They need explicit opt-in profiles, timeouts, and isolation. In particular, the
Dockerfile kernel should not be exposed to an untrusted agent merely because the
user belongs to the Docker group; Docker access is effectively host-root access.

## Proposed next compatibility wave

Start with a bounded matrix that maximizes distinct capabilities:

1. SageMath
2. LFortran
3. Maxima-Jupyter
4. GAP
5. Octave (or xeus-octave after a maintenance check)
6. xeus-sqlite
7. EvCxR

Evaluate Wolfram and MATLAB separately on licensed developer machines. Evaluate
Scilab and assembly kernels only after confirming current maintenance and a
repeatable arm64 macOS/Linux installation path.

For each candidate, record installation source, runtime version, kernelspec
name, disk estimate, license class, and whether execute results arrive as
`execute_result`, `display_data`, or `stream`. Replmux should test persistent
state with explicit textual output because several valid kernels render bare
expressions only as rich display data.
