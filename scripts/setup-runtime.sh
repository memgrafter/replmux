#!/usr/bin/env bash
#
# replmux-setup.sh — one-shot, idempotent installer for the Replmux kernel broker.
#
# Goal: make `replmux` "just work" from any directory/terminal, decoupled from any
# single repo or the /home/plato/code/replmux source tree.
#
# It provisions:
#   1. A dedicated python venv at ~/.venvs/replmux with a relocatable pyzmq
#      (copied from the club-3090 venv — see note below).
#   2. The kernel script at ~/.local/share/replmux/python_minimal_kernel.py.
#   3. A user systemd service (replmux.service) that keeps the broker alive and
#      restarts it on crash. Enable with `systemctl --user enable --now replmux`.
#
# Idempotent: safe to run repeatedly; won't clobber an existing good state.
#
# NOTE on pyzmq sourcing: system python3 on this box has no pip / ensurepip
# (python3.13-venv not installed, no sudo). The only working pyzmq is in the
# club-3090 repo venv, and it's a relocatable manylinux wheel (zmq/* + pyzmq.libs
# abi3 .so), so we COPY it rather than depend on that repo existing. If you later
# get pip (apt install python3.13-venv), prefer `pip install pyzmq`.

set -u

PYTHON_BIN="${PYTHON_BIN:-/usr/bin/python3}"
VENV_DIR="$HOME/.venvs/replmux"
VENV_PY="$VENV_DIR/bin/python"
SHARE_DIR="$HOME/.local/share/replmux"
KERNEL_SRC="${REPLMUX_KERNEL_SRC:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)/cli/assets/python_minimal_kernel.py}"
KERNEL_DST="$SHARE_DIR/python_minimal_kernel.py"
REPLMUX_BIN="${REPLMUX_BIN:-$HOME/.local/bin/replmux}"
UNIT="$HOME/.config/systemd/user/replmux.service"

# club-3090 venv holds the only pyzmq we can relocate (no pip on this box).
ZMQSYS_SRC="${ZMQSYS_SRC:-$HOME/clones/club-3090/.venv/lib/python3.13/site-packages}"

say(){ printf '[replmux-setup] %s\n' "$*"; }
die(){ say "FATAL: $*"; exit 1; }

[ -x "$REPLMUX_BIN" ] || die "replmux binary not found at $REPLMUX_BIN"

# --- 1. venv + pyzmq -----------------------------------------------------------
mkdir -p "$VENV_DIR"
if [ ! -x "$VENV_PY" ]; then
    say "creating venv (--without-pip; no ensurepip on this host)"
    "$PYTHON_BIN" -m venv --without-pip "$VENV_DIR" || die "venv creation failed"
fi
SP="$VENV_DIR/lib/python3.13/site-packages"
if ! "$VENV_PY" -c "import zmq" >/dev/null 2>&1; then
    say "installing pyzmq into venv (relocating manylinux wheel from club-3090 venv)"
    mkdir -p "$SP"
    for d in zmq pyzmq.libs pyzmq-*.dist-info; do
        [ -e "$ZMQSYS_SRC/$d" ] || die "missing $ZMQSYS_SRC/$d (club-3090 venv moved?)"
        rm -rf "$SP/$d"
        cp -r "$ZMQSYS_SRC/$d" "$SP/"
    done
    "$VENV_PY" -c "import zmq; print('zmq', zmq.__version__)" || die "pyzmq failed in $VENV_PY"
else
    say "zmq already present in $VENV_PY"
fi

# --- 2. kernel script ----------------------------------------------------------
mkdir -p "$SHARE_DIR"
if [ ! -f "$KERNEL_DST" ] || [ ! -s "$KERNEL_DST" ]; then
    [ -f "$KERNEL_SRC" ] || die "kernel source not found at $KERNEL_SRC"
    cp "$KERNEL_SRC" "$KERNEL_DST"
    say "copied kernel script to $KERNEL_DST"
else
    say "kernel script present at $KERNEL_DST"
fi

# --- 3. systemd unit -----------------------------------------------------------
mkdir -p "$(dirname "$UNIT")"
cat > "$UNIT" <<EOF
[Unit]
Description=Replmux kernel broker (persistent compute kernels over HTTP)
After=network.target

[Service]
Type=simple
Environment=REPLMUX_PYTHON=$VENV_PY
Environment=REPLMUX_KERNEL_SCRIPT=$KERNEL_DST
ExecStart=$REPLMUX_BIN serve
Restart=on-failure
RestartSec=5
KillMode=mixed

[Install]
WantedBy=default.target
EOF
systemctl --user daemon-reload
systemctl --user start replmux.service
systemctl --user enable replmux.service >/dev/null 2>&1 || say "note: not enabled at boot (run 'loginctl enable-linger' + this to autostart)"

say "done. state: active=$(systemctl --user is-active replmux.service)"
say "usage: replmux kernel create <name> ; replmux --json kernel list"
