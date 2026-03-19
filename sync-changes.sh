#!/bin/bash

# Sync local changes to test server
set -e

SERVER="149.28.143.214"
USER="root"
PASSWORD="9_xEC279?85n}R{M"
REMOTE_DIR="/root/uwu-my-opencode"

echo "🔄 Syncing local changes to $SERVER..."

# Function to run remote commands with password
run_remote() {
    sshpass -p "$PASSWORD" ssh -o StrictHostKeyChecking=no "$USER@$SERVER" "$1"
}

# Function to copy files with password
copy_to_remote() {
    sshpass -p "$PASSWORD" scp -o StrictHostKeyChecking=no -r "$1" "$USER@$SERVER:$2"
}

echo "📦 Step 1: Creating directory structure..."
run_remote "mkdir -p $REMOTE_DIR/oh-my-opencode/src/features/external-plugin-loader/__tests__"
run_remote "mkdir -p $REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/src/{components,tools,hooks,mcp}"

echo "📦 Step 2: Copying UI slot system files..."
# Copy UI slot system implementation
copy_to_remote "oh-my-opencode/src/features/external-plugin-loader/ui-slot-registry.ts" "$REMOTE_DIR/oh-my-opencode/src/features/external-plugin-loader/"
copy_to_remote "oh-my-opencode/src/features/external-plugin-loader/ui-slot-manager.ts" "$REMOTE_DIR/oh-my-opencode/src/features/external-plugin-loader/"
copy_to_remote "oh-my-opencode/src/features/external-plugin-loader/plugin-ui-api.ts" "$REMOTE_DIR/oh-my-opencode/src/features/external-plugin-loader/"
copy_to_remote "oh-my-opencode/src/features/external-plugin-loader/ui-error-boundary.tsx" "$REMOTE_DIR/oh-my-opencode/src/features/external-plugin-loader/"
copy_to_remote "oh-my-opencode/src/features/external-plugin-loader/__tests__/ui-slot.test.ts" "$REMOTE_DIR/oh-my-opencode/src/features/external-plugin-loader/__tests__/"

echo "📦 Step 3: Updating index.ts..."
copy_to_remote "oh-my-opencode/src/features/external-plugin-loader/index.ts" "$REMOTE_DIR/oh-my-opencode/src/features/external-plugin-loader/"

echo "📦 Step 4: Copying full plugin template..."
copy_to_remote "oh-my-opencode/docs/examples/full-plugin/plugin.json" "$REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/"
copy_to_remote "oh-my-opencode/docs/examples/full-plugin/README.md" "$REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/"
copy_to_remote "oh-my-opencode/docs/examples/full-plugin/src/index.ts" "$REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/src/"
copy_to_remote "oh-my-opencode/docs/examples/full-plugin/src/components/" "$REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/src/"
copy_to_remote "oh-my-opencode/docs/examples/full-plugin/src/tools/" "$REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/src/"
copy_to_remote "oh-my-opencode/docs/examples/full-plugin/src/hooks/" "$REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/src/"
copy_to_remote "oh-my-opencode/docs/examples/full-plugin/src/mcp/" "$REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/src/"

echo "📦 Step 5: Running tests..."
run_remote "cd $REMOTE_DIR/oh-my-opencode && export PATH=\$HOME/.bun/bin:\$PATH && bun test src/features/external-plugin-loader/__tests__/ui-slot.test.ts"

echo "✅ Sync complete!"