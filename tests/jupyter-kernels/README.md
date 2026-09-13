# Kernel interoperability tests

[`kernels.toml`](kernels.toml) is the authoritative interoperability catalog:
fourteen executable `[[kernels]]` entries plus two `[[manual_validations]]`
(IElixir and ROOT). The installer reads only `[[kernels]]`. Its `required` tier
is expected in the standard test environment; `optional` kernels broaden
protocol interoperability coverage when their runtimes are available.

## Latest targeted validation

[Wiki-top Elixir and C++ results](../../docs/WIKI_TOP_KERNEL_VALIDATION.md)
cover IElixir, xeus-cpp, and ROOT. All passed cross-call state, text output,
error-recovery state reads, and create/delete/recreate checks, but none passed
every protocol criterion in those configurations.

| Kernel | Catalog placement | Known limitations |
|---|---|---|
| xeus-cpp 0.10.0 | Existing executable entry, legacy ID `cpp-xeus-cling` | Original control-message interruption failed; later SIGINT killed the kernel and lost state. |
| IElixir / Elixir 1.11.2, OTP 23 | Manual entry `elixir-ielixir` | Pinned Docker setup, poor error summaries, inspection timeout, and failed control-message interruption. |
| ROOT 6.38.00 / JupyROOT | Manual entry `cpp-root` | Docker plus Python dependencies; invalid code reports `ok:true`, cold startup timed out, and control-message interruption failed. |

IElixir and ROOT have not been retested with the newer signal-mode implementation.
The report includes setup workarounds, pinned images, and references to local,
gitignored command transcripts. The Docker test images were removed afterward.
Manual entries are discoverable validation records, not automatic installation
recipes or unconditional lifecycle passes.

## Finding kernels

There is no fixed exhaustive set: any program implementing the Jupyter messaging
protocol can be a kernel. Use these catalogs to find candidates:

- [Jupyter community kernel list](https://github.com/jupyter/jupyter/wiki/Jupyter-kernels)
- [Jupyter kernels documentation](https://docs.jupyter.org/en/latest/projects/kernels.html)
- [Jupyter kernelspec specification](https://jupyter-client.readthedocs.io/en/stable/kernels.html#kernel-specs)

Treat the community list as a discovery catalog, not a compatibility guarantee.
Before adding a kernel, verify that its upstream project is maintained, has
installation instructions, and implements the protocol operations exercised by
this project. See the [agent-oriented kernel catalog](../../docs/AGENT_KERNEL_CATALOG.md)
for the selection policy, licensed-runtime constraints, and recommended next
compatibility wave.

Discover installed kernel names with:

```sh
jupyter kernelspec list --json
```

A local kernelspec name may differ from the conventional `kernelspec` value in
the matrix, especially when several runtime versions are installed. Keep runtime
versions in CI environment definitions and lockfiles rather than duplicating
them in the matrix.

## Provisioning the matrix

The installer creates one isolated micromamba environment per kernel under
`tests/jupyter-kernels/.kernels/`. Install the complete matrix with:

```sh
./tests/jupyter-kernels/install-kernels.py
```

Install only selected entries by ID:

```sh
./tests/jupyter-kernels/install-kernels.py python-ipykernel r-irkernel
```

Inspect the commands without downloading packages or changing environments:

```sh
./tests/jupyter-kernels/install-kernels.py --dry-run
```

Package and channel declarations live beside each kernel in `kernels.toml`.
Environment-local kernelspec executables are normalized so Replmux can launch
them without activating the micromamba environment first. .NET Interactive is
installed from its official NuGet tool because the conda-forge package omits a
compatible PowerShell automation assembly.
Provisioning stops at the first package resolution or installation failure so a
partially available matrix is never reported as complete. Remove
`tests/jupyter-kernels/.kernels/` to reclaim the environments' disk space.
