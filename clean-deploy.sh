#!/bin/bash

# Clean deployment using uwu-daemon architecture
set -e

SERVER="149.28.143.214"
USER="root"
PASSWORD="9_xEC279?85n}R{M"
DOMAIN="code.vidwadeseram.com"
REMOTE_DIR="/root/uwu-my-opencode"

echo "🧹 Cleaning and deploying uwu-my-opencode to $DOMAIN..."

# Function to run remote commands
run_remote() {
    sshpass -p "$PASSWORD" ssh -o StrictHostKeyChecking=no "$USER@$SERVER" "$1"
}

echo "📦 Step 1: Cleaning existing setup..."
run_remote "rm -rf $REMOTE_DIR"
run_remote "pkill -f uwu-daemon || true"
run_remote "pkill -f ttyd || true"

echo "📦 Step 2: Installing system dependencies..."
run_remote "apt-get update && apt-get install -y git curl build-essential nginx certbot python3-certbot-nginx tmux"

echo "📦 Step 3: Cloning uwu-my-opencode (THIS repo)..."
run_remote "git clone https://github.com/vidwadeseram/uwu-my-opencode.git $REMOTE_DIR"

echo "📦 Step 4: Initializing submodules (clean, no modifications)..."
run_remote "cd $REMOTE_DIR && git submodule update --init --recursive"

echo "📦 Step 5: Building forked tmux..."
run_remote "cd $REMOTE_DIR/tmux && sh autogen.sh && ./configure --prefix=\"$REMOTE_DIR/build/tmux\" && make -j\"\$(nproc)\" && make install"

echo "📦 Step 6: Installing dependencies for forks..."
run_remote "cd $REMOTE_DIR/opencode && export PATH=\$HOME/.bun/bin:\$PATH && bun install"
run_remote "cd $REMOTE_DIR/oh-my-opencode && export PATH=\$HOME/.bun/bin:\$PATH && bun install"

echo "📦 Step 7: Building uwu-daemon..."
run_remote "cd $REMOTE_DIR/daemon && cargo build --release"

echo "📦 Step 8: Setting up plugin system injection..."
# Create plugin injection directory
run_remote "mkdir -p $REMOTE_DIR/plugin-injection"
run_remote "mkdir -p $REMOTE_DIR/workspaces"

# Copy our UI slot system to plugin injection directory
echo "📦 Step 9: Copying plugin system implementation..."
# We'll inject via .opencode files in workspaces
run_remote "mkdir -p $REMOTE_DIR/workspaces/default/.opencode"

echo "✅ Clean setup complete!"
echo ""
echo "📋 Next: Configure domain $DOMAIN and deploy full system"