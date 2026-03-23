# Setup Guide

This guide explains how to create a tmux session script and start the project/microservices. It mirrors the patterns used in the allinonepos worktree scripts and env files.

## Prerequisites

- `tmux`
- `direnv`
- `air` (for Go services)
- `npm` (for web frontends)

## Environment setup

### Automatic Setup (All repos on server already have these files)

All API services in `/root/workspaces/allinonepos/` have `.envrc` files created from their `.env.example`.
All frontend services have `.env` files with the correct API base URLs.

**You don't need to create these manually** - they're already there!

### API Services (.envrc files)

Each Go API service uses a `.envrc` file for environment variables, loaded via `direnv`.

Example `.envrc` pattern:

```bash
export DEBUG=true
export HOST=localhost
export PORT=8001
export GRPC_SERVER_PORT=9001
export POSTGRESQL_DSL=postgresql://postgres:123456@localhost:5432/pos_identity
export GOPRIVATE=github.com/allinonepos
export JWT_SECRET=123456
export REFRESH_SECRET=abcd1234
```

**Available .envrc files:**
- `pos-identity-api/.envrc` (PORT=8001, GRPC=9001)
- `pos-commons-api/.envrc` (PORT=8003, GRPC=9003)
- `pos-customer-api/.envrc` (PORT=8002, GRPC=9002)
- `pos-inventory-api/.envrc` (PORT=8004, GRPC=9004)
- `pos-loro-api/.envrc` (PORT=8005, GRPC=9005)
- `pos-payment-api/.envrc` (PORT=8006, GRPC=9006)
- `pos-super-admin-api/.envrc` (PORT=8008, GRPC=9008)
- And more...

### Frontend Services (.env files)

Frontends use a `.env` file with base URLs:

```env
NEXT_PUBLIC_BASE_URL_IDENTITY="http://localhost:8001"
NEXT_PUBLIC_BASE_URL_COMMONS="http://localhost:8003"
NEXT_PUBLIC_BASE_URL_CUSTOMER="http://localhost:8002"
NEXT_PUBLIC_BASE_URL_INVENTORY="http://localhost:8004"
NEXT_PUBLIC_BASE_URL_LORO="http://localhost:8005"
NEXT_PUBLIC_BASE_URL_PAYMENT="http://localhost:8006"
NEXT_PUBLIC_BASE_URL_IPG="https://ipg.dev.marxpos.com"
```

**Available .env files:**
- `pos-web/.env`
- `pos-super-admin/.env`
- `pos-customer/.env`
- `pos-mobile/.env`

### Before Running Services

For API services, allow direnv to load the environment:

```bash
cd pos-identity-api
direnv allow
```

You only need to run `direnv allow` once per service after the `.envrc` is created.

## Tmux session script template

Create a script like `scripts/dev-tmux-session.sh` in your project:

```bash
#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SESSION_NAME="${MYAPP_TMUX_SESSION_NAME:-$(basename "${ROOT_DIR}")}"

if ! command -v tmux >/dev/null 2>&1; then
  echo "tmux is not installed. Install tmux and rerun this script." >&2
  exit 1
fi

if tmux has-session -t "${SESSION_NAME}" 2>/dev/null; then
  echo "tmux session \"${SESSION_NAME}\" already exists."
  echo "Attach with: tmux attach -t ${SESSION_NAME}"
  exit 0
fi

create_window() {
  local window_name="$1"
  local working_dir="$2"
  local command="$3"

  tmux new-window -t "${SESSION_NAME}:" -n "${window_name}" -c "${working_dir}"
  tmux send-keys -t "${SESSION_NAME}:${window_name}" "${command}" C-m
}

AIR_COMMAND="direnv allow && direnv exec . air"

tmux new-session -d -s "${SESSION_NAME}" -n "identity-api" \
  -c "${ROOT_DIR}/pos-identity-api"
tmux send-keys -t "${SESSION_NAME}:identity-api" "${AIR_COMMAND}" C-m

create_window "commons-api" "${ROOT_DIR}/pos-commons-api" "${AIR_COMMAND}"
create_window "customer-api" "${ROOT_DIR}/pos-customer-api" "${AIR_COMMAND}"
create_window "inventory-api" "${ROOT_DIR}/pos-inventory-api" "${AIR_COMMAND}"
create_window "loro-api" "${ROOT_DIR}/pos-loro-api" "${AIR_COMMAND}"
create_window "payment-api" "${ROOT_DIR}/pos-payment-api" "${AIR_COMMAND}"
create_window "bill-payment-api" "${ROOT_DIR}/pos-bill-payment-api" "${AIR_COMMAND}"
create_window "super-admin-api" "${ROOT_DIR}/pos-super-admin-api" "${AIR_COMMAND}"
create_window "web" "${ROOT_DIR}/pos-web" "npm run dev -- --port 3000"
create_window "super-admin" "${ROOT_DIR}/pos-super-admin" "npm run dev -- --port 3001"
create_window "loro-simulation" "${ROOT_DIR}/pos-loro-simulation-page" "npx serve -l 3002 src"

cat <<EOF
tmux session "${SESSION_NAME}" created.

Attach with: tmux attach -t ${SESSION_NAME}

Helpful tips:
  • Press Ctrl-b d to detach while keeping services alive.
EOF
```

