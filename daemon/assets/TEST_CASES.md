# Marx POS - Comprehensive Test Template

**Template Version:** 1.0  
**Date Created:** March 20, 2026  
**System:** Marx POS (Merchant Portal + Admin Portal)

---

## Overview

This template defines all test cases for the Marx POS system.

**Run Folder Naming Convention (Required):**

- Create run folders under `logs/`
- Format: `logs/{YYYY-MM-DD}{HH-MM-SS}/` (e.g., `logs/2026-03-2014-30-00/`)
- Always include date AND time in the run folder name
- Each run folder must include:
  - `index.html` (primary report page)
  - `manifest.json` (metadata + summary)
  - `screenshots/` (PNG artifacts)
  - `video/` (MP4/WebM artifacts)

**Important Notes:**

- **Page name is `junk-qr`** (NOT `junk-qr-payments`)
- **NIC numbers for testing:** Any 12-digit number (e.g., 199123456789)
- **Passport images for testing:** Any image file (use `http://localhost:3000/marx.png`)
- **OTP for merchant signup/testing:** Retrieve from workspace tmux session window `commons-api` (not `kyc-test`)
- **Mobile number format:** Use E.164 without spaces (example: `+94770805444`)
- **Merchant login phone field:** `+94` is pre-typed in UI; enter only the remaining digits (example input: `770805444`)

## HTML Regression Reports

- Run-folder naming: `logs/{date}{time}/`
- Run-folder schema (minimum required):
  - `index.html` (primary HTML regression report page)
  - `manifest.json` (run metadata and artifact index)
  - `screenshots/` (all captured screenshots)
  - `video/` (full regression video recording)
- Artifacts definitions:
  - `index.html`
  - `manifest.json`
  - `screenshots/*.png`
  - `video/full-process.mp4` (or `video/full-process.webm`)
- Server-open behavior:
  - Reports are served through the existing reports route (no standalone server).
  - A visible Reports button should open the reports index and allow opening each run page.
- HTML surface requirements:
  - `index.html` must show run metadata from `manifest.json`.
  - `index.html` must show screenshot entries and a playable video section.
  - Use run-relative media paths only (`screenshots/<file>` and `video/<file>`), never `logs/{run_id}/...` in `src` attributes.
  - If screenshots or video are missing, render an explicit warning state in the page.
  - `video/` must include one full-process recording named `full-process.mp4` (or `full-process.webm`).
  - Video section must include both:
    - a `<video controls>` element using `video/full-process.mp4` or `video/full-process.webm`
    - a direct clickable file link (`<a href="video/full-process.webm">Open video file</a>`)
  - Do not use placeholder-only text like `Video file available in the run folder` without a clickable file link.
  - `manifest.json` must set `video.path` to a file path (`video/full-process.webm` or `.mp4`), never a directory.

## Test Environment

| Item                 | Value                                        |
| -------------------- | -------------------------------------------- |
| Merchant Portal      | `http://localhost:3000`                      |
| Admin Portal         | `http://localhost:3001`                      |
| Merchant Credentials | `+94770805444` / `Alpha23@$`                 |
| Merchant Login Input | `770805444` (because `+94` is prefilled)     |
| Admin Credentials    | `SuperAdmin` / `Alpha23@$`                   |
| Test Mobile Numbers  | `+94771234XXX` (E.164, no spaces)            |
| OTP Source           | tmux session `<workspace-name>` window `commons-api` logs |

### Access Preconditions (Must verify before any test)

1. **SuperAdmin login**
   - Username: `SuperAdmin`
   - Password: `Alpha23@$`
   - If this fails, run:
     ```bash
     ./scripts/ensure-superadmin.sh "SuperAdmin" "Alpha23@$"
     ```
   - Retry login after script completes.
   - If it still fails, record `BLOCKED` with exact login error and DB command output.

2. **Merchant login path**
   - Existing test merchant: `+94770805444` / `Alpha23@$`
   - Merchant login phone field already includes `+94`; type `770805444` (not `+94770805444`) in the input box.
   - If merchant login fails, run Merchant signup flow first (`/register`) and then continue tests with the created account.
   - For signup OTP, read the latest OTP from tmux session `<workspace-name>` window `commons-api` for the same phone number used in `RS-002`.

3. **Do not assume accounts exist**
   - Confirm account existence by successful login before running protected route tests.
   - If account creation is required, create it first and update the test data table in results.

---

## SECTION 1: MERCHANT PORTAL - ALL SECTIONS

### 1.1 Login Page (`/login`)

| Test ID | Test Case                       | Test Data                          | Expected Result                                 | Status |
| ------- | ------------------------------- | ---------------------------------- | ----------------------------------------------- | ------ |
| ML-001  | Load login page                 | -                                  | Page loads with email/phone and password fields | -      |
| ML-002  | Login with correct credentials  | `770805444` (`+94` prefilled) / `Alpha23@$`    | Redirects to dashboard                          | -      |
| ML-003  | Login with wrong password       | `770805444` (`+94` prefilled) / `WrongPass123` | Error message displayed                         | -      |
| ML-004  | Login with non-existent account | `779999999` (`+94` prefilled) / `Alpha23@$`    | Error: "User not found" or similar              | -      |
| ML-005  | Login with empty fields         | (empty) / (empty)                  | Validation error                                | -      |
| ML-006  | Login with partial phone        | `+9477` / `Alpha23@$`              | Validation error                                | -      |
| ML-007  | Logout from portal              | -                                  | Redirect to login page                          | -      |

