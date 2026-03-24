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

## PostgreSQL bootstrap (required before API start)

If you see auth errors or missing DB errors, run this from the workspace root (`allinonepos`):

```bash
set -euo pipefail

POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-123456}"

# 1) Ensure PostgreSQL is running
if ! pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
  sudo systemctl start postgresql || sudo service postgresql start
fi

# 2) Align postgres user password with .envrc expectation
sudo -u postgres psql -v ON_ERROR_STOP=1 -d postgres -c "ALTER USER postgres WITH PASSWORD '${POSTGRES_PASSWORD}';"

# 3) Create required databases (idempotent)
for db in \
  pos_identity \
  pos_commons \
  pos_customer \
  pos_inventory \
  pos_loro \
  pos_payment \
  pos_super_admin; do
  exists="$(PGPASSWORD="${POSTGRES_PASSWORD}" psql -h localhost -U postgres -d postgres -Atqc "SELECT 1 FROM pg_database WHERE datname='${db}'" || true)"
  if [[ "${exists}" != "1" ]]; then
    PGPASSWORD="${POSTGRES_PASSWORD}" createdb -h localhost -U postgres "${db}"
  fi
done

# 4) Apply SQL migrations only on empty schemas
for entry in \
  "pos-identity-api:pos_identity" \
  "pos-commons-api:pos_commons" \
  "pos-customer-api:pos_customer" \
  "pos-inventory-api:pos_inventory" \
  "pos-loro-api:pos_loro" \
  "pos-payment-api:pos_payment" \
  "pos-super-admin-api:pos_super_admin"; do
  service="${entry%%:*}"
  db="${entry##*:}"

  if [[ ! -d "${service}" ]]; then
    echo "skip ${service} (directory missing)"
    continue
  fi

  if [[ ! -f "${service}/.envrc" && -f "${service}/.env.example" ]]; then
    cp "${service}/.env.example" "${service}/.envrc"
  fi

  (cd "${service}" && direnv allow >/dev/null 2>&1 || true)

  if [[ ! -d "${service}/internal/db/migrations" ]]; then
    echo "skip ${service} migrations (internal/db/migrations missing)"
    continue
  fi

  table_count="$(PGPASSWORD="${POSTGRES_PASSWORD}" psql -h localhost -U postgres -d "${db}" -Atqc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public'" || echo 0)"
  if [[ "${table_count}" != "0" ]]; then
    echo "skip ${service} migration replay (schema already has ${table_count} tables)"
    continue
  fi

  shopt -s nullglob
  files=("${service}/internal/db/migrations"/*.up.sql)
  shopt -u nullglob
  for migration in "${files[@]}"; do
    PGPASSWORD="${POSTGRES_PASSWORD}" psql -h localhost -U postgres -d "${db}" -v ON_ERROR_STOP=1 -f "${migration}"
  done
done
```

Notes:

- If your `.envrc` files use a different password, set `POSTGRES_PASSWORD` to match before running the block.
- This flow is for local PostgreSQL on `localhost:5432`.
- Migrations are only auto-replayed when the target schema is empty.

## API env normalization (required)

Before starting APIs, normalize `.envrc` values so agents do not accidentally run services on the wrong ports or wrong DB passwords.

Run from workspace root (`allinonepos`):

