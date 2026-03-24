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
7. `coverage.json` records route/button/form/functional totals and covered counts.
8. `index.html` contains real video embedding/link output, not placeholder messaging.

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
- `LOG-004` post-login dashboard readiness (no auth redirect, dashboard heading visible, primary widgets loaded)

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
6. Wait for stable page state before evidence capture:
   - URL is final
   - expected heading is visible
   - loading indicators are gone (`loading`, `spinner`, `skeleton`, `shimmer`)
7. Capture at least one stable screenshot after successful state load.
8. Capture failure screenshot for each failed assertion.

Mark route as complete only after steps 1-8 pass.

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
- placeholder-only video section (`Video recording placeholder` or similar)

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
  "summary": { "total": 0, "passed": 0, "failed": 0, "skipped": 0, "blocked": 0 },
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
- `summary.blocked` (required; use 0 if none)
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
  "functional_total": 0,
  "functional_covered": 0,
  "notes": "button/form/functional totals are generated from runtime discovery"
}
```

Rules:

- `route_total` must match the current Section 2 inventory count at runtime.
- `route_covered` must equal `route_total` for an exhaustive run.
- `button_covered <= button_total`, `form_covered <= form_total`, `functional_covered <= functional_total`.
- `functional_total` must include all `FUNC-*` scenarios from Section 12.
- `functional_covered` must equal `functional_total` for an exhaustive run.
- If `button_total`, `form_total`, or `functional_total` is `0`, add a note explaining discovery failure and mark run `FAIL`.

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
blocked = int(summary.get("blocked", 0))

errors = []
if total != passed + failed + skipped + blocked:
    errors.append("summary mismatch")

if total > 0 and len(manifest.get("tests", [])) == 0:
    errors.append("tests[] missing for non-empty run")

route_total = int(coverage.get("route_total", 0))
route_covered = int(coverage.get("route_covered", 0))
button_total = int(coverage.get("button_total", 0))
button_covered = int(coverage.get("button_covered", 0))
form_total = int(coverage.get("form_total", 0))
form_covered = int(coverage.get("form_covered", 0))
functional_total = int(coverage.get("functional_total", 0))
functional_covered = int(coverage.get("functional_covered", 0))

if route_total <= 0:
    errors.append("coverage.json route_total must be > 0")
if route_covered != route_total:
    errors.append("route coverage incomplete")
if button_covered > button_total:
    errors.append("button_covered cannot exceed button_total")
if form_covered > form_total:
    errors.append("form_covered cannot exceed form_total")
if functional_covered > functional_total:
    errors.append("functional_covered cannot exceed functional_total")
if functional_covered != functional_total:
    errors.append("functional_covered must equal functional_total for exhaustive run")
if button_total == 0 or form_total == 0 or functional_total == 0:
    errors.append("button/form/functional totals missing; runtime discovery did not run")

func_manifest_count = sum(
    1
    for case in manifest.get("tests", [])
    if str(case.get("id") or "").strip().upper().startswith("FUNC-")
)
if func_manifest_count == 0:
    errors.append("manifest has no FUNC-* entries")
if functional_total > 0 and func_manifest_count != functional_total:
    errors.append(
        f"manifest FUNC-* count ({func_manifest_count}) does not match functional_total ({functional_total})"
    )

for shot in manifest.get("screenshots", []):
    p = (shot.get("path") or "").strip()
    if not p:
        errors.append("empty screenshot path")
        continue
    if not (run_dir / p).is_file():
        errors.append(f"missing screenshot file: {p}")

shot_counts = {}
for shot in manifest.get("screenshots", []):
    test_id = (shot.get("test_id") or "").strip()
    if test_id:
        key = test_id.lower().replace("_", "-")
        shot_counts[key] = shot_counts.get(key, 0) + 1

for case in manifest.get("tests", []):
    status = str(case.get("status") or "").strip().lower()
    case_id = (case.get("id") or "").strip()
    if status in {"fail", "blocked"} and case_id:
        key = case_id.lower().replace("_", "-")
        if shot_counts.get(key, 0) <= 0:
            errors.append(f"missing FAIL/BLOCKED screenshot evidence for {case_id}")

dashboard_auth_failures = 0
for case in manifest.get("tests", []):
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

video_path = ((manifest.get("video") or {}).get("path") or "").strip()
video_file = run_dir / video_path if video_path else None
if not video_path or not video_file or not video_file.is_file():
    errors.append("video.path does not point to an existing file")
elif video_file.stat().st_size <= 0:
    errors.append("video artifact is zero bytes")

index_text = (run_dir / "index.html").read_text(errors="ignore").lower()
if "video recording placeholder" in index_text:
    errors.append("index.html still contains video placeholder text")

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
- Every `FAIL` and `BLOCKED` test must have at least one screenshot evidence entry.
- Dashboard login flow is only valid when post-login screen is the dashboard (not login redirect/auth error).

## 11) Completion Checklist

Mark run complete only when all are true:

- [ ] all Section 2 routes covered (`route_covered == route_total`)
- [ ] all critical auth flows executed
- [ ] all visible click paths exercised on each route
- [ ] all route forms tested with valid + invalid paths
- [ ] `coverage.json` confirms non-zero button/form/functional totals and full route coverage
- [ ] screenshot quality gate passed
- [ ] playable full-process video present
- [ ] full-process video file is non-zero bytes
- [ ] `index.html` has no video placeholder text
- [ ] `manifest.json` includes `tests[]` and `screenshots[]`
- [ ] summary totals are internally consistent
- [ ] deep functional scenarios (Section 12) executed with CRUD verification
- [ ] cross-role flows tested (merchant submit → super admin approve/reject)
- [ ] form validation tested for both valid and invalid paths on every form

## 12) Deep Functional Test Scenarios (End-to-End Workflows)

Route-visit coverage is necessary but NOT sufficient. This section defines real user workflow tests that exercise CRUD operations, multi-step flows, cross-role interactions, and form validations. Every test ID below MUST appear in `manifest.json`.

### 12.1 Employee / User Management (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-EMP-001 | Add new employee | Navigate to `/employeelist` → click Add Employee → fill name, phone, role → submit | Employee appears in list with correct details |
| FUNC-EMP-002 | View employee details | From `/employeelist` → click on an employee row | Employee detail view shows correct info |
| FUNC-EMP-003 | Edit employee | From employee detail → click Edit → change name or role → save | Updated info persists in list |
| FUNC-EMP-004 | Delete/deactivate employee | From employee detail → click Delete/Deactivate → confirm | Employee removed or marked inactive in list |
| FUNC-EMP-005 | Add employee invalid data | Navigate to Add Employee → submit with empty required fields | Validation errors shown, form not submitted |
| FUNC-EMP-006 | Add employee duplicate phone | Add employee with phone already in use | Error message about duplicate phone |

### 12.2 Access List / Role Management (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-ACL-001 | View access list | Navigate to `/accesslist` | Access list page loads with role/permission entries |
| FUNC-ACL-002 | Toggle permission | Click a permission toggle for a role | Permission state changes and persists on reload |
| FUNC-ACL-003 | Save access changes | Modify permissions → click Save | Success toast/confirmation, changes persist |

### 12.3 Item Management (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-ITEM-001 | Add new item | Navigate to `/items` → click Add Item → fill name, price, category, SKU → submit | Item appears in `/items/itemlist` with correct details |
| FUNC-ITEM-002 | View item list | Navigate to `/items/itemlist` | Item list loads with previously added items visible |
| FUNC-ITEM-003 | Edit item | From item list → click Edit on an item → change price → save | Updated price visible in item list |
| FUNC-ITEM-004 | Delete item | From item list → click Delete on an item → confirm | Item removed from list |
| FUNC-ITEM-005 | Add item missing required fields | Submit Add Item form with empty name or price | Validation errors displayed, item not created |
| FUNC-ITEM-006 | Add item with image | Add item → upload product image → submit | Item created with image thumbnail visible in list |
| FUNC-ITEM-007 | Search/filter items | Use search bar or filter on item list page | List narrows to matching items |
| FUNC-ITEM-008 | Item pagination | If item list has multiple pages, navigate to page 2+ | Page loads with different items |

### 12.4 Category Management (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-CAT-001 | Add new category | Navigate to `/items/categories` → click Add → fill name → submit | Category appears in list |
| FUNC-CAT-002 | Edit category | Click Edit on existing category → change name → save | Updated name visible in list |
| FUNC-CAT-003 | Delete category | Click Delete on a category → confirm | Category removed from list |
| FUNC-CAT-004 | Add category empty name | Submit with empty name | Validation error shown |

### 12.5 Customer Management (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-CUST-001 | Add new customer | Navigate to `/customers` → click Add → fill name, phone, email → submit | Customer appears in list |
| FUNC-CUST-002 | View customer details | Click on a customer in list | Detail view shows correct info |
| FUNC-CUST-003 | Edit customer | Edit customer details → change phone → save | Updated phone visible |
| FUNC-CUST-004 | Delete customer | Delete customer → confirm | Customer removed from list |
| FUNC-CUST-005 | Add customer invalid phone | Submit with malformed phone number | Validation error displayed |
| FUNC-CUST-006 | Search customers | Use search in customer list | Results filtered correctly |

### 12.6 KYC Full Flow (Merchant ↔ Super Admin Cross-Role)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-KYC-001 | Open KYC form | Login as merchant → navigate to `/settings/kyc` | KYC form loads with required fields |
| FUNC-KYC-002 | Submit KYC (valid) | Fill all required KYC fields (business name, NIC, documents) → submit | Success message, KYC status changes to "pending" |
| FUNC-KYC-003 | Submit KYC missing fields | Submit KYC with missing required documents | Validation errors shown |
| FUNC-KYC-004 | View KYC status (merchant) | After submit, revisit `/settings/kyc` | Shows "pending" or "under review" status |
| FUNC-KYC-005 | Super admin views pending KYC | Login as super admin → navigate to `/merchants-management/kyc-approvals` | Pending KYC submissions listed |
| FUNC-KYC-006 | Super admin approves KYC | Click on pending KYC → review details → click Approve | KYC status changes to "approved" |
| FUNC-KYC-007 | Verify merchant KYC approved | Login as merchant → navigate to `/settings/kyc` | KYC status shows "approved" |
| FUNC-KYC-008 | Super admin rejects KYC | Submit new KYC → super admin clicks Reject with reason | KYC status changes to "rejected" with reason |
| FUNC-KYC-009 | Merchant sees rejection | Login as merchant → navigate to `/settings/kyc` | Shows "rejected" status with rejection reason |

### 12.7 Business Name Approval Flow (Merchant ↔ Super Admin Cross-Role)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-BNA-001 | View business name approvals | Super admin → navigate to `/business-name-approvals` | List of pending business name requests |
| FUNC-BNA-002 | Approve business name | Click approve on a pending request | Status changes to approved |
| FUNC-BNA-003 | Reject business name | Click reject on a pending request → provide reason | Status changes to rejected |

### 12.8 Transaction Flows (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-TXN-001 | View transactions list | Navigate to `/transactions` | Transaction list loads with entries or empty state |
| FUNC-TXN-002 | Filter transactions by date | Use date range filter on transactions page | List filters to selected range |
| FUNC-TXN-003 | View transaction detail | Click on a transaction row | Detail view shows amount, items, payment type, timestamp |
| FUNC-TXN-004 | View IPG payments | Navigate to `/transactions/ipg-payments` | IPG payment list loads |
| FUNC-TXN-005 | View QR payments | Navigate to `/transactions/qr-payments` | QR payment list loads |
| FUNC-TXN-006 | View void history | Navigate to `/transactions/void-history` | Void transaction list loads |
| FUNC-TXN-007 | Export transactions | Click export/download button on transactions page | File downloads (CSV/PDF) |

### 12.9 IPG / Payment Links (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-IPG-001 | View IPG dashboard | Navigate to `/ipg` | IPG overview page loads |
| FUNC-IPG-002 | Create payment link | Navigate to `/ipg/payment-links` → click Create → fill amount, description → submit | Payment link created and visible in list |
| FUNC-IPG-003 | Create payment link invalid | Submit with negative amount or empty fields | Validation error shown |
| FUNC-IPG-004 | Copy payment link | Click copy on an existing payment link | Link copied to clipboard (or copy action feedback) |

### 12.10 Reports (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-RPT-001 | Sales summary report | Navigate to `/report/sales-summary` → select date range → generate | Report data loads with totals |
| FUNC-RPT-002 | Sales by item report | Navigate to `/report/sales-item` → generate | Per-item breakdown shown |
| FUNC-RPT-003 | Sales by category report | Navigate to `/report/sales-category` → generate | Per-category breakdown shown |
| FUNC-RPT-004 | Sales by employee report | Navigate to `/report/sales-employee` → generate | Per-employee breakdown shown |
| FUNC-RPT-005 | Sales by modifier report | Navigate to `/report/sales-modifier` → generate | Modifier report shown |
| FUNC-RPT-006 | Sales by payment type | Navigate to `/report/sales-payment-type` → generate | Payment type breakdown shown |
| FUNC-RPT-007 | Receipts report | Navigate to `/report/receipts` | Receipt list loads |
| FUNC-RPT-008 | Stock report | Navigate to `/report/stock-report` | Stock levels shown |
| FUNC-RPT-009 | Shifts report | Navigate to `/report/shifts` | Shift entries listed |
| FUNC-RPT-010 | Tax report | Navigate to `/report/taxes` | Tax summary loads |
| FUNC-RPT-011 | Export report | Click export button on any report page | File downloads or export modal appears |

### 12.11 Billing & Wallet (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-BIL-001 | View billing overview | Navigate to `/billing/overview` | Balance, plan info, usage summary visible |
| FUNC-BIL-002 | Wallet transfer | Navigate to `/billing/wallet-transfer` → fill amount → submit | Transfer success message, balance updated |
| FUNC-BIL-003 | Wallet transfer invalid | Submit with zero/negative amount | Validation error shown |
| FUNC-BIL-004 | View wallet usage history | Navigate to `/billing/wallet-usage-history` | Usage history entries listed |
| FUNC-BIL-005 | View SMS usage history | Navigate to `/billing/sms-usage-history` | SMS usage entries listed |
| FUNC-BIL-006 | View SMS recharge history | Navigate to `/billing/sms-recharge-history` | Recharge history entries listed |

### 12.12 Marketing (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-MKT-001 | Send custom SMS | Navigate to `/marketing/custom-sms` → fill recipient + message → send | Success confirmation, SMS logged |
| FUNC-MKT-002 | Send SMS empty message | Submit with empty message body | Validation error shown |
| FUNC-MKT-003 | Send SMS invalid phone | Submit with invalid phone number | Validation error shown |

### 12.13 Settings (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-SET-001 | View account settings | Navigate to `/settings/account` | Account info loads (name, email, phone) |
| FUNC-SET-002 | Update account info | Change display name → save | Success toast, name persists on reload |
| FUNC-SET-003 | Update store settings | Navigate to `/settings/store` → change store name → save | Store name updated |
| FUNC-SET-004 | Update POS settings | Navigate to `/settings/pos` → toggle a setting → save | Setting persists |
| FUNC-SET-005 | Toggle features | Navigate to `/settings/features` → enable/disable a feature | Feature state changes and persists |
| FUNC-SET-006 | Manage payment types | Navigate to `/settings/paymenttypes` → add/toggle payment type | Payment type config updated |
| FUNC-SET-007 | E-bill template | Navigate to `/settings/e-billTemplate` → modify template → save | Template updated |
| FUNC-SET-008 | IPG branding | Navigate to `/settings/ipg-branding` → upload logo or change colors → save | Branding updated |
| FUNC-SET-009 | Support ticket | Navigate to `/settings/ticket` → create new ticket → submit | Ticket created with confirmation |
| FUNC-SET-010 | Billing settings | Navigate to `/settings/billing` | Billing settings page loads correctly |
| FUNC-SET-011 | Settings form invalid | Submit any settings form with invalid data (empty required field, bad email format) | Validation errors shown |

### 12.14 Timecard & Work Hours (Merchant App)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-TIME-001 | View timecard | Navigate to `/timecard` | Timecard interface loads |
| FUNC-TIME-002 | Clock in/out | Click clock-in button (if present) | Time entry recorded |
| FUNC-TIME-003 | View total work hours | Navigate to `/totalworkhours` | Work hours summary loads |

### 12.15 Merchant Registration Full Flow (End-to-End)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-REG-001 | Full signup → OTP → login | 1) Navigate `/signup` 2) Fill valid merchant data 3) Check Terms 4) Submit 5) Read OTP from commons-api logs 6) Enter OTP at `/signup-verification` 7) Login with new credentials | Dashboard loads after login |
| FUNC-REG-002 | Signup duplicate phone | Use phone number already registered | Error: phone already registered |
| FUNC-REG-003 | Signup → wrong OTP → retry | Submit wrong OTP → see error → submit correct OTP | Correct OTP succeeds |
| FUNC-REG-004 | Email verification flow | Navigate to `/verify-email` with valid token | Email verified successfully |

### 12.16 Password Reset Flow (End-to-End)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-PWD-001 | Request password reset | Navigate `/resetpswd` → enter phone → submit | OTP sent, redirect to verification |
| FUNC-PWD-002 | Verify reset OTP | Enter OTP at `/resetpswd-verification` | Redirects to new password form |
| FUNC-PWD-003 | Set new password | At `/newpswd` → enter new password + confirm → submit | Password updated, can login with new password |
| FUNC-PWD-004 | Reset with invalid phone | Enter non-existent phone number | Error message displayed |

### 12.17 Super Admin User & Role Management

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-SA-USR-001 | View admin users | Login as super admin → navigate to `/user-management/users` | User list loads |
| FUNC-SA-USR-002 | Add admin user | Click Add User → fill details → submit | User created and appears in list |
| FUNC-SA-USR-003 | Edit admin user | Click Edit on a user → change role → save | Updated role persists |
| FUNC-SA-USR-004 | Delete admin user | Click Delete on a user → confirm | User removed from list |
| FUNC-SA-USR-005 | View roles | Navigate to `/user-management/roles` | Role list loads |
| FUNC-SA-USR-006 | Create role | Click Add Role → fill name + permissions → submit | Role created |
| FUNC-SA-USR-007 | Edit role | Edit existing role → change permissions → save | Permissions updated |

### 12.18 Super Admin Merchant Management

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-SA-MER-001 | View merchants list | Navigate to `/merchants-management/merchants` | Merchant list loads with entries |
| FUNC-SA-MER-002 | View merchant detail | Click on a merchant row | Merchant detail page with store info, KYC status |
| FUNC-SA-MER-003 | Filter/search merchants | Use search or filter on merchant list | Results filtered correctly |

### 12.19 Super Admin Digital Transactions

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-SA-TXN-001 | View digital transactions | Navigate to `/digital-transactions` | Transaction list loads |
| FUNC-SA-TXN-002 | View IPG payments | Navigate to `/digital-transactions/ipg-payments` | IPG payment list loads |
| FUNC-SA-TXN-003 | View QR payments | Navigate to `/digital-transactions/qr-payments` | QR payment list loads |
| FUNC-SA-TXN-004 | View junk QR | Navigate to `/digital-transactions/junk-qr` | Junk QR entries listed |
| FUNC-SA-TXN-005 | View void history | Navigate to `/digital-transactions/void-history` | Void entries listed |
| FUNC-SA-TXN-006 | Filter transactions | Apply date/merchant filter | Results filtered correctly |

### 12.20 Super Admin Customers & Receipts

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-SA-CUST-001 | View admin customers | Navigate to `/customers` (super admin) | Customer list loads |
| FUNC-SA-CUST-002 | View POS receipts | Navigate to `/pos/receipts` | Receipt list loads |
| FUNC-SA-CUST-003 | View receipt detail | Click on a receipt row | Receipt detail shown |

### 12.21 Super Admin Password Reset

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-SA-PWD-001 | Request OTP | Navigate to `/password/otp` → enter admin email/phone → submit | OTP sent |
| FUNC-SA-PWD-002 | Update password | Navigate to `/password/update` → enter OTP + new password → submit | Password updated |

### 12.22 Customer App Flows

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-CUS-001 | Customer landing | Navigate to customer app root `/` | Landing page loads correctly |
| FUNC-CUS-002 | Self registration | Navigate to `/self-register` → fill name, phone, email → submit | Registration success message |
| FUNC-CUS-003 | Self register invalid | Submit with missing required fields | Validation errors shown |
| FUNC-CUS-004 | Self register duplicate | Register with existing phone | Error about duplicate registration |

### 12.23 E-Bill & SMS Recharge Status (Public)

| Test ID | Scenario | Steps | Expected Result |
| --- | --- | --- | --- |
| FUNC-EBILL-001 | View e-bill | Navigate to `/e-bill` with valid bill ID parameter | E-bill content loads |
| FUNC-EBILL-002 | View e-bill invalid | Navigate to `/e-bill` without parameters or invalid ID | Error or empty state shown |
| FUNC-SMS-001 | SMS recharge status | Navigate to `/sms-recharge-status` with valid params | Recharge status displayed |

## 13) Functional Test Execution Contract

### 13.1 Test Data Seeding

Before running deep functional tests:

1. Ensure at least one merchant account exists (run registration flow or use existing `+94770805444`).
2. Ensure super admin account exists (`SuperAdmin` / `Alpha23@$`). If not, run `./scripts/ensure-superadmin.sh "SuperAdmin" "Alpha23@$"`.
3. Ensure backend APIs are running and reachable (all ports from SETUP.md).

### 13.2 Test Ordering

Execute in dependency order:

1. **Infrastructure validation** — ports, DB, services running
2. **Auth flows** — Registration (Section 3.1), Login (3.2), Super Admin Login (3.3)
3. **CRUD operations** — Items (12.3), Categories (12.4), Customers (12.5), Employees (12.1)
4. **Cross-role flows** — KYC (12.6), Business Name Approvals (12.7)
5. **Transaction views** — Merchant (12.8) and Super Admin (12.19)
6. **Reports** — All report types (12.10)
7. **Settings** — All settings pages (12.13)
8. **Billing** — Wallet and SMS (12.11)
9. **Marketing** — SMS sending (12.12)
10. **Password flows** — Reset (12.16), Super Admin reset (12.21)
11. **Route-level sweep** — Remaining routes from Section 2 not covered by above

### 13.3 CRUD Verification Rules

For every Create/Update/Delete operation:

1. **Create**: After submit, navigate to list view and verify new entry exists with correct data.
2. **Update**: After save, reload page and verify changed fields persist.
3. **Delete**: After confirm, verify entry is removed from list view.
4. **Invalid**: Submit invalid data and verify form shows validation errors without creating/modifying data.

If list view is empty after an operation that should have produced data, mark test `FAIL`.

### 13.4 Cross-Role Verification Rules

For flows involving multiple user roles:

1. Perform action as Role A (e.g., merchant submits KYC).
2. **Log out** of Role A session.
3. **Log in** as Role B (e.g., super admin).
4. Verify the action from step 1 is visible (e.g., pending KYC in approvals list).
5. Perform Role B action (e.g., approve KYC).
6. **Log out** of Role B.
7. **Log in** as Role A.
8. Verify the Role B action result is visible (e.g., KYC status = approved).

Each step must have screenshot evidence.

### 13.5 Manifest Entries for Functional Tests

Each functional test ID (`FUNC-*`) MUST appear in `manifest.json` tests array:

```json
{
  "id": "FUNC-KYC-006",
  "name": "Super admin approves merchant KYC",
  "status": "pass",
  "screenshots": ["screenshots/func-kyc-006-approve-click.png", "screenshots/func-kyc-006-approved-status.png"]
}
```

Screenshot naming convention: `screenshots/func-{test-id-lowercase}-{description}.png`

### 13.6 Updated Coverage Contract

A run is NOT exhaustive unless:

- `coverage.json` includes `functional_total` and `functional_covered` counts
- `functional_covered == functional_total` for exhaustive runs
- All FUNC-* test IDs from Section 12 are present in `manifest.json`
- CRUD operations have before/after verification screenshots
- Cross-role flows have screenshots from both role perspectives
