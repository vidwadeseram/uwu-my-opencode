#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "[uwu] Linux only." >&2; exit 1
fi

if [[ $EUID -eq 0 ]]; then
  echo "[uwu] Run as a normal user with sudo, not root." >&2; exit 1
fi

HOME_DIR="${HOME:-/root}"

if ! command -v cargo &>/dev/null; then
  echo "[uwu] installing rust..."
  curl https://sh.rustup.rs -sSf | sh -s -- -y
  export PATH="$HOME_DIR/.cargo/bin:$PATH"
fi

INSTALL_DIR="${HOME_DIR}/uwu-my-opencode"

if [ -d "$INSTALL_DIR/.git" ]; then
  echo "[uwu] updating repo..."
  git -C "$INSTALL_DIR" pull --ff-only
else
  echo "[uwu] cloning repo..."
  git clone https://github.com/vidwadeseram/uwu-my-opencode.git "$INSTALL_DIR"
fi

echo "[uwu] building uwu-daemon..."
cargo build --manifest-path "$INSTALL_DIR/daemon/Cargo.toml" --release

echo "[uwu] running installer..."
"$INSTALL_DIR/daemon/target/release/uwu-daemon" install "$@"