### 1.2 Reports Section (`/report/sales-summary`)

| Test ID | Test Case             | Test Data                  | Expected Result                              | Status |
| ------- | --------------------- | -------------------------- | -------------------------------------------- | ------ |
| MR-001  | Load Reports page     | -                          | Page loads with date range, KPI cards, chart | -      |
| MR-002  | Change date range     | Start: Mar 01, End: Mar 20 | Data refreshes                               | -      |
| MR-003  | Click on KPI card     | -                          | Card is clickable/expandable                 | -      |
| MR-004  | View empty data state | (no data range)            | "Nothing to show" message                    | -      |

### 1.3 Items Section (`/items`)

| Test ID | Test Case               | Test Data                    | Expected Result            | Status |
| ------- | ----------------------- | ---------------------------- | -------------------------- | ------ |
| MI-001  | Load Items page         | -                            | 10 items displayed         | -      |
| MI-002  | Filter by Category      | Category: Cold Drinks        | Filtered list shown        | -      |
| MI-003  | Filter by Availability  | Availability: Available      | Filtered list shown        | -      |
| MI-004  | Sort items              | Sort By: Price (Low to High) | Items sorted               | -      |
| MI-005  | Search item             | Search: "Coffee"             | Matching items shown       | -      |
| MI-006  | Pagination              | Click page 2                 | Second page of items       | -      |
| MI-007  | Click "Add Item" button | -                            | Add item dialog/form opens | -      |
| MI-008  | Items > Categories      | Navigate to Categories       | 6 categories displayed     | -      |

### 1.4 Digital Transactions (`/digital-transactions/ipg-payments`)

| Test ID | Test Case         | Test Data               | Expected Result          | Status |
| ------- | ----------------- | ----------------------- | ------------------------ | ------ |
| MDT-001 | Load IPG Payments | -                       | 10 rows, all PENDING     | -      |
| MDT-002 | Filter by Status  | Status: Completed       | Filtered results         | -      |
| MDT-003 | Filter by Date    | Date range              | Filtered results         | -      |
| MDT-004 | QR Payments page  | Navigate to QR Payments | Page loads, filters work | -      |
| MDT-005 | Export button     | Click Export            | File downloads           | -      |

### 1.5 Customers (`/customers`)

| Test ID | Test Case           | Test Data       | Expected Result         | Status |
| ------- | ------------------- | --------------- | ----------------------- | ------ |
| MC-001  | Load Customers page | -               | Customer list displayed | -      |
| MC-002  | Search customer     | Search: "Views" | Matching customer shown | -      |
| MC-003  | Pagination          | Click next page | Next page loads         | -      |

### 1.6 Marketing (`/marketing/custom-sms`)

| Test ID | Test Case        | Test Data              | Expected Result      | Status |
| ------- | ---------------- | ---------------------- | -------------------- | ------ |
| MM-001  | Load SMS page    | -                      | 10 records displayed | -      |
| MM-002  | Filter by Type   | Type: Promotional      | Filtered results     | -      |
| MM-003  | Filter by Status | Status: Sent           | Filtered results     | -      |
| MM-004  | Pagination       | Navigate through pages | Correct page loads   | -      |

### 1.7 Billing Section

#### 1.7.1 Billing Overview (`/billing/overview`)

| Test ID | Test Case      | Test Data | Expected Result                    | Status |
| ------- | -------------- | --------- | ---------------------------------- | ------ |
| MB-001  | Load Overview  | -         | Balance, SMS Count, Cost displayed | -      |
| MB-002  | Click Recharge | -         | Recharge dialog opens              | -      |

#### 1.7.2 SMS Usage History (`/billing/sms-usage`)

| Test ID | Test Case      | Test Data | Expected Result         | Status |
| ------- | -------------- | --------- | ----------------------- | ------ |
| MBS-001 | Load SMS Usage | -         | BILL_PAYMENT rows shown | -      |
| MBS-002 | Pagination     | Navigate  | Correct page loads      | -      |

#### 1.7.3 Wallet Credit History (`/billing/credit-history`)

| Test ID | Test Case           | Test Data | Expected Result              | Status |
| ------- | ------------------- | --------- | ---------------------------- | ------ |
| MBC-001 | Load Credit History | -         | Empty table or history shown | -      |
| MBC-002 | Click Recharge      | -         | Recharge dialog opens        | -      |

#### 1.7.4 Wallet Debit History (`/billing/debit-history`)

| Test ID | Test Case          | Test Data | Expected Result          | Status |
| ------- | ------------------ | --------- | ------------------------ | ------ |
| MBD-001 | Load Debit History | -         | Debit transactions shown | -      |
| MBD-002 | Pagination         | Navigate  | Correct page loads       | -      |

#### 1.7.5 Wallet Transfer (`/billing/wallet-transfer`)

| Test ID | Test Case               | Test Data | Expected Result                  | Status |
| ------- | ----------------------- | --------- | -------------------------------- | ------ |
| MBT-001 | Load Transfer page      | -         | Initiate Transfer button visible | -      |
| MBT-002 | Click Initiate Transfer | -         | Transfer form opens              | -      |

