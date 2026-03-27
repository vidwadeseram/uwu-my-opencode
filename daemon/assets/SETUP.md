# Setup Contract

This workspace uses root-level setup guidance plus machine-readable `.toon` setup files.

## Canonical Sources

- Human-readable setup contract: `SETUP.md` (this file)
- Machine-readable setup steps: `setup/*.toon`

## Required Setup Flow

1. Validate environment prerequisites (runtime, services, credentials).
2. Run setup commands defined in `setup/*.toon`.
3. Confirm health checks declared in setup contracts.
4. Proceed to suites from `tests/*.toon` only after setup is healthy.

## Notes

- Keep `SETUP.md` focused on operator flow and troubleshooting.
- Keep execution details in `.toon` files so API/MCP validation can enforce structure.
