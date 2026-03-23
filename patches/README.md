# Submodule Patch Overlays

This repository tracks local modifications to submodules as patch files so we can ship changes from the parent repo without pushing submodule remotes.

## Layout

- `patches/opencode/*.patch` applies to `./opencode`
- `patches/oh-my-opencode/*.patch` applies to `./oh-my-opencode`
- `patches/tmux/*.patch` applies to `./tmux`
- `patches/openagentscontrol/*.patch` applies to `./openagentscontrol`

## Apply patches

```bash
./scripts/apply-submodule-patches.sh
```

This is idempotent:

- already applied patches are skipped
- new patches are applied with `git apply`

## Refresh opencode patch

```bash
./scripts/refresh-opencode-patch.sh
```

This regenerates:

- `patches/opencode/0001-report-workspace-ui-and-html-routing.patch`

from the current working state of `./opencode`.
