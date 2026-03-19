#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
  echo "[uwu] Docker installer currently supports Linux only." >&2
  exit 1
fi

if [[ $EUID -eq 0 ]]; then
  echo "[uwu] Run as a normal user with sudo, not root." >&2
  exit 1
fi

HOME_DIR="${HOME:-/root}"
INSTALL_DIR="${HOME_DIR}/uwu-my-opencode"

echo "[uwu] ensuring Docker engine is installed..."
if ! command -v docker &>/dev/null; then
  curl -fsSL https://get.docker.com | sudo sh
fi

if ! sudo docker compose version &>/dev/null; then
  echo "[uwu] installing docker compose plugin..."
  sudo apt-get update -qq
  sudo apt-get install -y -qq docker-compose-plugin
fi

if [ -d "${INSTALL_DIR}/.git" ]; then
  echo "[uwu] updating repo..."
  git -C "${INSTALL_DIR}" pull --ff-only
else
  echo "[uwu] cloning repo..."
  git clone https://github.com/vidwadeseram/uwu-my-opencode.git "${INSTALL_DIR}"
fi

echo "[uwu] syncing submodules..."
git -C "${INSTALL_DIR}" submodule update --init --recursive

if [ ! -f "${INSTALL_DIR}/.env.docker" ]; then
  echo "[uwu] creating .env.docker from template..."
  cp "${INSTALL_DIR}/.env.docker.example" "${INSTALL_DIR}/.env.docker"
fi

echo "[uwu] building and starting Docker stack..."
sudo docker compose --env-file "${INSTALL_DIR}/.env.docker" -f "${INSTALL_DIR}/docker-compose.yml" up -d --build

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  uwu-my-opencode Docker stack is running"
echo
echo "  Dashboard API: http://127.0.0.1:18080/health"
echo "  Terminal:      http://127.0.0.1:7681"
echo "  Credentials:   admin / admin (change in .env.docker)"
echo
echo "  Manage:"
echo "    sudo docker compose --env-file ${INSTALL_DIR}/.env.docker -f ${INSTALL_DIR}/docker-compose.yml ps"
echo "    sudo docker compose --env-file ${INSTALL_DIR}/.env.docker -f ${INSTALL_DIR}/docker-compose.yml logs -f"
echo "    sudo docker compose --env-file ${INSTALL_DIR}/.env.docker -f ${INSTALL_DIR}/docker-compose.yml down"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
