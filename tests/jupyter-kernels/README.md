# Kernel interoperability tests

[`kernels.toml`](kernels.toml) is the authoritative kernel test matrix. The
`required` tier is expected in the standard test environment; `optional`
kernels broaden protocol interoperability coverage when their runtimes are
available.

## Finding kernels

There is no fixed exhaustive set: any program implementing the Jupyter messaging
protocol can be a kernel. Use these catalogs to find candidates:

- [Jupyter community kernel list](https://github.com/jupyter/jupyter/wiki/Jupyter-kernels)
- [Jupyter kernels documentation](https://docs.jupyter.org/en/latest/projects/kernels.html)
- [Jupyter kernelspec specification](https://jupyter-client.readthedocs.io/en/stable/kernels.html#kernel-specs)

Treat the community list as a discovery catalog, not a compatibility guarantee.
Before adding a kernel, verify that its upstream project is maintained, has
installation instructions, and implements the protocol operations exercised by
this project.

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
Environment-local kernelspec executables are normalized to absolute paths so
Replmux can launch them without activating the micromamba environment first.
Provisioning stops at the first package resolution or installation failure so a
partially available matrix is never reported as complete. Remove
`tests/jupyter-kernels/.kernels/` to reclaim the environments' disk space.
