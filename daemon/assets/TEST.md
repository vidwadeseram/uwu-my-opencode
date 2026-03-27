# Test Contract

This workspace uses a structured `.toon` contract for deterministic test execution.

## Canonical Files

- `setup/*.toon` defines environment and dependency bootstrap commands.
- `tests/*.toon` defines suite metadata and links to test-case files.
- `test_cases/*.toon` defines granular executable cases.
- `.toon/schema.v1.toon` defines the DSL contract.

## Execution Order

1. Run setup from `setup/*.toon`.
2. Execute suites from `tests/*.toon`.
3. Resolve each `cases[]` reference to `test_cases/*.toon`.

## Validation Rules

- Every `.toon` file must include `version` and `kind`.
- `tests/*.toon` `cases[]` entries must point to existing files.
- Target `repo` must resolve to a discovered repository in the workspace.

## Backward Compatibility

Legacy `TEMPLATE.md` / `workspace-docs/*` are deprecated.
Use root `SETUP.md`, root `TEST.md`, and `.toon` files only.
