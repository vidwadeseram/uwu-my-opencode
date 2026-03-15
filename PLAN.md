# PLAN.md — uwu-my-opencode

> Implementation roadmap from MVP to production.

---

## Vision

A self-hosted, browser-accessible AI coding workspace where you can:
- Run multiple AI-powered coding sessions concurrently across different projects
- Access everything from any device with a browser
- Get instant preview URLs for testing web apps, APIs, and mobile backends
- Authenticate once with GitHub, work forever

---

## Phase 0: Foundation (Week 1)

**Goal**: Rust project scaffold, CLI skeleton, basic build pipeline.

### Deliverables

- [ ] Initialize Cargo workspace
  ```
  uwu-my-opencode/
    Cargo.toml
    src/
      main.rs           # clap CLI: `uwu-daemon start`, `uwu-daemon status`
      config.rs          # Config struct + env/file loading
      error.rs           # Error types (thiserror)
    .github/
      workflows/ci.yml   # cargo check + test + clippy
  ```
- [ ] Configuration system
  - Load from: CLI flags → env vars → config file (`/etc/uwu/config.toml` or `~/.config/uwu/config.toml`)
  - Key config values:
    ```toml
    [server]
    host = "0.0.0.0"
    port = 8080

    [auth]
    github_client_id = "Iv1.xxx"
    # github_client_secret not needed for device flow with GitHub Apps

    [workspace]
    base_dir = "/home"
    default_memory_mb = 512
    default_cpu_shares = 512
    idle_timeout_secs = 3600

    [tunnel]
    provider = "cloudflare"  # or "bore"
    cloudflare_account_id = "xxx"
    cloudflare_api_token = "xxx"
    # bore_server = "bore.yourdomain.com"

    [opencode]
    port_range_start = 4100
    port_range_end = 4999
    ```
- [ ] Structured logging setup (`tracing` + `tracing-subscriber`)
- [ ] Basic integration test harness

### Success Criteria
- `cargo build` produces a binary
- `uwu-daemon --help` shows subcommands
- `uwu-daemon start` starts an axum server on configured port
- CI passes on GitHub Actions

---

## Phase 1: Core MVP (Weeks 2-4)

**Goal**: Single-user can authenticate, get a workspace, use opencode in browser, and get preview URLs.

### 1.1 GitHub Authentication (Week 2)

- [ ] GitHub App registration (manual, documented in README)
  - Permissions: `read:user`, `user:email`
  - Enable device flow in GitHub App settings
- [ ] Device flow implementation (`oauth2` + `oauth2-reqwest` crates)
  ```
  POST /auth/device → { device_code, user_code, verification_uri, interval }
  POST /auth/poll   → { access_token } (polls GitHub, handles slow_down/pending/expired)
  GET  /auth/me     → { github_id, login, avatar_url }
  ```
- [ ] JWT token issuance
  - On successful GitHub auth → mint JWT with `{ sub: github_id, exp: +24h }`
  - JWT used for all subsequent API calls and ttyd access
- [ ] Auth middleware for axum routes
- [ ] First-user lockdown: the first GitHub user to authenticate becomes the owner; subsequent logins must match that user ID
- [ ] Token storage: save refresh token to `~/.config/uwu/auth.json` (chmod 600)

### 1.2 Workspace Provisioning (Week 2-3)

- [ ] Linux user creation per platform user
  ```rust
  // useradd --create-home --shell /bin/bash uwu-<github_login>
  // Set up ~/.config/opencode/ with seeded configs
  ```
- [ ] opencode config seeding
  ```json
  // ~/.config/opencode/opencode.json
  {
    "plugin": ["oh-my-opencode"]
  }
  ```
  ```bash
  # Environment for opencode process
  OPENCODE_PERMISSION='{"all":"allow"}'
  HOME=/home/uwu-<user>
  ```
- [ ] Port allocator: assign unique port from configured range per user's opencode server
- [ ] Workspace directory management
  ```
  POST /workspace         → create workspace (clones repo or creates empty dir)
  GET  /workspace         → list user's workspaces
  DELETE /workspace/:id   → destroy workspace
  ```

### 1.3 Process Supervisor (Week 3)

- [ ] Per-user process group tracking
  ```rust
  struct UserProcesses {
      tmux_pid: Option<u32>,
      opencode_pid: Option<u32>,
      ttyd_pid: Option<u32>,
      tunnels: HashMap<u16, TunnelProcess>,  // port → tunnel
  }
  ```