```bash
set -euo pipefail

POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-123456}"

sed -i "s|^export PORT=.*|export PORT=8001|" pos-identity-api/.envrc
sed -i "s|^export GRPC_SERVER_PORT=.*|export GRPC_SERVER_PORT=9001|" pos-identity-api/.envrc
sed -i "s|^export POSTGRESQL_DSL=.*|export POSTGRESQL_DSL=postgresql://postgres:${POSTGRES_PASSWORD}@localhost:5432/pos_identity?sslmode=disable|" pos-identity-api/.envrc

sed -i "s|^export PORT=.*|export PORT=8003|" pos-commons-api/.envrc
sed -i "s|^export GRPC_SERVER_PORT=.*|export GRPC_SERVER_PORT=9003|" pos-commons-api/.envrc
sed -i "s|^export POSTGRESQL_DSL=.*|export POSTGRESQL_DSL=postgresql://postgres:${POSTGRES_PASSWORD}@localhost:5432/pos_commons?sslmode=disable|" pos-commons-api/.envrc

sed -i "s|^export PORT=.*|export PORT=8002|" pos-customer-api/.envrc
sed -i "s|^export GRPC_SERVER_PORT=.*|export GRPC_SERVER_PORT=9002|" pos-customer-api/.envrc
sed -i "s|^export POSTGRESDB_URL=.*|export POSTGRESDB_URL=postgresql://postgres:${POSTGRES_PASSWORD}@localhost:5432/pos_customer?sslmode=disable|" pos-customer-api/.envrc

sed -i "s|^export PORT=.*|export PORT=8004|" pos-inventory-api/.envrc
sed -i "s|^export GRPC_SERVER_PORT=.*|export GRPC_SERVER_PORT=9004|" pos-inventory-api/.envrc
sed -i "s|^export POSTGRESQL_DSL=.*|export POSTGRESQL_DSL=postgresql://postgres:${POSTGRES_PASSWORD}@localhost:5432/pos_inventory?sslmode=disable|" pos-inventory-api/.envrc

sed -i "s|^export PORT=.*|export PORT=8005|" pos-loro-api/.envrc
sed -i "s|^export GRPC_SERVER_PORT=.*|export GRPC_SERVER_PORT=9005|" pos-loro-api/.envrc
sed -i "s|^export POSTGRESDB_URL=.*|export POSTGRESDB_URL=postgresql://postgres:${POSTGRES_PASSWORD}@localhost:5432/pos_loro?sslmode=disable|" pos-loro-api/.envrc

sed -i "s|^export PORT=.*|export PORT=8006|" pos-payment-api/.envrc
sed -i "s|^export GRPC_SERVER_PORT=.*|export GRPC_SERVER_PORT=9006|" pos-payment-api/.envrc
sed -i "s|^export POSTGRESDB_URL=.*|export POSTGRESDB_URL=postgresql://postgres:${POSTGRES_PASSWORD}@localhost:5432/pos_payment?sslmode=disable|" pos-payment-api/.envrc

sed -i "s|^export PORT=.*|export PORT=8008|" pos-super-admin-api/.envrc
sed -i "s|^export GRPC_SERVER_PORT=.*|export GRPC_SERVER_PORT=9008|" pos-super-admin-api/.envrc
sed -i "s|^export POSTGRESQL_DSL=.*|export POSTGRESQL_DSL=postgresql://postgres:${POSTGRES_PASSWORD}@localhost:5432/pos_super_admin?sslmode=disable|" pos-super-admin-api/.envrc
```

If your password contains URL-reserved characters, use a URL-encoded password value.

## Route inventory refresh (required before claiming exhaustive coverage)

Before running a "full" test, refresh route counts from source to avoid stale coverage assumptions.

Run from workspace root (`allinonepos`):

```bash
WEB_COUNT=$(find pos-web/src/app -type f \( -name 'page.tsx' -o -name 'page.ts' -o -name 'page.jsx' -o -name 'page.js' \) | wc -l | tr -d ' ')
ADMIN_COUNT=$(find pos-super-admin/src/app -type f \( -name 'page.tsx' -o -name 'page.ts' -o -name 'page.jsx' -o -name 'page.js' \) | wc -l | tr -d ' ')
CUSTOMER_COUNT=$(find pos-customer/src/app -type f \( -name 'page.tsx' -o -name 'page.ts' -o -name 'page.jsx' -o -name 'page.js' \) | wc -l | tr -d ' ')
TOTAL_COUNT=$((WEB_COUNT + ADMIN_COUNT + CUSTOMER_COUNT))

printf 'Route counts -> web:%s admin:%s customer:%s total:%s\n' "$WEB_COUNT" "$ADMIN_COUNT" "$CUSTOMER_COUNT" "$TOTAL_COUNT"
```

