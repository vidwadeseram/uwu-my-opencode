# SKILLS.md — uwu-my-opencode

> Technical skills, technology decisions, and capability map for the project.

## Current Runtime Notes (2026-03)

- `uwu-daemon install` subcommand provisions a full server (deps, build, systemd, nginx, certbot)
- thin bootstrap: `bash <(curl ... install.sh)` installs Rust then delegates to `uwu-daemon install`
- runtime entrypoint is `daemon/` (not root `src/` layout in older planning notes)
- tmux/opencode/oh-my-opencode are tracked as submodules in this repo
- forked tmux adds `protected-pane` option (prevents kill, auto-respawn)
- daemon launches OpenCode from local fork source (`opencode/packages/opencode/src/index.ts`)
- Running Projects `Start`/`Stop` maps to tmux session lifecycle for session `<workspace-name>`
- TMUX Test Log captures panes from `<workspace-name>` session only
- frontend publish framework is manifest-driven via `.opencode/frontends.json` + `scripts/publish-frontends.sh`
- new workspaces are scaffolded with `scripts/dev-tmux-session.sh`, `scripts/publish-frontends.sh`, `scripts/tmux-test-log.sh`
- Linux startup bootstraps missing configs:
  - tmux/nvim from `vidwadeseram/dotfiles`
  - Oh My Zsh with zsh-autosuggestions, zsh-syntax-highlighting, zsh-completions
- pre-commit hook enforces `cargo fmt` + `cargo check` on daemon changes
- production deployment docs (Namecheap + Nginx + certbot) live in `README.md`

---

## Core Technology Stack

### Rust Crates (Primary)

| Crate | Version | Purpose | Why This One |
|---|---|---|---|
| `tokio` | 1.x (full features) | Async runtime | Industry standard, required by all async deps |
| `axum` | 0.8 | HTTP/WebSocket server | Best ergonomics for Rust web, tower ecosystem |
| `tower` | 0.5 | Middleware (auth, rate limiting) | Composable, axum-native |
| `serde` / `serde_json` | 1.x | Serialization | Universal Rust standard |
| `sqlx` | 0.8 | SQLite async DB | Compile-time checked queries, async-native |
| `tmux_interface` | 1.0 | tmux CLI wrapper | Typed Rust API for every tmux command |
| `oauth2` | 5.x | GitHub OAuth device flow | RFC-compliant, strongly typed |
| `oauth2-reqwest` | 0.1 | HTTP client for oauth2 | Official reqwest adapter |
| `reqwest` | 0.13 | HTTP client (CF API, GitHub API) | Async, battle-tested |
| `jsonwebtoken` | 9.x | JWT issuance/validation | Standard for short-lived tokens |
| `tracing` | 0.1 | Structured logging | Async-aware, span-based |
| `tracing-subscriber` | 0.3 | Log formatting/output | Pairs with tracing |
| `anyhow` | 1.x | Application error handling | Ergonomic error chains |
| `thiserror` | 2.x | Library error types | Derive-based, clean API |
| `clap` | 4.x | CLI argument parsing | Derive API, subcommands |
| `uuid` | 1.x | Session/user IDs | Standard UUID generation |
| `dotenvy` | 0.15 | Environment config loading | .env file support |

### Future Crates (Phase 2+)

| Crate | Purpose | When |
|---|---|---|
| `ratatui` + `crossterm` | Custom TUI (if replacing ttyd) | Phase 3 |
| `russh` + `russh-keys` | Embedded SSH server | Phase 2 |
| `tokio-tungstenite` | WebSocket for custom web client | Phase 3 |
| `bollard` | Docker API client | Phase 2 (container isolation) |
| `nix` | Unix syscalls (cgroups, namespaces) | Phase 2 |

### External Dependencies (Runtime)

| Tool | Version | Purpose | Install |
|---|---|---|---|
| `tmux` | 3.4+ | Terminal multiplexer (user UX surface) | `apt install tmux` |
| `Bun` | 1.x | opencode runtime | `curl -fsSL https://bun.sh/install \| bash` |
| `opencode` | 1.2.26+ | AI coding tool | `bun install -g opencode` |
| `oh-my-opencode` | 3.11+ | opencode plugin | `bun install -g oh-my-opencode` |
| `ttyd` | 1.7+ | Web terminal (MVP) | `apt install ttyd` or binary |
| `cloudflared` | latest | Cloudflare Tunnel client | `apt install cloudflared` or binary |
| `caddy` | 2.x | Reverse proxy + auto-HTTPS (optional) | `apt install caddy` |

---

## Capability Map

### What uwu-daemon Must Be Able To Do

#### 1. Authentication & Session Management
- GitHub App device flow (RFC 8628)
- Device code display in TUI context
- Token polling with backoff (`authorization_pending`, `slow_down`)
- Short-lived JWT issuance for internal service auth
- Token refresh flow (GitHub App tokens expire in 8 hours)
- Session persistence across reconnects
- Single-user lockdown (only the GitHub user who first authenticated can access)
- Reset mechanism accessible from TUI

#### 2. User Workspace Provisioning
- Create isolated Linux user account per platform user
- Set up home directory with correct structure:
  ```
  /home/uwu-<user>/
    .config/opencode/
      opencode.json        # plugin config (oh-my-opencode enabled)
      oh-my-opencode.jsonc # omo config
    workspaces/            # project directories
      project-a/
      project-b/
    .local/share/opencode/ # opencode DB + state
  ```
- Seed opencode config with permissions set to allow-all
- Configure oh-my-opencode with appropriate defaults
- Set cgroup resource limits (memory, CPU, PIDs)