- [ ] tmux session lifecycle
  ```rust
  // Create isolated tmux server per user
  tmux -L uwu-<user> new-session -d -s main

  // Create project window
  tmux -L uwu-<user> new-window -t main -n <project-name>

  // Start opencode TUI in first pane
  tmux -L uwu-<user> send-keys -t main:<window>.0 "opencode" Enter
  ```
- [ ] opencode server lifecycle
  ```rust
  // Start headless server for SDK access
  // Run as the user, bound to loopback
  opencode serve --port <assigned_port> --hostname 127.0.0.1
  ```
- [ ] Health check loop (every 30s)
  - Verify tmux session exists
  - Verify opencode server responds to `GET /global/health`
  - Restart crashed processes with exponential backoff
- [ ] ttyd lifecycle
  ```bash
  ttyd --port <web_port> --credential <user>:<jwt_token> \
       tmux -L uwu-<user> attach -t main
  ```
- [ ] Graceful shutdown (SIGTERM handler)
  - Stop tunnels → stop ttyd → stop opencode → kill tmux → remove user session from DB
- [ ] Boot reconciliation
  - On daemon start: scan DB for expected sessions → check which are actually running → restart missing ones, clean up stale DB entries

### 1.4 Preview Tunnel Manager (Week 3-4)

- [ ] Cloudflare quick tunnel integration
  ```rust
  // Spawn cloudflared for a user's port
  cloudflared tunnel --url http://127.0.0.1:<port> --no-autoupdate
  // Parse stdout for the assigned URL (trycloudflare.com subdomain)
  ```
- [ ] Tunnel registry (in state DB)
  ```sql
  CREATE TABLE tunnels (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL,
      local_port INTEGER NOT NULL,
      tunnel_url TEXT,
      started_at TEXT NOT NULL,
      last_active_at TEXT NOT NULL,
      FOREIGN KEY (user_id) REFERENCES users(id)
  );
  ```
- [ ] API endpoints
  ```
  POST /workspace/:id/preview   { port: 3000 } → { url: "https://xxx.trycloudflare.com" }
  GET  /workspace/:id/previews                  → [{ port, url, started_at }]
  DELETE /workspace/:id/preview/:port           → stops tunnel
  ```
- [ ] Idle expiry: reap tunnels with no traffic after `idle_timeout_secs`
- [ ] For non-HTTP (binary/APK): serve file downloads via a simple static file server on a tunneled port

### 1.5 State Database (Throughout)

- [ ] SQLite schema via sqlx migrations
  ```sql
  CREATE TABLE users (
      id TEXT PRIMARY KEY,           -- UUID
      github_id INTEGER UNIQUE NOT NULL,
      github_login TEXT NOT NULL,
      linux_user TEXT UNIQUE NOT NULL,
      created_at TEXT NOT NULL
  );

  CREATE TABLE workspaces (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL,
      name TEXT NOT NULL,
      path TEXT NOT NULL,
      opencode_port INTEGER,
      tmux_window INTEGER,
      created_at TEXT NOT NULL,
      FOREIGN KEY (user_id) REFERENCES users(id)
  );

  CREATE TABLE sessions (
      id TEXT PRIMARY KEY,
      user_id TEXT NOT NULL,
      jwt_hash TEXT NOT NULL,
      expires_at TEXT NOT NULL,
      created_at TEXT NOT NULL,
      FOREIGN KEY (user_id) REFERENCES users(id)
  );

  -- tunnels table as above
  ```

### MVP Success Criteria

- [ ] User visits `https://yourdomain.com` → GitHub device flow → JWT → redirected to ttyd
- [ ] ttyd shows tmux session with opencode running
- [ ] User can create workspaces (via API or TUI command)
- [ ] User can switch between project windows in tmux
- [ ] User can open vim/emacs in a split pane
- [ ] `uwu preview up 3000` (from inside tmux) → returns preview URL
- [ ] Preview URL is accessible from any browser
- [ ] Session persists across browser tab close/reopen
- [ ] Idle tunnels auto-expire after configured timeout
- [ ] Daemon restarts cleanly, reattaches to existing sessions

---

## Phase 2: Hardening & Multi-User (Weeks 5-8)

**Goal**: Production-grade isolation, multiple concurrent users, SSH access.

### 2.1 Container Isolation

