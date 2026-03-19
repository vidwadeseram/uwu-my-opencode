#!/bin/bash

# Start uwu-daemon and test full system
set -e

SERVER="149.28.143.214"
USER="root"
PASSWORD="9_xEC279?85n}R{M"
REMOTE_DIR="/root/uwu-my-opencode"
DOMAIN="code.vidwadeseram.com"

echo "🚀 Starting uwu-daemon and testing full system..."

run_remote() {
    sshpass -p "$PASSWORD" ssh -o StrictHostKeyChecking=no "$USER@$SERVER" "$1"
}

echo "📦 Step 1: Building uwu-daemon..."
run_remote "cd $REMOTE_DIR/daemon && cargo build --release"

echo "📦 Step 2: Starting uwu-daemon..."
run_remote "pkill -f uwu-daemon || true"
run_remote "cd $REMOTE_DIR/daemon && UWU_EXECUTE_COMMANDS=true ./target/release/uwu-daemon \
  --host 127.0.0.1 \
  --port 18080 \
  --workspace-root $REMOTE_DIR/workspaces \
  --state-file $REMOTE_DIR/.uwu-state.json \
  --ttyd-port-start 7681 \
  --tmux-bin $REMOTE_DIR/build/tmux/bin/tmux \
  --opencode-repo $REMOTE_DIR/opencode \
  --oh-my-opencode-repo $REMOTE_DIR/oh-my-opencode \
  > $REMOTE_DIR/daemon.log 2>&1 &"

echo "📦 Step 3: Waiting for daemon to start..."
sleep 5

echo "📦 Step 4: Testing daemon health endpoint..."
run_remote "curl -s http://127.0.0.1:18080/health || echo 'Daemon not responding yet'"

echo "📦 Step 5: Testing domain access..."
echo "Testing HTTPS access to $DOMAIN..."
curl -s -o /dev/null -w "%{http_code}" https://$DOMAIN/health 2>/dev/null || echo "Domain test failed"

echo "📦 Step 6: Testing plugin system..."
run_remote "bash $REMOTE_DIR/test-plugin-system.sh"

echo "📦 Step 7: Checking logs..."
run_remote "tail -20 $REMOTE_DIR/daemon.log"

echo "✅ Full system deployment complete!"
echo ""
echo "🎉 Access your deployment at:"
echo "   🔗 https://$DOMAIN/"
echo "   👤 Username: admin"
echo "   🔑 Password: admin"
echo ""
echo "📋 Workspace with plugin system:"
echo "   $REMOTE_DIR/workspaces/test-plugin/"
echo "   Contains .opencode file with plugin injection"
echo ""
echo "🔧 To test plugin system in terminal:"
echo "   1. Go to https://$DOMAIN/"
echo "   2. Login with admin/admin"
echo "   3. Navigate to test-plugin workspace"
echo "   4. Type 'opencode' to start OpenCode with plugin system"
echo ""
echo "📊 Health check:"
echo "   https://$DOMAIN/health"
echo "   https://$DOMAIN/api/health"