### 1.8 Settings Section

#### 1.8.1 Payment Types (`/settings/payment-types`)

| Test ID | Test Case          | Test Data             | Expected Result              | Status |
| ------- | ------------------ | --------------------- | ---------------------------- | ------ |
| MS-001  | Load Payment Types | -                     | Cash, Card, Lanka QR toggles | -      |
| MS-002  | Toggle Cash OFF    | Click Cash toggle     | Cash turned OFF              | -      |
| MS-003  | Toggle Card ON     | Click Card toggle     | Card turned ON               | -      |
| MS-004  | Toggle Lanka QR ON | Click Lanka QR toggle | Lanka QR turned ON           | -      |

#### 1.8.2 E-Bill Template (`/settings/e-billTemplate`)

| Test ID | Test Case            | Test Data       | Expected Result                  | Status |
| ------- | -------------------- | --------------- | -------------------------------- | ------ |
| MSE-001 | Load E-Bill Template | -               | Logo upload, text fields visible | -      |
| MSE-002 | Fill Header Text 1   | "Test Business" | Text saved                       | -      |
| MSE-003 | Fill Header Text 2   | "Receipt"       | Text saved                       | -      |
| MSE-004 | Fill Footer Text     | "Thank you!"    | Text saved                       | -      |
| MSE-005 | Click Save           | -               | Changes saved successfully       | -      |
| MSE-006 | Upload logo          | Image file      | Logo uploaded                    | -      |

---

## SECTION 2: ADMIN PORTAL - ALL SECTIONS

### 2.1 Dashboard (`/dashboard`)

| Test ID | Test Case                     | Test Data | Expected Result       | Status |
| ------- | ----------------------------- | --------- | --------------------- | ------ |
| AD-001  | Load Dashboard                | -         | 4 KPI cards displayed | -      |
| AD-002  | View KPI: IPG Transactions    | -         | Value shown           | -      |
| AD-003  | View KPI: Lanka QR            | -         | Value shown           | -      |
| AD-004  | View KPI: Number of Merchants | -         | Value: 133            | -      |
| AD-005  | View KPI: Pending KYC         | -         | Value: 15             | -      |

### 2.2 Digital Transactions

#### 2.2.1 IPG Payments (`/digital-transactions/ipg-payments`)

| Test ID | Test Case                | Test Data       | Expected Result      | Status |
| ------- | ------------------------ | --------------- | -------------------- | ------ |
| ADI-001 | Load IPG Payments        | -               | 10 rows with filters | -      |
| ADI-002 | Filter by Date Range     | Mar 01 - Mar 20 | Filtered results     | -      |
| ADI-003 | Search by Merchant RID   | MX0000176       | Merchant filtered    | -      |
| ADI-004 | Filter by Status         | Status: PENDING | Filtered             | -      |
| ADI-005 | Filter by Payment Method | Method: Card    | Filtered             | -      |
| ADI-006 | Export button            | Click Export    | File downloads       | -      |
| ADI-007 | Pagination               | Navigate pages  | Correct page loads   | -      |

#### 2.2.2 QR Payments (`/digital-transactions/qr-payments`)

| Test ID | Test Case        | Test Data  | Expected Result           | Status |
| ------- | ---------------- | ---------- | ------------------------- | ------ |
| ADQ-001 | Load QR Payments | -          | Filters and table visible | -      |
| ADQ-002 | Apply filters    | Date range | Filtered results          | -      |
| ADQ-003 | Empty state      | (no data)  | "Nothing to show"         | -      |

#### 2.2.3 Junk QR Payments (`/digital-transactions/junk-qr`)

| Test ID | Test Case                   | Test Data          | Expected Result                                                | Status |
| ------- | --------------------------- | ------------------ | -------------------------------------------------------------- | ------ |
| ADJ-001 | Load Junk QR page           | -                  | Page loads with heading "Junk QR Payments"                     | -      |
| ADJ-002 | View filters                | -                  | Date range, Merchant RID, Merchant dropdown, Resolution Status | -      |
| ADJ-003 | Filter by Date Range        | Mar 01 - Mar 20    | Data filtered                                                  | -      |
| ADJ-004 | Search by Merchant RID      | Valid RID          | Data filtered                                                  | -      |
| ADJ-005 | Filter by Merchant          | Select merchant    | Data filtered                                                  | -      |
| ADJ-006 | Filter by Resolution Status | Status: Pending    | Data filtered                                                  | -      |
| ADJ-007 | View empty state            | (no matching data) | "Nothing to show here"                                         | -      |
| ADJ-008 | Pagination                  | Navigate           | Correct page loads                                             | -      |

### 2.3 Merchants Management

#### 2.3.1 Merchants (`/merchants-management/merchants`)