- [ ] Replace Linux user isolation with Docker containers per user
  ```yaml
  # Per-user container spec
  image: uwu-workspace:latest  # includes tmux, bun, opencode, oh-my-opencode
  network_mode: none            # deny-all by default
  read_only: true
  tmpfs: /tmp:size=512m
  mem_limit: 512m
  cpus: 0.5
  pids_limit: 100
  security_opt: [no-new-privileges, seccomp:profile.json]
  cap_drop: [ALL]
  user: "1000:1000"
  ```
- [ ] `bollard` crate for Docker API (create/start/stop/exec containers)
- [ ] Volume mounts for persistent workspace data
- [ ] Network policy: allow outbound only for tunnels + package managers

### 2.2 Embedded SSH Server

- [ ] `russh` crate for SSH server embedded in uwu-daemon
- [ ] SSH public key auth (backed by GitHub keys: `https://github.com/<user>.keys`)
- [ ] On SSH connect → attach to user's tmux session
- [ ] Port forwarding for preview URLs (alternative to cloudflared)

### 2.3 Resource Management

- [ ] Per-user resource quotas (configurable)
  ```toml
  [quotas]
  max_workspaces = 5
  max_concurrent_previews = 3
  max_memory_mb = 1024
  max_cpu_percent = 50
  workspace_disk_mb = 5000
  ```
- [ ] Quota enforcement in API + supervisor
- [ ] Usage metrics collection (CPU, memory, disk per user)
- [ ] Admin dashboard endpoint (`GET /admin/users`, `/admin/metrics`)

### 2.4 Resilience

- [ ] Systemd service file for uwu-daemon
- [ ] Automatic restart with backoff
- [ ] Structured audit log (all auth events, workspace operations, tunnel lifecycle)
- [ ] Alerting on resource exhaustion (webhook/email)

### Phase 2 Success Criteria