Expected baseline for current allinonepos revision:

- web: `55`
- admin: `17`
- customer: `2`
- total: `74`

If counts differ, update `workspace-docs/TEST_CASES.md` route inventory before marking runs as exhaustive.

## Start required backend APIs (tmux session contract)

Do not run APIs in a random shell. Use the workspace tmux session (`allinonepos`) and one window per API.
If tmux has only the default `app` window, treat that as **not started** (scaffold placeholder only).

```bash
set -euo pipefail

ROOT_DIR="$(pwd)"
SESSION_NAME="${MYAPP_TMUX_SESSION_NAME:-$(basename "$ROOT_DIR")}" 

if [[ "${SESSION_NAME}" == "uwu-main" ]]; then
  SESSION_NAME="$(basename "$ROOT_DIR")"
fi

tmux has-session -t "${SESSION_NAME}" 2>/dev/null || tmux new-session -d -s "${SESSION_NAME}" -n app -c "${ROOT_DIR}"

for pair in \
  "identity-api:pos-identity-api" \
  "commons-api:pos-commons-api" \
  "customer-api:pos-customer-api" \
  "inventory-api:pos-inventory-api" \
  "loro-api:pos-loro-api" \
  "payment-api:pos-payment-api" \
  "super-admin-api:pos-super-admin-api"; do
  win="${pair%%:*}"
  dir="${pair##*:}"

  if tmux list-windows -t "${SESSION_NAME}" -F "#{window_name}" | grep -qx "${win}"; then
    tmux send-keys -t "${SESSION_NAME}:${win}" C-c
    tmux send-keys -t "${SESSION_NAME}:${win}" "cd ${ROOT_DIR}/${dir} && direnv allow && direnv exec . air" C-m
  else
    tmux new-window -t "${SESSION_NAME}:" -n "${win}" -c "${ROOT_DIR}/${dir}"
    tmux send-keys -t "${SESSION_NAME}:${win}" "direnv allow && direnv exec . air" C-m
  fi
done
```

Required backend port map:

- identity-api: `localhost:8001`
- commons-api: `localhost:8003`
- customer-api: `localhost:8002`
- inventory-api: `localhost:8004`
- loro-api: `localhost:8005`
- payment-api: `localhost:8006`
- super-admin-api: `localhost:8008`

Session rule:

- `uwu-main` is reserved for OpenCode tabs; backend services must run in workspace session (`allinonepos` or your workspace name).

Validation command:

```bash
ss -ltnp | grep -E ':(8001|8002|8003|8004|8005|8006|8008)\b'
```

## gRPC dependency health check (identity signup path)

For merchant signup (`POST /user/register`), identity-api must reach inventory gRPC service.

Quick checks:

```bash
cd /root/workspaces/allinonepos

grep -E '^export GRPC_INVENTORY_SERVICE_(HOST|PORT)=' pos-identity-api/.envrc
grep -E '^export GRPC_SERVER_PORT=' pos-inventory-api/.envrc
ss -ltnp | grep -E ':(9001|9004)\b'
```

Expected values:

- `pos-identity-api/.envrc` -> `GRPC_INVENTORY_SERVICE_HOST=localhost`, `GRPC_INVENTORY_SERVICE_PORT=9004`
- `pos-inventory-api/.envrc` -> `GRPC_SERVER_PORT=9004`

If mismatched:

1. Fix `.envrc` values (infra only, no logic/code changes).
2. Restart affected tmux windows (at minimum `identity-api`, and dependency service if changed).
3. Re-run signup test before marking any blocker.

Note: If failure is page/route mismatch (404 or wrong page name like `junk-qr-payments`), treat it as test-case failure and correct the navigation/page target; do not classify as infra blocker.

## Merchant signup OTP retrieval (commons-api tmux window)

Merchant registration OTPs must be read from `commons-api` logs in the workspace tmux session.