| Test ID | Test Case                | Test Data                        | Expected Result         | Status |
| ------- | ------------------------ | -------------------------------- | ----------------------- | ------ |
| AMM-001 | Load Merchants           | -                                | 10 rows displayed       | -      |
| AMM-002 | Search by name           | "Lunestra"                       | Matching merchant shown | -      |
| AMM-003 | Filter by KYC Status     | Status: Approved                 | Filtered                | -      |
| AMM-004 | Filter by IPG Status     | Status: Active                   | Filtered                | -      |
| AMM-005 | Filter by QR Status      | Status: Inactive                 | Filtered                | -      |
| AMM-006 | Filter by Deleted Status | Include Deleted                  | Shows deleted merchants | -      |
| AMM-007 | Filter by Is Junk        | Is Junk: Yes                     | Shows junk merchants    | -      |
| AMM-008 | Pagination               | Navigate to page 5               | Page 5 loads            | -      |
| AMM-009 | Mark merchant as Junk    | Click Junk icon on row           | Junk modal opens        | -      |
| AMM-010 | Confirm mark as Junk     | Enter note, confirm              | Merchant marked as junk | -      |
| AMM-011 | Remove from Junk         | Click Junk icon on junk merchant | Junk modal opens        | -      |
| AMM-012 | View merchant details    | Click view/eye icon              | Detail dialog opens     | -      |

#### 2.3.2 KYC Approvals (`/merchants-management/kyc-approvals`)

| Test ID | Test Case                            | Test Data                   | Expected Result                                               | Status |
| ------- | ------------------------------------ | --------------------------- | ------------------------------------------------------------- | ------ |
| AMK-001 | Load KYC Approvals                   | -                           | Table with Draft/Submitted/Approved statuses                  | -      |
| AMK-002 | Search merchant                      | "theceylontreasurehunt"     | Found                                                         | -      |
| AMK-003 | Filter by Status                     | Status: Submitted           | Only submitted shown                                          | -      |
| AMK-004 | Click view action                    | Click eye icon on row       | KYC detail dialog opens                                       | -      |
| AMK-005 | View KYC details                     | -                           | All sections visible: Business, Address, Director, Bank, Docs | -      |
| AMK-006 | Approve without note                 | Click Approve               | Error: "Please enter review note"                             | -      |
| AMK-007 | Approve with note                    | Note: "Approved"            | Status changes to Approved                                    | -      |
| AMK-008 | View approved status                 | -                           | Row shows "Approved"                                          | -      |
| AMK-009 | Reject without note                  | Click Reject, no note       | Error: "Please enter review note"                             | -      |
| AMK-010 | Reject with note                     | Note: "Missing documents"   | Status changes to Rejected                                    | -      |
| AMK-011 | View rejection notification          | -                           | Rejection notification shown                                  | -      |
| AMK-012 | Approve second submitted application | Click Approve on second row | Status changes to Approved                                    | -      |
| AMK-013 | View all KYC statuses                | Check table                 | Draft/Submitted/Approved/Rejected visible                     | -      |

### 2.4 User Management

#### 2.4.1 Users (`/user-management/users`)

| Test ID | Test Case               | Test Data   | Expected Result        | Status |
| ------- | ----------------------- | ----------- | ---------------------- | ------ |
| AMU-001 | Load Users              | -           | User list with roles   | -      |
| AMU-002 | Search user             | "testadmin" | User found             | -      |
| AMU-003 | Click Create User       | -           | Create user form opens | -      |
| AMU-004 | Filter by Active Status | Active: Yes | Filtered               | -      |

#### 2.4.2 Roles (`/user-management/roles`)

| Test ID | Test Case         | Test Data | Expected Result        | Status |
| ------- | ----------------- | --------- | ---------------------- | ------ |
| AMR-001 | Load Roles        | -         | Roles list displayed   | -      |
| AMR-002 | Click Create Role | -         | Create role form opens | -      |
| AMR-003 | Search role       | "test"    | Role found             | -      |

### 2.5 Customers (`/customers`)

| Test ID | Test Case      | Test Data    | Expected Result              | Status |
| ------- | -------------- | ------------ | ---------------------------- | ------ |
| AC-001  | Load Customers | -            | Empty table or customer list | -      |
| AC-002  | Export button  | Click Export | File downloads               | -      |

### 2.6 Business Name Approvals (`/business-name-approvals`)

| Test ID | Test Case        | Test Data       | Expected Result                  | Status |
| ------- | ---------------- | --------------- | -------------------------------- | ------ |
| ABN-001 | Load page        | -               | Status dropdown, search, columns | -      |
| ABN-002 | Filter by Status | Status: Pending | Filtered                         | -      |
| ABN-003 | Empty state      | (no pending)    | "Nothing to show"                | -      |

### 2.7 Settings

#### 2.7.1 Update Password (`/password/otp`)

| Test ID | Test Case             | Test Data | Expected Result         | Status |
| ------- | --------------------- | --------- | ----------------------- | ------ |
| ASP-001 | Load Password page    | -         | OTP verification form   | -      |
| ASP-002 | View pre-filled email | -         | Email displayed         | -      |
| ASP-003 | Click Send OTP        | -         | OTP sent to email/phone | -      |
| ASP-004 | Click Go Back         | -         | Navigates back          | -      |

---

## SECTION 3: REGISTRATION FLOW

### 3.1 Signup Form