- [ ] Multiple users can sign in concurrently with full isolation
- [ ] No cross-user data access possible
- [ ] SSH access works (`ssh -p 2222 user@yourdomain.com`)
- [ ] Container crash → automatic restart within 30s
- [ ] Resource limits enforced (fork bomb doesn't take down host)
- [ ] Admin can view all users and their resource usage

---

## Phase 3: Polish & Scale (Weeks 9-12+)

**Goal**: Web-first experience, multi-host, production scale.

### 3.1 Custom Web Client (Replace ttyd)

- [ ] xterm.js frontend served from uwu-daemon
- [ ] Dual WebSocket channels (Zellij pattern):
  - Terminal channel: raw terminal I/O
  - Control channel: JSON commands (switch workspace, start preview, auth refresh)
- [ ] Rich UI elements overlaid on terminal (preview URL display, workspace switcher, status bar)
- [ ] Mobile-responsive terminal layout

### 3.2 Stable Preview Subdomains

- [ ] Cloudflare Tunnel API integration (not quick tunnels)
  - Named tunnels with DNS records
  - Pattern: `{workspace-name}.preview.yourdomain.com`
- [ ] Preview access controls (public, authenticated, team-only)
- [ ] Preview history and sharing

### 3.3 Multi-Host Scaling

- [ ] Separate control plane (API + auth + state) from worker plane (workspace containers)
- [ ] Central PostgreSQL for state (replace SQLite)
- [ ] Worker agent on each VPS, reports to control plane
- [ ] Workspace scheduling: assign new workspaces to least-loaded worker
- [ ] Cross-host workspace migration (stop container → move volume → restart)

### 3.4 Advanced Features

- [ ] Workspace templates (Next.js, React, Python Flask, etc.)
- [ ] Collaborative sessions (multiple users in same tmux session)
- [ ] Persistent workspace snapshots (pause/resume with full state)
- [ ] API key auth (for CI/CD integration)
- [ ] Webhook notifications (workspace events)
- [ ] Usage billing metering (if commercializing)

---

## Technical Risk Register

| # | Risk | Impact | Likelihood | Mitigation | Phase |
|---|---|---|---|---|---|
| 1 | opencode permission prompt hang in headless mode | Blocks all headless usage | Certain (known bug) | Force `OPENCODE_PERMISSION='{"all":"allow"}'` in config seeding; validate before server start | 1 |
| 2 | Orphaned processes after daemon crash | Resource leaks, port conflicts | High | PID registry in DB + boot reconciliation + cgroup tracking | 1 |
| 3 | Cross-user data access via filesystem | Security breach | Medium (MVP) | Per-user Linux accounts (MVP), containers (P2) | 1-2 |
| 4 | ttyd auth bypass | Unauthorized terminal access | Medium | JWT-based credential rotation; short-lived tokens | 1 |
| 5 | Cloudflare quick tunnel instability | Preview URLs go down | Low-Medium | Retry logic + bore fallback + tunnel health checks | 1 |
| 6 | Bun/opencode version incompatibility | Server won't start | Low | Pin versions in workspace provisioning; test upgrades in CI | 1 |
| 7 | tmux session corruption | Lost user work | Low | tmux is battle-tested; workspace git auto-commit as safety net | 2 |
| 8 | Resource exhaustion from single user | Other users affected | Medium | cgroup limits (MVP), container limits (P2), quota enforcement | 1-2 |

---

## Infrastructure Requirements

### MVP (Single VPS)

| Resource | Minimum | Recommended |
|---|---|---|
| Provider | Hetzner CX21 / DigitalOcean Basic | Hetzner CX31 |
| vCPU | 2 | 4 |
| RAM | 4 GB | 8 GB |
| Disk | 40 GB SSD | 80 GB NVMe |
| Network | 1 Gbps | 1 Gbps |
| OS | Ubuntu 24.04 LTS | Ubuntu 24.04 LTS |
| Cost | ~$6/mo | ~$12/mo |
| Concurrent Users | ~3-5 | ~8-12 |

### Required Services

| Service | Purpose | Cost |
|---|---|---|
| Cloudflare (free tier) | DNS + Tunnel | Free |
| GitHub App | Authentication | Free |
| Domain name | `yourdomain.com` | ~$12/yr |

### Setup Checklist

```bash
# 1. Server setup
apt update && apt install -y tmux ttyd cloudflared caddy

# 2. Install Bun
curl -fsSL https://bun.sh/install | bash

# 3. Install opencode + oh-my-opencode
bun install -g opencode oh-my-opencode

# 4. Install uwu-daemon (from release)
curl -fsSL https://github.com/yourorg/uwu-my-opencode/releases/latest/download/uwu-daemon-linux-amd64 -o /usr/local/bin/uwu-daemon
chmod +x /usr/local/bin/uwu-daemon

# 5. Configure
cp /etc/uwu/config.example.toml /etc/uwu/config.toml
# Edit config.toml with your GitHub App client ID, CF credentials, domain

# 6. Start
systemctl enable --now uwu-daemon
```

---

## Open Questions

1. **Should workspaces support git clone at creation?** Or should users clone manually in the terminal?
   - Leaning: support `POST /workspace { repo: "https://github.com/user/repo" }` for convenience
2. **How to handle opencode API key management?** Users need their own AI provider keys.
   - Options: user provides via TUI env vars, or platform provides shared keys with rate limiting
3. **Should we support multiple tmux sessions per user, or one session with multiple windows?**
   - Leaning: one session, multiple windows (simpler supervision, single ttyd attachment point)
4. **Mobile experience**: Is ttyd usable on mobile? Should we invest in a mobile-optimized web client early?
   - Leaning: defer to Phase 3. ttyd + tmux is functional on mobile but not optimized
5. **Workspace persistence across daemon upgrades**: How to handle in-place upgrades without disrupting running sessions?
   - Leaning: tmux sessions survive daemon restart by design; just restart the daemon

---

## Decision Log

| Date | Decision | Rationale |
|---|---|---|
| 2026-03-15 | Rust for daemon | Single binary, memory-safe, tmux_interface crate, Firecracker ecosystem alignment |
| 2026-03-15 | tmux over Zellij | Universal, battle-tested, ttyd compatibility, lighter weight |
| 2026-03-15 | ttyd for MVP web access | Zero frontend code, replaceable, xterm.js terminal emulation |
| 2026-03-15 | Cloudflare Tunnel for previews | Free, no bandwidth caps, REST API, custom subdomains |
| 2026-03-15 | GitHub App + Device Flow | Short-lived tokens, fine-grained permissions, TUI-native |
| 2026-03-15 | SQLite for state DB | Zero infrastructure, sufficient for single-host, upgradeable to PostgreSQL |
| 2026-03-15 | Per-user Linux accounts (MVP) | Simple isolation, upgradeable to containers in Phase 2 |
| 2026-03-15 | Supervisor pattern (not embedding) | Orchestrate mature tools rather than reimplement them |
