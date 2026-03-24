# Workspace Test Template (Compact)

Use this file as the strict execution contract for `allinonepos` runs.

## Canonical Docs (Read In Order)

1. `workspace-docs/TEMPLATE.md`
2. `workspace-docs/SETUP.md`
3. `workspace-docs/TEST_CASES.md`

## Mandatory Outcome

Every run must produce trustworthy end-to-end coverage across the discovered app surface:

- `pos-web` routes: **55**
- `pos-super-admin` routes: **17**
- `pos-customer` routes: **2**
- Total frontend routes: **74**

In addition to route loading, each run must exercise:

- every visible click path (buttons, menu entries, actionable table icons)
- every form flow (positive + negative validation)
- core auth flows (login, signup, OTP, reset/password where present)

Coverage is only considered complete when route/button/form totals are explicitly recorded in run output.

## Critical Rules (Do Not Break)

1. **Phone handling**
   - Canonical merchant phone is E.164 (`+94770805444`, no spaces).
   - Merchant login UI pre-fills `+94`; type only remaining digits (`770805444`).

2. **Signup OTP source**
   - Read OTP only from tmux session `<workspace-name>` window `commons-api`.
   - OTP must match the same signup phone used in the current run.

3. **Terms checkbox**
   - Merchant signup submit must only happen after Terms checkbox is checked and verified checked.

4. **No false PASS evidence**
   - `PASS` is invalid if screenshot/video evidence shows `404`, `Not Found`, wrong page, loading-only state, skeleton-only state, spinner-only state, or app error page.

5. **Page identity guard**
   - Verify both URL path and heading before marking `PASS`.
   - Use canonical names from `workspace-docs/TEST_CASES.md` (example: `junk-qr`, never `junk-qr-payments`).

6. **Infra-first retry policy**
   - For infra/port/dependency errors (`unknown service`, `connection refused`, `deadline exceeded`, gRPC unimplemented, DB unavailable), fix infra and retry the same case.
   - Do not change app business logic to bypass infra failures.
   - Mark `BLOCKED` only after at least one documented infra-fix + retry cycle.

7. **Run bootstrap required**
   - Before executing tests, create `logs/{run_id}/index.html`, `logs/{run_id}/manifest.json`, `logs/{run_id}/screenshots/`, and `logs/{run_id}/video/`.

8. **Manifest integrity required**
   - `summary.total = passed + failed + skipped`
   - `status=pass` cannot coexist with failed tests or non-empty blocker
   - `video.path` must point to a real file (`video/full-process.webm` or `.mp4`)
   - `tests[]` must include route-level and action-level entries (route opens, button flows, form submissions)

9. **tmux isolation**
   - Run project services in workspace session `<workspace-name>` only.
   - Do not place project service tabs under `uwu-main`.

10. **Media requirements**
    - HTML report must show clickable screenshot links + previews.
    - HTML report must contain playable `<video controls>` + direct video link.

## Execution Flow (Required)

1. Complete setup checks in `workspace-docs/SETUP.md` (DB, env, ports, tmux, Playwright).
2. Bootstrap run artifacts under `logs/{run_id}/` before first test.
3. Execute coverage plan from `workspace-docs/TEST_CASES.md` for all route groups.
4. Capture screenshot evidence at every checkpoint and failure.
5. Record one full-process video per run.
6. Run artifact and manifest integrity validation commands.
7. Publish final status with explicit pass/fail/blocker reasoning.

## Required Run Artifacts

- `logs/{run_id}/index.html`
- `logs/{run_id}/manifest.json`
- `logs/{run_id}/screenshots/*.png`
- `logs/{run_id}/video/full-process.webm` or `logs/{run_id}/video/full-process.mp4`
- `logs/{run_id}/coverage.json` (route/button/form coverage summary)

## Status Policy

- `PASS`: expected behavior confirmed and evidence is valid
- `FAIL`: functional mismatch, validation failure, wrong page, or invalid evidence
- `BLOCKED`: external dependency remains broken after infra-fix + retry

## Existing Workspaces

If root `TEMPLATE.md` / `SETUP.md` are short pointers, use canonical files under `workspace-docs/`.