```bash
set -euo pipefail

SESSION_NAME="${MYAPP_TMUX_SESSION_NAME:-$(basename "$PWD")}" 
PHONE_INPUT="771234567"      # exact digits typed in UI when +94 is prefilled
PHONE_E164="+94${PHONE_INPUT}"

if [[ "${SESSION_NAME}" == "uwu-main" ]]; then
  SESSION_NAME="$(basename "$PWD")"
fi

tmux capture-pane -pt "${SESSION_NAME}:commons-api.0" -S -500 \
  | grep -E "New Sandbox mode SMS|Your OTP is|OTP" \
  | grep -F "${PHONE_E164}" \
  | tail -n 20
```

Rules:

- Do not use OTP from other sessions (for example `kyc-test`).
- Match OTP to the exact phone number used in the merchant signup step.
- Use phone format `+94770805444` (E.164, no spaces) for signup/login test data.
- For merchant login UI fields where `+94` is prefilled, type only the remaining digits (example: `770805444`).
- In merchant signup, Terms & Conditions checkbox must be checked before submit, otherwise treat as `FAIL` if the flow is stuck.
- If no matching OTP is present in `commons-api` logs, mark OTP-dependent tests as `FAIL`.

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

Button semantics:

- `Start` -> ensure tmux session `<workspace-name>` exists.
- `Stop` -> stop ttyd for the workspace and kill tmux session `<workspace-name>`.
- `Publish Frontends` -> publish all ports declared in `.opencode/frontends.json`.
- `TMUX Test Log` -> capture from tmux session `<workspace-name>` only.

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

## Regression report artifact validation

Before declaring a regression run successful, validate report artifacts from workspace root:

If `/test-reports` shows a run with missing `index.html`/`manifest.json`, that run did not bootstrap correctly.
Start by creating run artifacts first (as defined in `workspace-docs/TEST_CASES.md` step `Run bootstrap`), then execute tests.

Also ensure each run writes `coverage.json` so exhaustive route/button/form coverage is auditable.

