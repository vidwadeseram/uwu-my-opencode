#!/bin/bash

# Deployment script for plugin system to test server
set -e

SERVER="149.28.143.214"
USER="root"
PASSWORD="9_xEC279?85n}R{M"
REMOTE_DIR="/root/uwu-my-opencode"

echo "🚀 Deploying Plugin System to $SERVER..."

# Function to run remote commands with password
run_remote() {
    sshpass -p "$PASSWORD" ssh -o StrictHostKeyChecking=no "$USER@$SERVER" "$1"
}

# Function to copy files with password
copy_to_remote() {
    sshpass -p "$PASSWORD" scp -o StrictHostKeyChecking=no -r "$1" "$USER@$SERVER:$2"
}

echo "📦 Step 1: Checking server environment..."
run_remote "uname -a"
run_remote "df -h /"

echo "📦 Step 2: Installing dependencies..."
run_remote "apt-get update && apt-get install -y git curl build-essential"

# Install Bun
run_remote "curl -fsSL https://bun.sh/install | bash || true"
run_remote "export PATH=\"\$HOME/.bun/bin:\$PATH\" && bun --version || echo 'Bun not installed'"

# Install Rust
run_remote "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || true"
run_remote "source \$HOME/.cargo/env && cargo --version || echo 'Rust not installed'"

echo "📦 Step 3: Cloning repository..."
run_remote "rm -rf $REMOTE_DIR"
run_remote "git clone https://github.com/vidwadeseram/uwu-my-opencode.git $REMOTE_DIR"

echo "📦 Step 4: Setting up submodules..."
run_remote "cd $REMOTE_DIR && git submodule update --init --recursive"

echo "📦 Step 5: Building oh-my-opencode plugin system..."
run_remote "cd $REMOTE_DIR/oh-my-opencode && export PATH=\"\$HOME/.bun/bin:\$PATH\" && bun install"

echo "📦 Step 6: Running UI slot system tests..."
run_remote "cd $REMOTE_DIR/oh-my-opencode && export PATH=\"\$HOME/.bun/bin:\$PATH\" && bun test src/features/external-plugin-loader/__tests__/ui-slot.test.ts"

echo "📦 Step 7: Checking build..."
run_remote "cd $REMOTE_DIR/oh-my-opencode && export PATH=\"\$HOME/.bun/bin:\$PATH\" && bun run build 2>&1 | head -50"

echo "📦 Step 8: Creating test structure..."
run_remote "cd $REMOTE_DIR && mkdir -p test-plugins"
run_remote "cd $REMOTE_DIR && cp -r oh-my-opencode/docs/examples/full-plugin test-plugins/"

echo "✅ Deployment complete!"
echo ""
echo "📋 Next steps:"
echo "1. SSH into server: sshpass -p '$PASSWORD' ssh root@$SERVER"
echo "2. Navigate: cd $REMOTE_DIR"
echo "3. Test plugin system: cd oh-my-opencode && bun test"
echo "4. Create PR from local machine with changes"
echo ""
echo "🔗 Server accessible at: $SERVER"
echo "📁 Code at: $REMOTE_DIR"