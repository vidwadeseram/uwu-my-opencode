# allinonepos - Exhaustive Test Cases

This file defines the required coverage contract for full regression runs.

## 1) Coverage Contract

A full run is complete only when all of the following are true:

1. Every discovered frontend route is exercised.
2. Every visible click path on each route is exercised at least once.
3. Every form flow is exercised with both valid and invalid input paths.
4. Screenshots for PASS cases are verified to be stable and on the intended page.
5. One full-process video is present and playable.
6. `manifest.json` includes per-test entries (`tests`) and screenshot links.
7. `coverage.json` records route/button/form totals and covered counts.

## 2) Route Inventory (Source of Truth)

Generated from:

- `pos-web/src/app/**/page.tsx` -> 55 routes
- `pos-super-admin/src/app/**/page.tsx` -> 17 routes
- `pos-customer/src/app/**/page.tsx` -> 2 routes

### 2.1 Merchant App (`pos-web`) - 55 routes

Treat each entry below as mandatory route coverage. URL path is derived by removing route-group folders like `(logged-in)`.

| Route Key | Source Path | URL Path (expected) | Access |
| --- | --- | --- | --- |
| WEB-001 | `src/app/page.tsx` | `/` | public |
| WEB-002 | `src/app/(logged-in)/(dashboard)/page.tsx` | `/` | auth |
| WEB-003 | `src/app/(logged-out)/login/page.tsx` | `/login` | public |
| WEB-004 | `src/app/(logged-out)/signup/page.tsx` | `/signup` | public |
| WEB-005 | `src/app/(logged-out)/signup-verification/page.tsx` | `/signup-verification` | public |
| WEB-006 | `src/app/(logged-out)/verify-email/page.tsx` | `/verify-email` | public |
| WEB-007 | `src/app/(logged-out)/resetpswd/page.tsx` | `/resetpswd` | public |
| WEB-008 | `src/app/(logged-out)/resetpswd-verification/page.tsx` | `/resetpswd-verification` | public |
| WEB-009 | `src/app/(logged-out)/newpswd/page.tsx` | `/newpswd` | public |
| WEB-010 | `src/app/(logged-out)/e-bill/page.tsx` | `/e-bill` | public |
| WEB-011 | `src/app/(logged-out)/sms-recharge-status/page.tsx` | `/sms-recharge-status` | public |
| WEB-012 | `src/app/(logged-in)/accesslist/page.tsx` | `/accesslist` | auth |
| WEB-013 | `src/app/(logged-in)/myaccount/page.tsx` | `/myaccount` | auth |
| WEB-014 | `src/app/(logged-in)/help/page.tsx` | `/help` | auth |
| WEB-015 | `src/app/(logged-in)/timecard/page.tsx` | `/timecard` | auth |
| WEB-016 | `src/app/(logged-in)/totalworkhours/page.tsx` | `/totalworkhours` | auth |
| WEB-017 | `src/app/(logged-in)/employeelist/page.tsx` | `/employeelist` | auth |
| WEB-018 | `src/app/(logged-in)/customers/page.tsx` | `/customers` | auth |
| WEB-019 | `src/app/(logged-in)/items/page.tsx` | `/items` | auth |
| WEB-020 | `src/app/(logged-in)/items/itemlist/page.tsx` | `/items/itemlist` | auth |
| WEB-021 | `src/app/(logged-in)/items/categories/page.tsx` | `/items/categories` | auth |
| WEB-022 | `src/app/(logged-in)/transactions/page.tsx` | `/transactions` | auth |
| WEB-023 | `src/app/(logged-in)/transactions/ipg-payments/page.tsx` | `/transactions/ipg-payments` | auth |
| WEB-024 | `src/app/(logged-in)/transactions/qr-payments/page.tsx` | `/transactions/qr-payments` | auth |
| WEB-025 | `src/app/(logged-in)/transactions/void-history/page.tsx` | `/transactions/void-history` | auth |
| WEB-026 | `src/app/(logged-in)/marketing/custom-sms/page.tsx` | `/marketing/custom-sms` | auth |
| WEB-027 | `src/app/(logged-in)/ipg/page.tsx` | `/ipg` | auth |
| WEB-028 | `src/app/(logged-in)/ipg/payment-links/page.tsx` | `/ipg/payment-links` | auth |
| WEB-029 | `src/app/(logged-in)/report/page.tsx` | `/report` | auth |
| WEB-030 | `src/app/(logged-in)/report/sales-summary/page.tsx` | `/report/sales-summary` | auth |
| WEB-031 | `src/app/(logged-in)/report/sales-item/page.tsx` | `/report/sales-item` | auth |
| WEB-032 | `src/app/(logged-in)/report/sales-category/page.tsx` | `/report/sales-category` | auth |
| WEB-033 | `src/app/(logged-in)/report/sales-employee/page.tsx` | `/report/sales-employee` | auth |
| WEB-034 | `src/app/(logged-in)/report/sales-modifier/page.tsx` | `/report/sales-modifier` | auth |
| WEB-035 | `src/app/(logged-in)/report/sales-payment-type/page.tsx` | `/report/sales-payment-type` | auth |
| WEB-036 | `src/app/(logged-in)/report/receipts/page.tsx` | `/report/receipts` | auth |
| WEB-037 | `src/app/(logged-in)/report/stock-report/page.tsx` | `/report/stock-report` | auth |
| WEB-038 | `src/app/(logged-in)/report/shifts/page.tsx` | `/report/shifts` | auth |
| WEB-039 | `src/app/(logged-in)/report/taxes/page.tsx` | `/report/taxes` | auth |
| WEB-040 | `src/app/(logged-in)/billing/overview/page.tsx` | `/billing/overview` | auth |
| WEB-041 | `src/app/(logged-in)/billing/wallet-transfer/page.tsx` | `/billing/wallet-transfer` | auth |
| WEB-042 | `src/app/(logged-in)/billing/wallet-usage-history/page.tsx` | `/billing/wallet-usage-history` | auth |
| WEB-043 | `src/app/(logged-in)/billing/sms-usage-history/page.tsx` | `/billing/sms-usage-history` | auth |
| WEB-044 | `src/app/(logged-in)/billing/sms-recharge-history/page.tsx` | `/billing/sms-recharge-history` | auth |
| WEB-045 | `src/app/(logged-in)/settings/page.tsx` | `/settings` | auth |
| WEB-046 | `src/app/(logged-in)/settings/account/page.tsx` | `/settings/account` | auth |
| WEB-047 | `src/app/(logged-in)/settings/store/page.tsx` | `/settings/store` | auth |
| WEB-048 | `src/app/(logged-in)/settings/pos/page.tsx` | `/settings/pos` | auth |
| WEB-049 | `src/app/(logged-in)/settings/features/page.tsx` | `/settings/features` | auth |
| WEB-050 | `src/app/(logged-in)/settings/paymenttypes/page.tsx` | `/settings/paymenttypes` | auth |
| WEB-051 | `src/app/(logged-in)/settings/e-billTemplate/page.tsx` | `/settings/e-billTemplate` | auth |
| WEB-052 | `src/app/(logged-in)/settings/ipg-branding/page.tsx` | `/settings/ipg-branding` | auth |
| WEB-053 | `src/app/(logged-in)/settings/ticket/page.tsx` | `/settings/ticket` | auth |
| WEB-054 | `src/app/(logged-in)/settings/billing/page.tsx` | `/settings/billing` | auth |
| WEB-055 | `src/app/(logged-in)/settings/kyc/page.tsx` | `/settings/kyc` | auth |