```bash
set -euo pipefail

RUN_ID="2026-03-2014-30-00"   # replace
export RUN_ID
WORKSPACE_NAME="$(basename "$PWD")"
BASE_URL="https://code.vidwadeseram.com/test-reports/${WORKSPACE_NAME}/${RUN_ID}"
RUN_DIR="logs/${RUN_ID}"

test -f "${RUN_DIR}/index.html"
test -f "${RUN_DIR}/manifest.json"
test -f "${RUN_DIR}/coverage.json"
test -d "${RUN_DIR}/screenshots"
test -f "${RUN_DIR}/video/full-process.webm" || test -f "${RUN_DIR}/video/full-process.mp4"

curl -fsS "${BASE_URL}/index.html" >/dev/null
curl -fsS "${BASE_URL}/manifest.json" >/dev/null

python3 - <<'PY'
import json, os, pathlib, sys

run_id = os.environ.get("RUN_ID", "").strip()
if not run_id:
    print("FAIL")
    print("- RUN_ID is required")
    sys.exit(1)

run = pathlib.Path("logs") / run_id
manifest = json.loads((run / "manifest.json").read_text())
coverage = json.loads((run / "coverage.json").read_text())
summary = manifest.get("summary") or {}

errors = []
for item in manifest.get("screenshots", []):
    p = str(item.get("path", "")).strip()
    if p.startswith(f"logs/{run_id}/"):
        p = p[len(f"logs/{run_id}/"):]
    if not (run / p).is_file():
        errors.append(f"missing screenshot artifact: {item.get('path')}")

v = str((manifest.get("video") or {}).get("path", "")).strip()
if v.startswith(f"logs/{run_id}/"):
    v = v[len(f"logs/{run_id}/"):]
video_file = run / v
if not video_file.is_file():
    errors.append(f"video.path is not a file: {manifest.get('video')}")
elif video_file.stat().st_size <= 0:
    errors.append("video artifact is zero bytes")

total = int(summary.get("total", 0))
passed = int(summary.get("passed", 0))
failed = int(summary.get("failed", 0))
skipped = int(summary.get("skipped", 0))
blocked = int(summary.get("blocked", 0))
if total != passed + failed + skipped + blocked:
    errors.append("manifest summary mismatch (total != passed+failed+skipped+blocked)")

tests = manifest.get("tests") or []
screenshot_counts = {}
for shot in manifest.get("screenshots", []):
    test_id = str(shot.get("test_id") or "").strip()
    if test_id:
        key = test_id.lower().replace("_", "-")
        screenshot_counts[key] = screenshot_counts.get(key, 0) + 1

for case in tests:
    status = str(case.get("status") or "").strip().lower()
    case_id = str(case.get("id") or "").strip()
    if status in {"fail", "blocked"} and case_id:
        key = case_id.lower().replace("_", "-")
        if screenshot_counts.get(key, 0) <= 0:
            errors.append(f"missing FAIL/BLOCKED screenshot evidence for {case_id}")

dashboard_auth_failures = 0
for case in tests:
    status = str(case.get("status") or "").strip().lower()
    if status not in {"fail", "blocked"}:
        continue
    blob = " ".join([
        str(case.get("id") or ""),
        str(case.get("name") or ""),
        str(case.get("error") or ""),
    ]).lower()
    if "dashboard" in blob and any(k in blob for k in ["redirected to login", "requires authentication", "unauthorized", "401", "403"]):
        dashboard_auth_failures += 1
if dashboard_auth_failures > 0:
    errors.append(f"dashboard/login readiness failed in {dashboard_auth_failures} case(s)")

index_text = (run / "index.html").read_text(errors="ignore").lower()
if "video recording placeholder" in index_text:
    errors.append("index.html contains video placeholder text")

route_total = int(coverage.get("route_total", 0))
route_covered = int(coverage.get("route_covered", 0))
button_total = int(coverage.get("button_total", 0))
button_covered = int(coverage.get("button_covered", 0))
form_total = int(coverage.get("form_total", 0))
form_covered = int(coverage.get("form_covered", 0))

if route_total <= 0:
    errors.append("coverage route_total must be > 0")
if route_covered != route_total:
    errors.append("coverage route_covered must equal route_total for exhaustive run")
if button_covered > button_total:
    errors.append("coverage button_covered exceeds button_total")
if form_covered > form_total:
    errors.append("coverage form_covered exceeds form_total")
if button_total <= 0 or form_total <= 0:
    errors.append("coverage button_total/form_total must be > 0 for exhaustive run")

if errors:
    print("FAIL")
    for err in errors:
        print(f"- {err}")
    sys.exit(1)

print("PASS: report artifacts are present")
PY
```

Manual quality gate (required):

- Open a few screenshot links from `index.html` and verify they are real app states, not error pages.
- If a screenshot shows `404`, `Not Found`, or an app error screen, mark that test `FAIL`.
- If a screenshot only shows loading UI (spinner/skeleton/blank placeholder), do not mark that test `PASS`; recapture after UI stabilizes or mark `FAIL`.
- Ensure the video section includes a direct clickable file link to `video/full-process.webm` (or `.mp4`).
- Ensure screenshot capture happens only after route + heading + primary data content are all visible.
- If `index.html` contains `Video recording placeholder` text, the run is invalid and must be regenerated.

## New workspace scaffolding contract

When a workspace is created through daemon APIs, these files must be present:

- `scripts/dev-tmux-session.sh`
- `scripts/publish-frontends.sh`
- `scripts/tmux-test-log.sh`
- `.opencode/frontends.json`

Agents should treat these files as the default automation framework for run/publish/log tasks.

Slash command contract for testing:

- `/start-test` -> test `main` branch
- `/start-test <branch-name>` or `/start-test --branch <branch-name>` -> test that branch
- `/start-test <pr-url> [<pr-url> ...]` -> resolve PR branches and test each target
- `/start-test --repo <repo-path-or-name> [targets...]` -> run only for one repo inside a multi-repo workspace
- If workspace root is not a git repo, `/start-test` must discover nested git repos and continue instead of failing immediately.

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