| Test ID | Test Case                   | Test Data                               | Expected Result                   | Status |
| ------- | --------------------------- | --------------------------------------- | --------------------------------- | ------ |
| RS-001  | Load Signup page            | -                                       | Signup form loads                 | -      |
| RS-002  | Signup with valid data      | Phone: +9477XXXXXXX, Email, Password | User created, OTP sent            | -      |
| RS-003  | Signup with existing phone  | Existing: +94770805444               | Error: "User already exists"      | -      |
| RS-004  | Signup with invalid email   | email: "notanemail"                     | Validation error                  | -      |
| RS-005  | Signup with weak password   | password: "123"                         | Validation: password requirements | -      |
| RS-006  | Signup with empty fields    | (empty)                                 | Validation errors                 | -      |
| RS-007  | Signup without phone prefix | Phone: 771234567                        | Should add +94 prefix             | -      |

### 3.2 OTP Verification

| Test ID | Test Case                   | Test Data     | Expected Result                 | Status |
| ------- | --------------------------- | ------------- | ------------------------------- | ------ |
| RO-001  | Enter correct OTP           | Latest OTP from `commons-api` tmux logs for same signup phone | Verified, redirect to dashboard | -      |
| RO-002  | Enter wrong OTP             | OTP: 0000     | Error: "Invalid OTP"            | -      |
| RO-003  | Enter expired OTP           | (old OTP)     | Error: "OTP expired"            | -      |
| RO-004  | Enter empty OTP             | (empty)       | Validation error                | -      |
| RO-005  | Resend OTP                  | Click Resend  | New OTP sent                    | -      |
| RO-006  | Multiple wrong OTP attempts | 3x wrong OTP  | Account locked or cooldown      | -      |

### 3.3 KYC Application

#### 3.3.1 KYC Step 1 - Business Details

| Test ID | Test Case                           | Test Data                    | Expected Result               | Status |
| ------- | ----------------------------------- | ---------------------------- | ----------------------------- | ------ |
| RK1-001 | Select Business Type                | Type: Sole Proprietor        | Selected                      | -      |
| RK1-002 | Select Business Type                | Type: Private Company        | Selected                      | -      |
| RK1-003 | Select Business Type                | Type: Partnership            | Selected                      | -      |
| RK1-004 | Select Business Type                | Type: Public Company         | Selected                      | -      |
| RK1-005 | Select Business Category            | Category: Retail             | Selected                      | -      |
| RK1-006 | Select Business Category            | Category: Food & Beverage    | Selected                      | -      |
| RK1-007 | Select Business Category            | Category: Services           | Selected                      | -      |
| RK1-008 | Select Business Sub Category        | Sub Category: Clothing       | Selected                      | -      |
| RK1-009 | Enter Registered Business Name      | "Test Business ABC"          | Text entered                  | -      |
| RK1-010 | Enter Trading Name                  | "Test Shop"                  | Text entered                  | -      |
| RK1-011 | Enter Incorporation Date            | Date: 01-01-2020             | Date selected                 | -      |
| RK1-012 | Enter Commencement Date             | Date: 01-01-2020             | Date selected                 | -      |
| RK1-013 | Enter Phone Number                  | Phone: 771234567             | Phone entered                 | -      |
| RK1-014 | Leave required field empty          | Business Name: (empty)       | Validation error              | -      |
| RK1-015 | Leave Business Type empty           | Type: (not selected)         | Validation error              | -      |
| RK1-016 | Leave Category empty                | Category: (not selected)     | Validation error              | -      |
| RK1-017 | Leave Sub Category empty            | Sub Category: (not selected) | Validation error              | -      |
| RK1-018 | Click Next without filling required | -                            | Stays on Step 1, errors shown | -      |
| RK1-019 | Click Next                          | All fields filled            | Proceeds to Step 2            | -      |

#### 3.3.2 KYC Step 2 - Business Address

| Test ID | Test Case                           | Test Data         | Expected Result                 | Status |
| ------- | ----------------------------------- | ----------------- | ------------------------------- | ------ |
| RK2-001 | Enter Street                        | "123 Main Street" | Text entered                    | -      |
| RK2-002 | Select State                        | State: Western    | Selected                        | -      |
| RK2-003 | Enter District                      | District: Colombo | Text entered                    | -      |
| RK2-004 | Enter Town/City                     | Town: Colombo 15  | Text entered                    | -      |
| RK2-005 | Physical Address same as registered | Toggle: Yes       | Same as registered              | -      |
| RK2-006 | Physical Address different          | Toggle: No        | Different address fields appear | -      |
| RK2-007 | Leave required field empty          | Street: (empty)   | Validation error                | -      |
| RK2-008 | Click Next                          | -                 | Proceeds to Step 3              | -      |

#### 3.3.3 KYC Step 3 - Directors/Partners

