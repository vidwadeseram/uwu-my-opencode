# uwu-my-opencode

Self-hosted browser access to a persistent tmux workspace running forked `opencode` + `oh-my-opencode`.

## Current Behavior

- Daemon starts one tmux session: `uwu-main`
- Every directory inside `--workspace-root` becomes a tmux tab
- Each tab auto-launches forked OpenCode from `opencode/packages/opencode/src/index.ts`
- Per-workspace `.opencode` files are generated automatically:
  - plugin loader for forked `oh-my-opencode`
  - `/host-project` command for project hosting workflow
- ttyd serves the tmux session at `http://127.0.0.1:7681` by default
- ttyd auth is enabled: `admin` / `admin`
- Forked tmux protected pane mode prevents accidental pane kill for OpenCode panes

## Repository Layout

- `daemon/` — Rust supervisor and bootstrap logic
- `tmux/` — forked tmux (tracked as submodule)
- `opencode/` — forked opencode (tracked as submodule)
- `oh-my-opencode/` — forked plugin (tracked as submodule)
- `openagentscontrol/` — OpenAgentsControl (tracked as submodule)

## Local Run

From repo root:

```bash
cd daemon

UWU_EXECUTE_COMMANDS=true cargo run -- \
  --port 18080 \
  --workspace-root ./tmp-workspaces \
  --state-file ./.tmp-state.json \
  --tmux-bin "$(pwd)/../build/tmux/bin/tmux" \
  --opencode-repo ../opencode \
  --oh-my-opencode-repo ../oh-my-opencode
```

Open:

- `http://127.0.0.1:7681`
- username: `admin`
- password: `admin`

Health endpoint:

```bash
curl http://127.0.0.1:18080/health
```

## One-Command Install

SSH into a fresh Ubuntu VPS and run:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/vidwadeseram/uwu-my-opencode/main/scripts/install.sh)
```

It installs Rust, builds `uwu-daemon`, then runs the interactive installer which asks for your domain, email, and credentials. When done it prints your live HTTPS URL.

You can also pass flags to skip prompts:

```bash
bash <(curl -fsSL https://raw.githubusercontent.com/vidwadeseram/uwu-my-opencode/main/scripts/install.sh) \
  --domain code.vidwadeseram.com \
  --email vidwadeseram2002gmail.com \
  --ttyd-user admin \
  --ttyd-pass admin
```

If you already have Rust/cargo installed, skip the bootstrap script and run the CLI directly:

```bash
cargo install --git https://github.com/vidwadeseram/uwu-my-opencode --path daemon uwu-daemon
uwu-daemon install --domain code.example.com --email you@email.com
```

## Manual Deployment (Namecheap Domain + VPS)

Step-by-step alternative to the one-command installer.

### 1) Server Prerequisites

```bash
sudo apt update
sudo apt install -y git curl build-essential nginx certbot python3-certbot-nginx tmux

# ttyd (Ubuntu package may be old, use package or binary as preferred)
sudo apt install -y ttyd || true

# bun
curl -fsSL https://bun.sh/install | bash
echo 'export PATH="$HOME/.bun/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# rust
curl https://sh.rustup.rs -sSf | sh -s -- -y
source "$HOME/.cargo/env"
```

### 2) Clone and Build

```bash
git clone https://github.com/vidwadeseram/uwu-my-opencode.git
cd uwu-my-opencode

git submodule update --init --recursive

# apply parent-repo patch overlays to submodules
./scripts/apply-submodule-patches.sh

# build forked tmux
cd tmux
sh autogen.sh
./configure --prefix="$(pwd)/../build/tmux"
make -j"$(nproc)"
make install
cd ..

# install deps for forks
bun install --cwd opencode
bun install --cwd oh-my-opencode

# build daemon
cd daemon
cargo build --release
cd ..
```

### 3) Namecheap DNS

In Namecheap `Domain List -> Manage -> Advanced DNS`:

- Add `A` record for root:
  - Host: `@`
  - Value: `<YOUR_VPS_PUBLIC_IP>`
- Add `A` record for subdomain (recommended):
  - Host: `code`
  - Value: `<YOUR_VPS_PUBLIC_IP>`

Use either root domain (`example.com`) or subdomain (`code.example.com`) in steps below.

### 4) systemd Service

Create `/etc/systemd/system/uwu-daemon@.service`:

```ini
[Unit]
Description=uwu daemon
After=network.target

[Service]
User=%i
WorkingDirectory=/home/%i/uwu-my-opencode/daemon
Environment=UWU_EXECUTE_COMMANDS=true
ExecStart=/home/%i/uwu-my-opencode/daemon/target/release/uwu-daemon \
  --host 127.0.0.1 \
  --port 18080 \
  --workspace-root /home/%i/workspaces \
  --state-file /home/%i/.config/uwu/state.json \
  --ttyd-port-start 7681 \
  --tmux-bin /home/%i/uwu-my-opencode/build/tmux/bin/tmux \
  --opencode-repo /home/%i/uwu-my-opencode/opencode \
  --oh-my-opencode-repo /home/%i/uwu-my-opencode/oh-my-opencode
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
```

Enable it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now uwu-daemon@$(whoami)
sudo systemctl status uwu-daemon@$(whoami)
```

### 5) Nginx Reverse Proxy

Create `/etc/nginx/sites-available/uwu-my-opencode`:

```nginx
server {
    listen 80;
    server_name code.example.com;

    location / {
        proxy_pass http://127.0.0.1:7681;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }
}
```

Enable config:

```bash
sudo ln -sf /etc/nginx/sites-available/uwu-my-opencode /etc/nginx/sites-enabled/uwu-my-opencode
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl restart nginx
```

### 6) TLS Certificate

```bash
sudo certbot --nginx -d code.example.com
```

Choose redirect to HTTPS when prompted.

### 7) Verify

```bash
curl http://127.0.0.1:18080/health
curl -I https://code.example.com
```

Then open `https://code.example.com` and log in with `admin` / `admin`.

## Linux Auto-Bootstrap Behavior

On Linux only, daemon checks user config and installs missing files from `vidwadeseram/dotfiles`:

- `~/.tmux.conf` (if missing)
- `~/.config/nvim` (if missing)
- `~/.oh-my-zsh` (if missing)
- Oh My Zsh plugins (if missing):
  - `zsh-autosuggestions`
  - `zsh-syntax-highlighting`
  - `zsh-completions`
- `~/.zshrc` with plugin-enabled defaults (if missing)

It clones/pulls to `~/.cache/uwu-dotfiles` and does not overwrite existing configs.

## Troubleshooting

- DNS not resolving: wait for propagation, verify with `dig code.example.com`
- Certbot challenge failed: ensure port 80 is open and DNS points to this VPS
- Nginx welcome page still shows: remove default site and restart nginx
- ttyd unreachable behind proxy: verify websocket headers (`Upgrade`, `Connection`) in nginx config
- Firewall blocks: allow SSH/HTTP/HTTPS (`sudo ufw allow OpenSSH && sudo ufw allow 'Nginx Full'`)

## Pre-commit Checks

Enable repo hooks:

```bash
./scripts/setup-hooks.sh
```

Current pre-commit behavior:

- Runs only when staged files include `daemon/`
- Executes:
  - `cargo fmt --manifest-path daemon/Cargo.toml --all -- --check`
  - `cargo check --manifest-path daemon/Cargo.toml`

## License

[MIT](LICENSE)
