# Workspace Test Template (Compact)

This file is intentionally short so agents can execute consistently.

## Canonical Docs (Workspace Folder Layout)

Use these docs in this order:

1. `workspace-docs/TEMPLATE.md` (this compact execution contract)
2. `workspace-docs/SETUP.md` (runtime, DB, tmux, OTP log retrieval)
3. `workspace-docs/TEST_CASES.md` (full test matrix)

Only load sections needed for the tests you are currently running.

## Critical Rules (Do Not Break)

1. **Merchant phone format**
   - Canonical phone storage/output uses E.164, no spaces (example: `+94770805444`).
   - Merchant login UI pre-fills `+94`; input only the remaining digits (example: `770805444`).

2. **Merchant signup OTP source**
   - Read OTP only from tmux session `<workspace-name>` window `commons-api`.
   - Do not use OTP from unrelated sessions (for example `kyc-test`).
   - OTP must match the same signup phone used in the test input.

3. **Screenshot truthfulness**
   - A test cannot be `PASS` if screenshot shows `404`, `Not Found`, app error, spinner-only, skeleton-only, or blank loading state.
   - Re-capture after UI is stable; if never stable, mark `FAIL` and record root cause.

4. **Report media requirements**
   - HTML report must include clickable screenshot links and image previews.
   - HTML report must include `<video controls>` and a direct file link to the video artifact.
   - Placeholder-only text (for example "video is in folder") is not accepted.

5. **Manifest correctness**
   - `manifest.json` `video.path` must point to a real file (`video/full-process.webm` or `.mp4`), not a directory.
   - `summary.total` must equal `passed + failed + skipped`.
   - `status=pass` cannot coexist with failures or blockers.

6. **Run bootstrap is mandatory**
   - Create `logs/{run_id}/index.html` and `logs/{run_id}/manifest.json` at test start (before executing cases).
   - Never leave a run folder without `index.html` and `manifest.json`.
   - If execution aborts early, keep artifacts and set `status` to `fail` with a clear blocker reason.

7. **tmux session isolation**
   - Run backend/frontend service windows in tmux session `<workspace-name>` only.
   - Do not create service windows under `uwu-main` (reserved for OpenCode workspace tabs).

8. **Merchant signup terms checkbox**
   - For signup submit, Terms & Conditions checkbox must be checked before clicking submit.
   - If submit is blocked because terms is unchecked, mark test as `FAIL` with screenshot evidence.

9. **Infra retry policy (no logic changes)**
   - If a test fails due to infrastructure/port wiring (`unknown service`, `connection refused`, `deadline exceeded`, or similar dependency-call errors), fix infra first and re-run the same test.
   - Do not change business logic/code to bypass infra failures.
   - Mark as `BLOCKED` only if the same test still fails after at least one documented infra fix + retry cycle.

## Execution Flow

1. Prepare environment with `workspace-docs/SETUP.md`.
2. Verify backend APIs and login preconditions before feature tests.
3. Run only required test sections from `workspace-docs/TEST_CASES.md`.
4. Capture screenshots for checkpoints and failures.
5. Record one full-process video per run.
6. Generate report files under `logs/{run_id}/`.
7. Run artifact validation checks from `workspace-docs/SETUP.md` before final status.

## Required Run Artifacts

For each run (`logs/{run_id}/`), produce:

- `index.html`
- `manifest.json`
- `screenshots/*.png`
- `video/full-process.webm` (or `video/full-process.mp4`)

## Test Status Policy

- Use `PASS` only when expected behavior is visible and artifacts prove it.
- Use `FAIL` for any functional mismatch, missing required artifact, or invalid screenshot evidence.
- Use `BLOCKED` only for external blockers with explicit evidence.

## Note for Existing Workspaces

If root files (`TEMPLATE.md`, `SETUP.md`) are short pointers, canonical content is in `workspace-docs/`.