| Test ID | Test Case                        | Test Data                         | Expected Result          | Status |
| ------- | -------------------------------- | --------------------------------- | ------------------------ | ------ |
| RK3-001 | Click Add button                 | -                                 | Director dialog opens    | -      |
| RK3-002 | Enter Full Name                  | "Test Director"                   | Text entered             | -      |
| RK3-003 | Enter Address                    | "123 Main St"                     | Text entered             | -      |
| RK3-004 | Enter NIC (12 digits)            | NIC: 199123456789                 | Valid 12-digit NIC       | -      |
| RK3-005 | Enter Passport instead of NIC    | Passport: AB1234567, NIC: (empty) | Passport accepted        | -      |
| RK3-006 | Enter Contact Number             | Phone: 771234567                  | Text entered             | -      |
| RK3-007 | Select PEP: No                   | PEP: No                           | Selected                 | -      |
| RK3-008 | Select PEP: Yes                  | PEP: Yes                          | Selected                 | -      |
| RK3-009 | Upload NIC Front                 | Fake image (from localhost)       | File selected            | -      |
| RK3-010 | Upload NIC Back                  | Fake image (from localhost)       | File selected            | -      |
| RK3-011 | Upload Passport Bio              | Fake image (from localhost)       | File selected            | -      |
| RK3-012 | Leave NIC empty without Passport | NIC: (empty), Passport: (empty)   | Validation error         | -      |
| RK3-013 | Click Save                       | -                                 | Director added to table  | -      |
| RK3-014 | Click Next                       | -                                 | Proceeds to submission   | -      |
| RK3-015 | Submit KYC                       | -                                 | KYC submitted for review | -      |

### 3.4 Validation Tests - All Forms

| Test ID | Test Case                  | Field          | Invalid Value               | Expected Result               | Status |
| ------- | -------------------------- | -------------- | --------------------------- | ----------------------------- | ------ |
| VAL-001 | Invalid NIC format         | NIC            | 12345 (too short)           | Validation error              | -      |
| VAL-002 | Invalid phone number       | Phone          | 123                         | Validation error              | -      |
| VAL-003 | Future incorporation date  | Date           | 01-01-2099                  | Warning or error              | -      |
| VAL-004 | Special characters in name | Business Name  | "Test @#$%"                 | Validation or sanitized       | -      |
| VAL-005 | SQL injection attempt      | Business Name  | "'; DROP TABLE--"           | Handled safely                | -      |
| VAL-006 | XSS attempt                | Address        | "<script>alert(1)</script>" | Handled safely                | -      |
| VAL-007 | Very long input            | Business Name  | 500+ characters             | Validation error or truncated | -      |
| VAL-008 | Leading/trailing spaces    | Business Name  | " Test "                    | Trimmed or saved with spaces  | -      |
| VAL-009 | Special characters in NIC  | NIC            | 1991###456789               | Validation error              | -      |
| VAL-010 | Negative date              | Date           | -1 year                     | Validation error              | -      |
| VAL-011 | Duplicate director         | Same NIC twice | Add same director twice     | Warning or error              | -      |

### 3.5 KYC Status After Admin Action (Merchant Portal)

| Test ID | Test Case                       | Test Data                    | Expected Result                   | Status |
| ------- | ------------------------------- | ---------------------------- | --------------------------------- | ------ |
| RKA-001 | View KYC status after approval  | Login as approved merchant   | KYC status shows "Approved"       | -      |
| RKA-002 | View KYC status after rejection | Login as rejected merchant   | KYC status shows "Rejected"       | -      |
| RKA-003 | View rejection reason           | Check rejection notification | Rejection reason displayed        | -      |
| RKA-004 | Resubmit rejected KYC           | Click Edit/Resubmit          | KYC form opens with previous data | -      |
| RKA-005 | View KYC status during review   | Login while KYC under review | KYC status shows "Submitted"      | -      |

---

## SECTION 4: ERROR STATES AND EDGE CASES

### 4.1 Network Errors

| Test ID | Test Case           | Scenario           | Expected Result                 | Status |
| ------- | ------------------- | ------------------ | ------------------------------- | ------ |
| EN-001  | Offline signup      | Disconnect network | Error: "No internet connection" | -      |
| EN-002  | OTP request timeout | Slow network       | Loading state, then error       | -      |
| EN-003  | Session timeout     | Leave page idle    | Redirect to login               | -      |
| EN-004  | API error 500       | Server error       | Error: "Something went wrong"   | -      |
| EN-005  | API error 401       | Unauthorized       | Redirect to login               | -      |
| EN-006  | API error 403       | Forbidden          | Error: "Access denied"          | -      |
| EN-007  | API error 404       | Not found          | Error: "Resource not found"     | -      |

### 4.2 Browser/UI Edge Cases

| Test ID | Test Case                 | Scenario                            | Expected Result                | Status |
| ------- | ------------------------- | ----------------------------------- | ------------------------------ | ------ |
| EU-001  | Back button during signup | Click browser back                  | Correct state maintained       | -      |
| EU-002  | Refresh during KYC Step 1 | F5 during form                      | Data should persist or warning | -      |
| EU-003  | Refresh during KYC Step 2 | F5 during form                      | Data should persist or warning | -      |
| EU-004  | Refresh during KYC Step 3 | F5 during form                      | Data should persist or warning | -      |
| EU-005  | Multiple tabs             | Open portal in 2 tabs               | Session handled correctly      | -      |
| EU-006  | Resize window small       | Mobile width                        | Responsive layout              | -      |
| EU-007  | Very long business name   | 500 characters                      | Truncated or validation error  | -      |
| EU-008  | Paste into disabled field | -                                   | Field remains disabled         | -      |
| EU-009  | Double-click submit       | Click Next twice rapidly            | Only one action processed      | -      |
| EU-010  | Navigate away during form | Click other menu item               | Warning about unsaved changes  | -      |
| EU-011  | Browser back on OTP       | Click back on OTP page              | Correct state                  | -      |
| EU-012  | Direct URL to KYC         | Navigate to /kyc when not logged in | Redirect to login              | -      |

