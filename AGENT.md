# AGENT.md — uwu-my-opencode

> Project knowledge base for AI agents and developers working on this codebase.

## What This Project Is

**uwu-my-opencode** is a remote AI-powered development environment. It wraps [opencode](https://github.com/sst/opencode) + [oh-my-opencode](https://github.com/code-yeongyu/oh-my-openagent) inside tmux sessions, serves them over the internet via a browser terminal, and provides tunnel-based preview URLs for testing — all behind GitHub authentication.

## Implementation Snapshot (Current)

Current daemon behavior differs from some planned sections below:

- startup-centered workflow (no active `/workspaces` UX path)
- single tmux session (`uwu-main`) with one tab per workspace directory
- ttyd browser entry with static credentials (`admin`/`admin`)
- forked OpenCode and forked oh-my-opencode loaded from local submodules
- Linux auto-bootstrap for missing tmux/nvim configs from `vidwadeseram/dotfiles`

Treat README as the source of truth for deploy/run commands. This file still contains roadmap architecture notes that are partially future-state.

**One-liner**: A self-hosted, browser-accessible AI coding workspace with live preview tunnels.

### The Problem It Solves

You want to prototype, feasibility-test, or rework UI across multiple projects concurrently — from anywhere, on any device. You need:
- AI coding agents (opencode + oh-my-opencode) always running
- Multiple project tabs with persistent sessions
- One-click preview URLs for testing web apps, APIs, or mobile backends
- No local setup — just a browser and a GitHub account

### The Core Idea

```
Browser/SSH → uwu-daemon (Rust) → tmux → opencode + oh-my-opencode
                                      ↓
                              cloudflared tunnel → preview URL
```

---

## Architecture Overview

### Process Model

uwu-my-opencode follows a **supervisor pattern**: a single Rust daemon (`uwu-daemon`) orchestrates external processes. It does NOT embed terminal rendering, muxing, or tunnel logic — it supervises mature tools.

```
                   Internet
                      |
              Cloudflare (ingress)
                      |
                +----------------+
                |   uwu-daemon   |  ← Rust binary (this project)
                | - axum HTTP/WS |
                | - GitHub OAuth  |
                | - state DB     |
                | - supervisor   |
                +-------+--------+
                        |
          +-------------+--------------------+
          |                                  |
   per-user supervisor                 per-user supervisor
          |                                  |
   +------+-------+                    +------+-------+
   |  tmux server |                    |  tmux server |
   | session: u1  |                    | session: u2  |
   +------+-------+                    +------+-------+
          |                                  |
   +------+--------+                   +------+--------+
   | opencode serve |                   | opencode serve|
   | 127.0.0.1:p1   |                   | 127.0.0.1:p2  |
   +------+--------+                   +------+--------+
          |
   +------+--------+
   | ttyd (webterm) |
   | auth via JWT   |
   +------+--------+
          |
   Browser (user)
```

### Key Architecture Decisions

| Decision | Choice | Reasoning |
|---|---|---|
| Binary strategy | Single Rust binary, multi-process supervision | tmux/opencode/cloudflared are mature; embedding them makes v1 brittle |
| User isolation | One `opencode serve` per user | opencode has per-user SQLite, no multi-tenancy. Shared server = cross-user blast radius |
| TUI serving | Serve tmux directly via ttyd (MVP) | tmux IS the product surface. Custom ratatui UI = second UI to maintain |
| State model | Control state in DB, interactive state in tmux + filesystem | Separation of concerns. tmux handles reconnects natively |
| Preview tunnels | Cloudflare Tunnel (primary), bore (fallback) | Free, no bandwidth caps, REST API, custom subdomains |
| Auth | GitHub App + Device Flow (RFC 8628) | Fine-grained permissions, short-lived tokens, TUI-native |
| Isolation (MVP) | Per-user Linux accounts + cgroups | Simple, adequate for beta. Upgradeable to containers/Firecracker later |

### What We Build vs Reuse

**Reuse (DO NOT reimplement)**:
- `tmux` — session UX, multi-attach, panes, reconnection
- `opencode` + `oh-my-opencode` — AI agents, hooks, task graph, SDK
- `ttyd` — web terminal for MVP
- `cloudflared` — tunnel + ingress
- `caddy` — reverse proxy with auto-HTTPS (optional)

**Build (this project)**:
- `uwu-daemon` — auth, provisioning, supervisor, policy engine
- State DB + process reconciliation logic
- Preview manager — tunnel lifecycle, URL reporting, idle expiry
- Token issuance/validation — JWT for ttyd + API access
- Workspace contract — directory layout, env vars, opencode config seeding

---

## Component Deep-Dive

### opencode (sst/opencode)

- **Runtime**: Bun (NOT Node.js)
- **Architecture**: Client/server split — TUI thread + Worker thread (HTTP server via Hono)
- **TUI**: `@opentui/solid` (custom SolidJS terminal renderer, NOT Bubbletea/Ink)
- **DB**: SQLite via Drizzle ORM at `~/.local/share/opencode/opencode.db`
- **API**: Full REST API on port 4096 (default), OpenAPI 3.1 spec at `/doc`
- **SDK**: `@opencode-ai/sdk` — auto-generated TypeScript client
- **Headless modes**: `opencode serve` (HTTP API), `opencode run` (one-shot), `opencode attach` (remote TUI)

**Critical for our use case**:
- MUST set `OPENCODE_PERMISSION='{"all":"allow"}'` — permission prompts hang in headless mode
- One server handles multiple directories via `x-opencode-directory` header
- Sessions scoped to directories, support parent/child hierarchy (forking)
- SSE event stream at `GET /event` for real-time status

### oh-my-opencode (code-yeongyu/oh-my-openagent)

An opencode plugin providing:
- **11 specialized agents** — Sisyphus (orchestrator), Oracle (consultant), Librarian (researcher), Explore (grep), Prometheus/Metis/Momus (planning), Hephaestus (deep worker), Atlas (executor), Sisyphus-Junior (category-spawned)
- **Category-based task delegation** — visual-engineering, ultrabrain, deep, quick, etc. with model routing
- **Skill system** — SKILL.md files with embedded MCPs (playwright, git-master, frontend-ui-ux, dev-browser)
- **25+ hooks** — lifecycle interception (recovery, fallback, context injection, notifications)
- **Background agents** — spawns child sessions with tmux pane management
- **Session introspection** — list/read/search/info across sessions
- **Persistent task graph** — `.sisyphus/tasks/` survives session restarts

### tmux

Used as the primary UX surface:
- Each user gets an isolated tmux server (`-L uwu-<user>`)
- Projects are tmux windows/panes within the user's session
- opencode TUI runs inside tmux panes
- Users can open additional panes for vim/emacs/manual editing
- tmux handles reconnection natively (detach/reattach)
- Control mode (`-CC`) available for programmatic event streaming

### Preview Tunnel System

```
User/AI starts app on port 3000
  → uwu-daemon detects or receives request
  → spawns: cloudflared tunnel --url http://127.0.0.1:3000
  → returns: https://<session-id>.preview.yourdomain.com
  → tracks in state DB
  → auto-expires on idle
```

---

## Codebase Conventions

### Language & Runtime

- **Primary language**: Rust (2021 edition)
- **Async runtime**: Tokio
- **Error handling**: `anyhow` for applications, `thiserror` for libraries
- **Serialization**: `serde` + `serde_json`

### Project Structure (target)

```
uwu-my-opencode/
  src/
    main.rs              # Entry point, CLI parsing
    daemon/              # uwu-daemon core
      mod.rs
      server.rs          # axum HTTP/WebSocket server
      auth.rs            # GitHub OAuth device flow
      supervisor.rs      # Per-user process manager
      state.rs           # State DB (SQLite)
    workspace/           # User workspace management
      mod.rs
      provisioner.rs     # Create user, dirs, configs
      tmux.rs            # tmux session management
      opencode.rs        # opencode server lifecycle
    preview/             # Tunnel management
      mod.rs
      tunnel.rs          # cloudflared lifecycle
      registry.rs        # Active tunnel tracking
    config.rs            # Configuration
    error.rs             # Error types
  tests/
  Cargo.toml
  AGENT.md               # This file
  SKILLS.md              # Tech stack & capabilities
  PLAN.md                # Implementation roadmap
```

### Code Style Rules

1. **No `unwrap()` in production code** — use `?` with proper error types
2. **Structured logging** via `tracing` crate — every spawned process gets a span
3. **Process cleanup is non-negotiable** — every spawn must have a corresponding reap path
4. **Configuration over hardcoding** — ports, paths, timeouts all configurable
5. **Test process lifecycle** — integration tests must verify spawn → health check → cleanup

### Git Conventions

- Branch naming: `feat/`, `fix/`, `refactor/`, `docs/`
- Commit messages: imperative mood, concise ("add user provisioning", "fix tunnel cleanup on idle")
- No force pushes to `main`

---

## Known Risks & Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| opencode permission prompt hangs in headless | Critical | Force `OPENCODE_PERMISSION='{"all":"allow"}'` at workspace provision; validate config before starting server |
| Orphaned processes (tmux/opencode/cloudflared) | High | PID registry + cgroup tracking + reaper loop on boot (reconcile DB vs live processes) |
| Cross-user filesystem exposure | High | Per-user Linux accounts + strict workspace permissions; never share opencode server across users |
| ttyd auth bypass | Medium | JWT-protected access; ttyd `--credential` flag + daemon-issued short-lived tokens |
| Resource exhaustion (fork bomb, memory) | Medium | cgroup limits per user (memory, CPU, PIDs); idle session reaping |

---

## Environment Requirements

- **OS**: Linux (required for cgroups, user isolation, KVM for future Firecracker)
- **Dependencies**: tmux, Bun (for opencode), ttyd, cloudflared, caddy (optional)
- **Rust toolchain**: stable, latest
- **Minimum VPS**: 2 vCPU, 4GB RAM, 40GB SSD (supports ~5 concurrent users in MVP)
