#!/usr/bin/env bash
set -euo pipefail

: "${UWU_EXECUTE_COMMANDS:=true}"
: "${UWU_SKIP_DOTFILES_BOOTSTRAP:=true}"
: "${UWU_HOST:=0.0.0.0}"
: "${UWU_PORT:=18080}"
: "${UWU_WORKSPACE_ROOT:=/data/workspaces}"
: "${UWU_STATE_FILE:=/data/state/state.json}"
: "${UWU_PORT_RANGE_START:=4100}"
: "${UWU_PORT_RANGE_END:=4999}"
: "${UWU_TTYD_PORT_START:=7681}"
: "${UWU_TTYD_USER:=admin}"
: "${UWU_TTYD_PASS:=admin}"

export UWU_EXECUTE_COMMANDS
export UWU_SKIP_DOTFILES_BOOTSTRAP

mkdir -p "${UWU_WORKSPACE_ROOT}" "$(dirname "${UWU_STATE_FILE}")"
mkdir -p "${UWU_WORKSPACE_ROOT}/workspace-1"

exec /app/daemon/target/release/uwu-daemon \
  --host "${UWU_HOST}" \
  --port "${UWU_PORT}" \
  --workspace-root "${UWU_WORKSPACE_ROOT}" \
  --state-file "${UWU_STATE_FILE}" \
  --port-range-start "${UWU_PORT_RANGE_START}" \
  --port-range-end "${UWU_PORT_RANGE_END}" \
  --ttyd-port-start "${UWU_TTYD_PORT_START}" \
  --ttyd-user "${UWU_TTYD_USER}" \
  --ttyd-pass "${UWU_TTYD_PASS}" \
  --tmux-bin "/opt/uwu/tmux/bin/tmux" \
  --opencode-repo "/app/opencode" \
  --oh-my-opencode-repo "/app/oh-my-opencode"