### 4.3 Security Tests

| Test ID | Test Case                           | Method                                         | Expected Result     | Status |
| ------- | ----------------------------------- | ---------------------------------------------- | ------------------- | ------ |
| SEC-001 | Direct URL access to protected page | Navigate to /settings without login            | Redirect to login   | -      |
| SEC-002 | Admin URL as merchant               | Navigate to /admin while logged in as merchant | Access denied       | -      |
| SEC-003 | JWT token manipulation              | Change token in storage                        | Rejected by server  | -      |
| SEC-004 | CSRF token missing                  | Submit form without token                      | Request rejected    | -      |
| SEC-005 | Rate limiting                       | Rapid OTP requests                             | Blocked after limit | -      |

---

## SECTION 5: TEST DATA

### 5.1 Fake NIC Numbers (12 digits each)

| ID      | NIC Number   | Notes            |
| ------- | ------------ | ---------------- |
| NIC-001 | 199123456789 | Generic 12-digit |
| NIC-002 | 198765432109 | Generic 12-digit |
| NIC-003 | 200112345678 | Generic 12-digit |

### 5.2 Fake Passport Numbers

| ID      | Passport  | Notes                        |
| ------- | --------- | ---------------------------- |
| PAS-001 | AB1234567 | Format: 2 letters + 7 digits |
| PAS-002 | XY9876543 | Format: 2 letters + 7 digits |

### 5.3 Test Images (Available via localhost)

| Image          | URL                                       | Notes     |
| -------------- | ----------------------------------------- | --------- |
| Marx Logo      | `http://localhost:3000/marx.png`          | Available |
| Marx Rectangle | `http://localhost:3000/marxRectangle.png` | Available |

### 5.4 Test Mobile Numbers

| ID     | Phone           | Use Case        |
| ------ | --------------- | --------------- |
| TM-001 | +94771234567 | New signup      |
| TM-002 | +94771234568 | Duplicate test  |
| TM-003 | +94771234569 | Validation test |

### 5.5 Test Email Addresses

| ID     | Email            | Use Case       |
| ------ | ---------------- | -------------- |
| TE-001 | test001@test.com | New signup     |
| TE-002 | test002@test.com | Duplicate test |

---

## EXECUTION NOTES

**IMPORTANT: Create logs directory before testing**

```bash
mkdir -p logs
```

**tmux isolation requirement:**

- Use tmux session `<workspace-name>` for API/frontend windows.
- Do not create service windows in `uwu-main`.