### 2.2 Super Admin App (`pos-super-admin`) - 17 routes

| Route Key | Source Path | URL Path (expected) | Access |
| --- | --- | --- | --- |
| ADM-001 | `src/app/page.tsx` | `/` | public |
| ADM-002 | `src/app/(logged-out)/login/page.tsx` | `/login` | public |
| ADM-003 | `src/app/(logged-out)/password/otp/page.tsx` | `/password/otp` | public |
| ADM-004 | `src/app/(logged-out)/password/update/page.tsx` | `/password/update` | public |
| ADM-005 | `src/app/(logged-in)/(dashboard)/dashboard/page.tsx` | `/dashboard` | auth |
| ADM-006 | `src/app/(logged-in)/(dashboard)/digital-transactions/page.tsx` | `/digital-transactions` | auth |
| ADM-007 | `src/app/(logged-in)/(dashboard)/digital-transactions/ipg-payments/page.tsx` | `/digital-transactions/ipg-payments` | auth |
| ADM-008 | `src/app/(logged-in)/(dashboard)/digital-transactions/qr-payments/page.tsx` | `/digital-transactions/qr-payments` | auth |
| ADM-009 | `src/app/(logged-in)/(dashboard)/digital-transactions/junk-qr/page.tsx` | `/digital-transactions/junk-qr` | auth |
| ADM-010 | `src/app/(logged-in)/(dashboard)/digital-transactions/void-history/page.tsx` | `/digital-transactions/void-history` | auth |
| ADM-011 | `src/app/(logged-in)/(dashboard)/merchants-management/merchants/page.tsx` | `/merchants-management/merchants` | auth |
| ADM-012 | `src/app/(logged-in)/(dashboard)/merchants-management/kyc-approvals/page.tsx` | `/merchants-management/kyc-approvals` | auth |
| ADM-013 | `src/app/(logged-in)/(dashboard)/user-management/users/page.tsx` | `/user-management/users` | auth |
| ADM-014 | `src/app/(logged-in)/(dashboard)/user-management/roles/page.tsx` | `/user-management/roles` | auth |
| ADM-015 | `src/app/(logged-in)/(dashboard)/customers/page.tsx` | `/customers` | auth |
| ADM-016 | `src/app/(logged-in)/(dashboard)/business-name-approvals/page.tsx` | `/business-name-approvals` | auth |
| ADM-017 | `src/app/(logged-in)/(dashboard)/pos/receipts/page.tsx` | `/pos/receipts` | auth |