## Start all services

From the project root:

```bash
./scripts/dev-tmux-session.sh
tmux attach -t "$(basename "$PWD")"
```

## Dashboard start/stop contract

The dashboard "Running Projects" start/stop buttons control tmux sessions directly.

To make this deterministic, keep this contract in each workspace:

- Session bootstrap script: `scripts/dev-tmux-session.sh`
- Session name env: `MYAPP_TMUX_SESSION_NAME` (default workspace folder name)
- Optional frontend tunnels: create one tunnel per frontend port (3000, 3001, 3002, ...)

When a project is started from the dashboard:

1. The daemon runs `bash scripts/dev-tmux-session.sh` if present.
2. If no session is created by script, daemon falls back to `<workspace-name>`.
3. Dashboard exposes clickable URLs:
   - frontend URLs from active preview tunnels (multiple supported)

When a project is stopped from the dashboard:

- The daemon stops ttyd and kills tmux session `<workspace-name>`.

## Multi-frontend URL workflow

If your project runs multiple frontends (for example on ports 3000, 3001, 3002), define them in:

- `.opencode/frontends.json`

Example:

```json
{
  "frontends": [
    { "name": "web", "port": 3000 },
    { "name": "admin", "port": 3001 },
    { "name": "docs", "port": 3002 }
  ]
}
```

Then publish from workspace root:

```bash
./scripts/publish-frontends.sh
```

Or from dashboard:

- Click `Publish Frontends` in the project card.

The dashboard will render all frontend links as separate clickable URLs:

- Hosted Frontend links when public tunnels are active
- Local Frontend links as fallback per configured port

Recommended labels in your tmux script:

- `web` -> port 3000
- `admin` -> port 3001
- `docs` -> port 3002

This naming makes it easier for agents and humans to match URLs to windows.

## Tmux test log framework

Each workspace should include:

- `scripts/tmux-test-log.sh` to capture tmux pane output into logs
- `logs/tmux/` directory for output files

Create a test log manually:

```bash
./scripts/tmux-test-log.sh
```

The script should produce files like:

- `logs/tmux/tmux-test-<session>-<timestamp>.log`

From the dashboard, `TMUX Test Log` triggers the same flow through daemon APIs and returns the exact log path.

The log capture session must be named exactly as the workspace folder.

## Optional: lazygit session

If you want a lazygit window per service, use the pattern from `dev-tmux-lazygit-session.sh` in allinonepos:

```bash
SERVICES=(
  "identity-api:${ROOT_DIR}/pos-identity-api"
  "commons-api:${ROOT_DIR}/pos-commons-api"
  "customer-api:${ROOT_DIR}/pos-customer-api"
  "inventory-api:${ROOT_DIR}/pos-inventory-api"
)
```

Then create tmux windows and run `lazygit` in each.

## Run a single service

Typical Go service:

```bash
direnv allow
direnv exec . air
```

Typical web frontend:

```bash
npm run dev -- --port 3000
```

## Notes

- Ensure each service has its `.envrc` and that `direnv allow` has been run.
- Use a distinct port per service to avoid conflicts.