0. **Run bootstrap (must happen before test execution)**

   ```bash
   RUN_ID="$(date +%Y-%m-%d%H-%M-%S)"
   RUN_DIR="logs/${RUN_ID}"
   mkdir -p "${RUN_DIR}/screenshots" "${RUN_DIR}/video"

   cat > "${RUN_DIR}/manifest.json" <<JSON
   {
     "run_id": "${RUN_ID}",
     "created_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
     "status": "fail",
     "summary": {"total": 0, "passed": 0, "failed": 0, "skipped": 0},
     "blocker": "run started - results pending",
     "screenshots": [],
     "video": {"path": "video/full-process.webm"}
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

   If tests abort, update the same files with final `fail` status and blocker details.

1. **OTP Retrieval (Merchant Signup)**

   Use workspace tmux session + `commons-api` window logs (do not use `kyc-test`):

   ```bash
   SESSION_NAME="${MYAPP_TMUX_SESSION_NAME:-$(basename "$PWD")}"
   PHONE_E164="+94771234567"   # replace with the signup phone used in RS-002
   tmux capture-pane -pt "${SESSION_NAME}:commons-api.0" -S -500 | grep -E "New Sandbox mode SMS|Your OTP is|OTP" | grep -F "${PHONE_E164}" | tail -n 30
   ```

   Match OTP to the same phone number used in `RS-002` signup input.
   If OTP for the target phone is missing, mark signup/OTP tests as `FAIL` (not `PASS`).

2. **Playwright + headless browser prerequisites (required for screenshots/video)**

   ```bash
   bun add -d playwright
   bunx playwright install --with-deps chromium
   ```

   Verification:

   ```bash
   bunx playwright --version
   ```

3. **File Upload for NIC/Passport**: Use `browser_evaluate` with DataTransfer API:

   ```javascript
   async () => {
     const response = await fetch("http://localhost:3000/marx.png");
     const blob = await response.blob();
     const file = new File([blob], "nic_front.png", { type: "image/png" });
     const dt = new DataTransfer();
     dt.items.add(file);
     const fi = document.querySelectorAll("input[type=file]")[0];
     fi.files = dt.files;
     fi.dispatchEvent(new Event("change", { bubbles: true }));
     return file.name;
   };
   ```

4. **Form Field Filling**: Use `browser_fill_form` with format:

   ```javascript
   {"fields": [{"name": "Field Label", "ref": "e123", "type": "textbox", "value": "input value"}]}
   ```

5. **Combobox Selection**: Click the combobox, wait for options, click option by text.

6. **Date Fields**: Use format DD-MM-YYYY (e.g., "01-01-2020")

7. **Mandatory media capture policy**
   - Capture screenshots for every failed assertion and every major workflow checkpoint.
   - Record at least one end-to-end video per test run.
   - If `screenshots/` or `video/` is missing, mark run as `FAIL` (not partial) and include remediation commands.
   - A test marked `PASS` must have at least one valid screenshot that matches the expected page state.
   - If a screenshot shows `404`, `Not Found`, `This page could not be found`, or any app error screen, that test must be `FAIL`.
   - If a screenshot only shows loading UI (spinner, skeleton, blank placeholder, or progress-only state), that test cannot be `PASS`.
   - Re-capture after waiting for stable UI. If stable UI never appears, mark the test `FAIL` and record timeout/root cause.
   - Before finalizing results, open each screenshot referenced by `PASS` rows and verify it is not an error page.

8. **Playwright capture baseline (must follow)**

   ```typescript
   const run = `logs/${runId}`;
   await fs.promises.mkdir(`${run}/screenshots`, { recursive: true });
   await fs.promises.mkdir(`${run}/video`, { recursive: true });

   const browser = await chromium.launch({ headless: true });
   const context = await browser.newContext({
     recordVideo: { dir: `${run}/video` },
   });
   const page = await context.newPage();

   // checkpoint screenshot example
   await page.screenshot({
     path: `${run}/screenshots/login.png`,
     fullPage: true,
   });

   // readiness guard example for non-loading screenshots
   await page.waitForLoadState("networkidle");
   await page.waitForTimeout(500);

   await context.close();
   await browser.close();
   ```

---

## RESULTS OUTPUT FORMAT

When completing tests, create a run folder at:

`logs/{YYYY-MM-DD}{HH-MM-SS}/`

Required files:

- `logs/{run_id}/index.html`
- `logs/{run_id}/manifest.json`
- `logs/{run_id}/screenshots/*.png`
- `logs/{run_id}/video/full-process.mp4` (or `full-process.webm`)

Required HTML behavior in `index.html`:

- Every screenshot card must include a clickable file link and an image preview (`<a href="screenshots/...">` + `<img src="screenshots/...">`).
- Video section must include a playable `<video controls>` element.
- Video section must include a direct clickable file link to `video/full-process.webm` or `video/full-process.mp4`.
- Do not use `logs/{run_id}/...` prefixes in screenshot/video URLs inside HTML.

Required summary fields in `manifest.json`:

```json
{
  "run_id": "2026-03-2014-30-00",
  "created_at": "2026-03-20T14:30:00Z",
  "status": "pass",
  "summary": {
    "total": 100,
    "passed": 95,
    "failed": 5,
    "skipped": 0
  },
  "video": {
    "path": "video/full-process.webm"
  }
}
```

Required integrity checks before marking the run complete:

```bash
# Replace with actual run ID first
RUN_ID="2026-03-2014-30-00"
export RUN_ID
RUN_DIR="logs/${RUN_ID}"

test -f "${RUN_DIR}/index.html"
test -f "${RUN_DIR}/manifest.json"
test -d "${RUN_DIR}/screenshots"
test -f "${RUN_DIR}/video/full-process.webm" || test -f "${RUN_DIR}/video/full-process.mp4"

python3 - <<'PY'
import json, os, pathlib, sys

run_id = os.environ.get("RUN_ID", "").strip()
if not run_id:
    print("FAIL")
    print("- RUN_ID is required")
    sys.exit(1)

run = pathlib.Path("logs") / run_id
manifest = json.loads((run / "manifest.json").read_text())

errors = []

summary = manifest.get("summary", {})
total = int(summary.get("total", 0))
passed = int(summary.get("passed", 0))
failed = int(summary.get("failed", 0))
skipped = int(summary.get("skipped", 0))
if total != passed + failed + skipped:
    errors.append("summary mismatch: total != passed + failed + skipped")

status = str(manifest.get("status", "")).strip().lower()
if status == "pass" and failed > 0:
    errors.append("status is pass but failed > 0")
if status == "pass" and manifest.get("blocker"):
    errors.append("status is pass but blocker is present")
if status == "fail" and failed == 0:
    errors.append("status is fail but failed == 0 (mark at least one failed test)")

for item in manifest.get("screenshots", []):
    path = str(item.get("path", "")).strip()
    if path.startswith(f"logs/{run_id}/"):
        path = path[len(f"logs/{run_id}/"):]
    file_path = run / path
    if not file_path.is_file():
        errors.append(f"missing screenshot file: {item.get('path')}")

video_path = str((manifest.get("video") or {}).get("path", "")).strip()
if video_path.startswith(f"logs/{run_id}/"):
    video_path = video_path[len(f"logs/{run_id}/"):]
resolved_video = run / video_path
if not resolved_video.is_file():
    errors.append(f"video.path must point to an existing file, got: {manifest.get('video')}")

if errors:
    print("FAIL")
    for err in errors:
        print(f"- {err}")
    sys.exit(1)

print("PASS: manifest and artifact paths are internally consistent")
PY
```

Manual screenshot quality gate (required):

- Review screenshots linked by all `PASS` rows.
- If image content is only loading/skeleton/spinner/blank placeholder, change that test to `FAIL` or `BLOCKED` with reason.

---

_Template generated for Marx POS comprehensive testing_