### 2.3 Customer App (`pos-customer`) - 2 routes

| Route Key | Source Path | URL Path (expected) | Access |
| --- | --- | --- | --- |
| CUS-001 | `src/app/page.tsx` | `/` | public |
| CUS-002 | `src/app/(logged-out)/self-register/page.tsx` | `/self-register` | public |

## 3) High-Risk Canonical Flows (Must Always Run)

### 3.1 Merchant Registration + OTP

- `REG-001` load `/signup`
- `REG-002` submit valid merchant signup with Terms checked
- `REG-003` retrieve OTP from `<workspace>:commons-api` logs for the same phone
- `REG-004` submit OTP and reach post-verification state
- `REG-005` negative OTP path (`0000` or stale OTP)
- `REG-006` submit with Terms unchecked must be blocked/validated

### 3.2 Merchant Login

- `LOG-001` login with phone digits only (`770805444`) and valid password
- `LOG-002` wrong password validation
- `LOG-003` non-existent account validation

### 3.3 Super Admin Login

- `SAL-001` login with `SuperAdmin` / `Alpha23@$`
- `SAL-002` if login fails, run `./scripts/ensure-superadmin.sh "SuperAdmin" "Alpha23@$"` and retry

### 3.4 Wrong-Page Guard

- `PG-001` never use `junk-qr-payments`; canonical route is `/digital-transactions/junk-qr`
- `PG-002` URL and heading must match before PASS

## 4) Route-Level Coverage Procedure (For Each Route Key)

For every route key in Section 2:

1. Open route (menu navigation preferred, direct URL allowed for unreachable menu).
2. Assert URL path and page heading identity.
3. Collect all visible actionable controls:
   - buttons (`button`, `[role="button"]`)
   - anchors that trigger page transitions
   - table row actions (view/edit/delete/junk/etc.)
   - dropdowns/comboboxes and filters
   - pagination controls
4. Execute each control path once.
5. For each form on that route:
   - submit valid data path
   - submit one invalid/empty path and confirm validation
6. Capture at least one stable screenshot after successful state load.
7. Capture failure screenshot for each failed assertion.

Mark route as complete only after steps 1-7 pass.

## 4.1) Button and Form Coverage IDs (Required)

For each route key, create additional test IDs in `manifest.json`:

- `ROUTE-<route_key>`: route load + page identity check
- `BTN-<route_key>-NN`: each unique clickable action on that route
- `FORM-<route_key>-NN-VALID`: successful form submit path
- `FORM-<route_key>-NN-INVALID`: validation/error path

Example:

- `ROUTE-WEB-023`
- `BTN-WEB-023-01` (open filter)
- `BTN-WEB-023-02` (click export)
- `FORM-WEB-023-01-VALID`
- `FORM-WEB-023-01-INVALID`

If a route has no form, no `FORM-*` IDs are required for that route.

## 5) Infra Failure Policy (Before BLOCKED)

If errors indicate infra wiring (ports, DB, gRPC, unavailable service):

1. Fix infra in env/tmux wiring only.
2. Restart affected services.
3. Re-run the same test IDs.
4. Keep app logic unchanged.

Set `BLOCKED` only if failure persists after at least one infra fix + retry cycle with logs.

## 6) Evidence Quality Policy

PASS evidence is invalid when screenshot/video indicates:

- `404` / `Not Found`
- loading spinner/skeleton-only view
- blank placeholder without loaded content
- app error screen
- wrong page heading/URL mismatch

When invalid:

1. Wait for stable UI and recapture.
2. If still invalid, mark test `FAIL`.

## 7) Run Bootstrap (Required Before Test Execution)

```bash
RUN_ID="$(date +%Y-%m-%d%H-%M-%S)"
RUN_DIR="logs/${RUN_ID}"
mkdir -p "${RUN_DIR}/screenshots" "${RUN_DIR}/video"

cat > "${RUN_DIR}/manifest.json" <<JSON
{
  "run_id": "${RUN_ID}",
  "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "status": "fail",
  "summary": { "total": 0, "passed": 0, "failed": 0, "skipped": 0 },
  "blocker": "run started - results pending",
  "tests": [],
  "screenshots": [],
  "video": { "path": "video/full-process.webm" }
}
JSON

cat > "${RUN_DIR}/index.html" <<HTML
<!doctype html>
<html><head><meta charset="utf-8"><title>Run ${RUN_ID}</title></head>
<body>
<h1>Regression Run ${RUN_ID}</h1>
<p>Status: in progress</p>
<p>Artifacts are being generated. Refresh after completion.</p>
</body></html>
HTML
```

## 8) Required `manifest.json` Fields

Each finished run must include:

- `summary.total`, `summary.passed`, `summary.failed`, `summary.skipped`
- `tests[]` entries with `id`, `name`, `status`, `error` (optional)
- `screenshots[]` entries with `test_id`, `path`, `description`
- `video.path` set to an actual file

Example per-test entry:

```json
{
  "id": "WEB-023",
  "name": "Merchant IPG payments route",
  "status": "pass"
}
```

Example screenshot entry:

```json
{
  "test_id": "WEB-023",
  "path": "screenshots/web-023-ipg-payments.png",
  "description": "Loaded route with table rows and filters"
}
```

## 8.1) Required `coverage.json`

Each run must include `logs/{run_id}/coverage.json` with this structure:

```json
{
  "route_total": 74,
  "route_covered": 74,
  "button_total": 0,
  "button_covered": 0,
  "form_total": 0,
  "form_covered": 0,
  "notes": "button/form totals are generated from runtime discovery"
}
```

