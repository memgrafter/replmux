#!/usr/bin/env python3
"""Provision the kernel test matrix with micromamba."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

TEST_DIRECTORY = Path(__file__).resolve().parent
DEFAULT_MATRIX = TEST_DIRECTORY / "kernels.toml"
DEFAULT_ENVIRONMENT_ROOT = TEST_DIRECTORY / ".kernels"


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "kernel_ids",
        nargs="*",
        help="kernel IDs to install; omit to install the complete matrix",
    )
    parser.add_argument("--matrix", type=Path, default=DEFAULT_MATRIX)
    parser.add_argument("--environment-root", type=Path, default=DEFAULT_ENVIRONMENT_ROOT)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="print commands without creating environments",
    )
    return parser.parse_args()


def load_kernels(matrix_path: Path) -> list[dict[str, Any]]:
    with matrix_path.open("rb") as matrix_file:
        kernels = tomllib.load(matrix_file).get("kernels", [])
    if not kernels:
        raise ValueError(f"no kernels found in {matrix_path}")

    seen_ids: set[str] = set()
    for kernel in kernels:
        kernel_id = kernel.get("id")
        provisioner = kernel.get("micromamba")
        if not isinstance(kernel_id, str) or not kernel_id:
            raise ValueError("every kernel must have a non-empty id")
        if kernel_id in seen_ids:
            raise ValueError(f"duplicate kernel id: {kernel_id}")
        if not isinstance(provisioner, dict) or not provisioner.get("packages"):
            raise ValueError(f"kernel {kernel_id} has no micromamba packages")
        seen_ids.add(kernel_id)
    return kernels


def select_kernels(
    kernels: list[dict[str, Any]], requested_ids: list[str]
) -> list[dict[str, Any]]:
    if not requested_ids:
        return kernels
    by_id = {kernel["id"]: kernel for kernel in kernels}
    unknown_ids = sorted(set(requested_ids) - by_id.keys())
    if unknown_ids:
        raise ValueError(f"unknown kernel IDs: {', '.join(unknown_ids)}")
    return [by_id[kernel_id] for kernel_id in requested_ids]


def create_command(kernel: dict[str, Any], environment_root: Path) -> list[str]:
    provisioner = kernel["micromamba"]
    command = [
        "micromamba",
        "create",
        "--yes",
        "--prefix",
        str(environment_root / kernel["id"]),
    ]
    for channel in provisioner.get("channels", ["conda-forge"]):
        command.extend(("--channel", channel))
    command.extend(provisioner["packages"])
    return command


def setup_commands(kernel: dict[str, Any], environment_root: Path) -> list[list[str]]:
    prefix = environment_root / kernel["id"]
    return [
        [
            "micromamba",
            "run",
            "--prefix",
            str(prefix),
            *(argument.replace("{prefix}", str(prefix)) for argument in command),
        ]
        for command in kernel["micromamba"].get("setup", [])
    ]


def run_command(command: list[str], dry_run: bool) -> int:
    print(subprocess.list2cmdline(command), flush=True)
    if dry_run:
        return 0
    return subprocess.run(command, check=False).returncode


def main() -> int:
    arguments = parse_arguments()
    try:
        kernels = select_kernels(load_kernels(arguments.matrix), arguments.kernel_ids)
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2

    if not arguments.dry_run and shutil.which("micromamba") is None:
        print("error: micromamba is not installed or not on PATH", file=sys.stderr)
        return 127

    environment_root = arguments.environment_root.resolve()
    for kernel in kernels:
        print(f"\n==> {kernel['display_name']} ({kernel['id']})", flush=True)
        create = create_command(kernel, environment_root)
        return_code = run_command(create, arguments.dry_run)
        if return_code == 0 and not arguments.dry_run:
            kernelspec_directory = (
                environment_root / kernel["id"] / "share" / "jupyter" / "kernels"
            )
            kernelspec_directory.mkdir(parents=True, exist_ok=True)
        for command in setup_commands(kernel, environment_root):
            if return_code != 0:
                break
            return_code = run_command(command, arguments.dry_run)
        if return_code != 0:
            print(
                f"error: failed to provision {kernel['id']} ({return_code})",
                file=sys.stderr,
            )
            return return_code
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
