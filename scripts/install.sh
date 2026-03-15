#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "[uwu] Linux only." >&2; exit 1
fi

if [[ $EUID -eq 0 ]]; then
  echo "[uwu] Run as a normal user with sudo, not root." >&2; exit 1
fi

HOME_DIR="${HOME:-/root}"

if ! command -v gh &>/dev/null; then
  echo "[uwu] installing GitHub CLI (gh)..."
  if command -v apt-get &>/dev/null; then
    sudo apt-get update -qq
    if ! sudo apt-get install -y -qq gh; then
      sudo mkdir -p -m 755 /etc/apt/keyrings
      curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
        | sudo tee /etc/apt/keyrings/githubcli-archive-keyring.gpg >/dev/null
      sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg
      echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
        | sudo tee /etc/apt/sources.list.d/github-cli.list >/dev/null
      sudo apt-get update -qq
      sudo apt-get install -y -qq gh
    fi
  fi
fi

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