Rules:

- `route_total` must match current inventory (`55 + 17 + 2` unless refreshed counts changed).
- `route_covered` must equal `route_total` for an exhaustive run.
- `button_covered <= button_total`, `form_covered <= form_total`.
- If `button_total` or `form_total` is `0`, add a note explaining discovery failure and mark run `FAIL`.

## 9) Artifact Validation Before Finalizing Run

```bash
RUN_ID="2026-03-2014-30-00"   # replace
export RUN_ID
RUN_DIR="logs/${RUN_ID}"

test -f "${RUN_DIR}/index.html"
test -f "${RUN_DIR}/manifest.json"
test -f "${RUN_DIR}/coverage.json"
test -d "${RUN_DIR}/screenshots"
test -f "${RUN_DIR}/video/full-process.webm" || test -f "${RUN_DIR}/video/full-process.mp4"

python3 - <<'PY'
import json, os, pathlib, sys

run_id = os.environ.get("RUN_ID", "").strip()
if not run_id:
    print("FAIL: RUN_ID missing")
    sys.exit(1)

run_dir = pathlib.Path("logs") / run_id
manifest = json.loads((run_dir / "manifest.json").read_text())
coverage = json.loads((run_dir / "coverage.json").read_text())
summary = manifest.get("summary", {})

total = int(summary.get("total", 0))
passed = int(summary.get("passed", 0))
failed = int(summary.get("failed", 0))
skipped = int(summary.get("skipped", 0))

errors = []
if total != passed + failed + skipped:
    errors.append("summary mismatch")

if total > 0 and len(manifest.get("tests", [])) == 0:
    errors.append("tests[] missing for non-empty run")

route_total = int(coverage.get("route_total", 0))
route_covered = int(coverage.get("route_covered", 0))
button_total = int(coverage.get("button_total", 0))
button_covered = int(coverage.get("button_covered", 0))
form_total = int(coverage.get("form_total", 0))
form_covered = int(coverage.get("form_covered", 0))

if route_total <= 0:
    errors.append("coverage.json route_total must be > 0")
if route_covered != route_total:
    errors.append("route coverage incomplete")
if button_covered > button_total:
    errors.append("button_covered cannot exceed button_total")
if form_covered > form_total:
    errors.append("form_covered cannot exceed form_total")
if button_total == 0 or form_total == 0:
    errors.append("button/form totals missing; runtime discovery did not run")

for shot in manifest.get("screenshots", []):
    p = (shot.get("path") or "").strip()
    if not p:
        errors.append("empty screenshot path")
        continue
    if not (run_dir / p).is_file():
        errors.append(f"missing screenshot file: {p}")

video_path = ((manifest.get("video") or {}).get("path") or "").strip()
if not video_path or not (run_dir / video_path).is_file():
    errors.append("video.path does not point to an existing file")

if errors:
    print("FAIL")
    for e in errors:
        print("-", e)
    sys.exit(1)

print("PASS: manifest + artifacts are consistent")
PY
```

## 10) Execution Notes

- Mobile format: use E.164 (`+94770805444`) where full number is needed.
- Merchant login input: type only digits after the prefilled `+94`.
- OTP source: only `<workspace-name>:commons-api` tmux window.
- Avoid over-reliance on direct URL navigation; include realistic menu-click journeys.
- If `PASS` rows exist with zero screenshots, run is invalid.

## 11) Completion Checklist

Mark run complete only when all are true:

- [ ] 74/74 routes covered (`55 + 17 + 2`)
- [ ] all critical auth flows executed
- [ ] all visible click paths exercised on each route
- [ ] all route forms tested with valid + invalid paths
- [ ] `coverage.json` confirms non-zero button/form totals and full route coverage
- [ ] screenshot quality gate passed
- [ ] playable full-process video present
- [ ] `manifest.json` includes `tests[]` and `screenshots[]`
- [ ] summary totals are internally consistent