#### 3. Process Supervision
- Spawn and monitor per-user processes:
  - tmux server (isolated socket: `-L uwu-<user>`)
  - `opencode serve` (bound to `127.0.0.1:<port>`, per-user)
  - `ttyd` (attached to tmux session, JWT-protected)
  - `cloudflared` tunnels (per preview request)
- Health checks (periodic liveness probes)
- Automatic restart on crash (with backoff)
- Clean shutdown (SIGTERM → wait → SIGKILL)
- Orphan reaping on daemon boot (reconcile DB vs running processes)
- Idle detection and session cleanup

#### 4. tmux Session Management
- Create tmux sessions with project-specific windows
- Manage window/pane layout
- Send commands to panes programmatically
- Capture pane output for status reporting
- Support user opening manual edit panes (vim/emacs)
- Handle window navigation (tabs for projects)

#### 5. Preview Tunnel Management
- Start Cloudflare quick tunnels on demand
- Track active tunnels per user (port → URL mapping)
- Return preview URLs to requesting agent/user
- Auto-expire tunnels on idle (configurable timeout)
- Support for binary/APK downloads (serve static files via HTTP)
- Fallback to bore for TCP-only tunnels

#### 6. API Surface
- `POST /auth/device` — initiate GitHub device flow
- `POST /auth/token` — poll for token
- `GET /auth/me` — current user info
- `POST /workspace` — create workspace
- `GET /workspace` — list workspaces
- `DELETE /workspace/:id` — destroy workspace
- `POST /workspace/:id/preview` — start preview tunnel
- `DELETE /workspace/:id/preview/:port` — stop preview tunnel
- `GET /workspace/:id/status` — workspace health
- `GET /ws/terminal/:id` — WebSocket for terminal access (future)

---

## Technology Decisions Log

### Why Rust (not Go, not TypeScript)

1. **Single binary deployment** — no runtime dependencies for the daemon itself
2. **Memory safety without GC** — critical for a long-running supervisor managing child processes
3. **tmux_interface crate** — first-class tmux bindings already exist
4. **axum/tokio ecosystem** — production-grade async web framework
5. **Future path to Firecracker** — Firecracker itself is Rust, and the ecosystem (vmm crates) is Rust-native
6. **Performance** — supervisor must be lightweight; Rust's zero-cost abstractions matter when managing many concurrent users

### Why tmux (not Zellij, not custom muxer)

1. **Universal** — every Linux server has or can install tmux
2. **Battle-tested** — decades of production use in screen-sharing and remote dev
3. **Programmatic API** — CLI commands for every operation, control mode for events
4. **Lightweight** — minimal resource overhead per session
5. **Detach/reattach** — native session persistence, the core UX requirement
6. **ttyd compatibility** — ttyd can serve tmux sessions directly

Zellij was considered but rejected for MVP because:
- Its web client is newer and less battle-tested
- It would replace tmux entirely (bigger surface area to own)
- tmux + ttyd achieves the same result with less code

### Why Cloudflare Tunnel (not ngrok, not bore)

1. **Free tier with no bandwidth caps** — ngrok's free tier has 1GB/mo and injects interstitial pages
2. **Custom subdomains** — `{session}.preview.yourdomain.com` on free tier (with your own domain)
3. **REST API** — programmatic tunnel lifecycle management
4. **Enterprise-grade reliability** — Cloudflare's global network
5. **Post-quantum encryption** — security by default
6. **1,000 tunnels per account** — plenty for a multi-user platform

bore is kept as TCP fallback for environments without Cloudflare access.

### Why GitHub App (not OAuth App)

1. **Short-lived tokens** (8hr) — leaked tokens have bounded damage
2. **Fine-grained permissions** — request only `read:user`, not broad `repo` scope
3. **Device flow support** — perfect for TUI authentication
4. **GitHub's official recommendation** — "GitHub Apps are preferred over OAuth apps"
5. **Higher rate limits** — as installation, not per-user

### Why ttyd for MVP (not custom web client)

1. **Zero frontend code** — browser-accessible terminal immediately
2. **xterm.js under the hood** — full terminal emulation (colors, mouse, resize)
3. **SSL support** — built-in
4. **Minimal integration** — `ttyd tmux attach -t <session>` is the entire command
5. **Replaceable** — when we outgrow it, swap for custom axum WebSocket + xterm.js without changing the tmux layer

### Why SQLite (not PostgreSQL, not Redis)

1. **Zero infrastructure** — embedded, no external DB server
2. **Single-file deployment** — state lives next to the binary
3. **sqlx compile-time checks** — catch query errors at build time
4. **Sufficient for scale** — single VPS, hundreds of users, SQLite handles it
5. **Upgradeable** — sqlx supports PostgreSQL; swap when multi-host scaling demands it

---

## Skills Required for Contributors

### Must-Have
- Rust (async/await, tokio, error handling)
- Linux process management (fork, exec, signals, cgroups)
- tmux (sessions, windows, panes, programmatic control)
- HTTP APIs (REST design, JWT auth)
- OAuth 2.0 (device flow specifically)

### Good-to-Have
- opencode internals (SDK, API, configuration)
- oh-my-opencode plugin system (hooks, agents, skills)
- Cloudflare Tunnel API
- Container runtimes (Docker, Firecracker)
- xterm.js / web terminal protocols
- WebSocket protocol

### Not Needed for MVP
- Frontend development (ttyd handles UI)
- Kubernetes / cloud orchestration
- Mobile development
- Database administration (SQLite is self-managing)
