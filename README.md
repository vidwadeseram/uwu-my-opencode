# uwu-my-opencode

A self-hosted, browser-accessible AI coding workspace. Run multiple [opencode](https://github.com/sst/opencode) + [oh-my-opencode](https://github.com/code-yeongyu/oh-my-openagent) sessions from anywhere, get instant preview URLs for testing, authenticate with GitHub.

## What It Does

- **AI coding in the browser** — tmux sessions with opencode + oh-my-opencode, accessible from any device
- **Multiple concurrent projects** — each project gets its own tmux window with a persistent AI coding session
- **Instant preview URLs** — ask the AI to serve your app, get a shareable URL via Cloudflare Tunnel
- **GitHub auth** — sign in once, locked to your account
- **Session persistence** — close your browser, come back later, everything is exactly where you left it
- **Manual editing** — open vim/emacs in a split pane alongside the AI

## Architecture

```
Browser/SSH ──► uwu-daemon (Rust) ──► tmux ──► opencode + oh-my-opencode
                     │
                     └──► cloudflared ──► preview URL
```

A single Rust binary (`uwu-daemon`) supervises everything. It doesn't reimplement terminal muxing or AI coding — it orchestrates mature tools:

| Component | Role |
|---|---|
| **uwu-daemon** | Auth, provisioning, process supervision, tunnel management |
| **tmux** | Terminal multiplexer — the user-facing shell |
| **opencode** | AI coding tool (headless server + TUI) |
| **oh-my-opencode** | 11 specialized AI agents, skills, hooks |
| **ttyd** | Serves tmux to the browser (MVP) |
| **cloudflared** | Preview tunnels with free custom subdomains |

## Tech Stack

- **Rust** — daemon, async with Tokio, axum for HTTP/WebSocket
- **tmux** — session UX, multi-attach, pane management
- **Bun** — opencode runtime
- **SQLite** — state persistence
- **Cloudflare Tunnel** — free preview URLs, no bandwidth caps
- **GitHub App** — device flow auth with short-lived tokens

## Project Status

**Pre-alpha** — architecture and planning phase. See:

- [`AGENT.md`](AGENT.md) — architecture, component deep-dives, codebase conventions
- [`SKILLS.md`](SKILLS.md) — tech stack decisions, crate choices, capability map
- [`PLAN.md`](PLAN.md) — phased implementation roadmap with deliverables

## MVP You Can Test Now

The repository now includes a runnable Rust daemon with these endpoints:

- `GET /health`
- `GET /workspaces`
- `POST /workspaces`
- `POST /workspaces/{id}/start`
- `POST /workspaces/{id}/preview`

### Quickstart

```bash
cargo run -- --port 18080 --workspace-root ./tmp-workspaces --state-file ./.tmp-state.json
```

Then in another terminal:

```bash
curl http://127.0.0.1:18080/health

curl -X POST http://127.0.0.1:18080/workspaces \
  -H "content-type: application/json" \
  -d '{"name":"demo-project"}'
```

By default, the daemon runs in **dry-run mode** (`UWU_EXECUTE_COMMANDS=false`) so it returns the tmux/opencode/cloudflared commands without executing them.

To execute real commands:

```bash
UWU_EXECUTE_COMMANDS=true cargo run -- --port 18080
```

## How It Will Work

```
1. Visit https://yourdomain.com
2. Authenticate with GitHub (device flow)
3. Land in a browser terminal (tmux session)
4. Create a workspace: clone a repo or start fresh
5. opencode is already running — start coding with AI
6. Need to test? "Serve this on port 3000"
7. Get a preview URL instantly: https://xxx.trycloudflare.com
8. Open a split pane for manual editing anytime
9. Close your browser — session persists
10. Come back from any device — reattach instantly
```

## Requirements

- Linux VPS (2+ vCPU, 4+ GB RAM)
- Domain on Cloudflare DNS (for preview subdomains)
- GitHub App (for authentication)
- tmux, Bun, ttyd, cloudflared installed on the server

## License

[MIT](LICENSE)
