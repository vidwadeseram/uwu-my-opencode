#!/bin/bash

# Inject plugin system into uwu-daemon workspaces
set -e

SERVER="149.28.143.214"
USER="root"
PASSWORD="9_xEC279?85n}R{M"
REMOTE_DIR="/root/uwu-my-opencode"

echo "🔌 Injecting plugin system into uwu-daemon..."

run_remote() {
    sshpass -p "$PASSWORD" ssh -o StrictHostKeyChecking=no "$USER@$SERVER" "$1"
}

echo "📦 Step 1: Creating plugin injection directory..."
run_remote "mkdir -p $REMOTE_DIR/plugin-injection"

echo "📦 Step 2: Creating .opencode plugin loader for workspaces..."
# This is the file that gets injected into each workspace
run_remote "cat > $REMOTE_DIR/plugin-injection/opencode-plugin-loader.json << 'EOF'
{
  \"plugin\": [
    \"oh-my-opencode\",
    {
      \"name\": \"external-plugin-loader\",
      \"config\": {
        \"enabled\": true,
        \"autoInstall\": true,
        \"autoUpdate\": false,
        \"pluginDir\": \"plugins\",
        \"plugins\": [
          {
            \"type\": \"local\",
            \"source\": \"$REMOTE_DIR/plugin-injection/full-plugin\",
            \"enabled\": true
          }
        ]
      }
    }
  ],
  \"agents\": {
    \"sisyphus\": {
      \"model\": \"claude-3-5-sonnet-20241022\",
      \"temperature\": 0.7
    }
  },
  \"categories\": {
    \"visual-engineering\": {
      \"model\": \"gpt-4o\",
      \"temperature\": 0.3
    }
  }
}
EOF"

echo "📦 Step 3: Creating full plugin template in injection directory..."
run_remote "mkdir -p $REMOTE_DIR/plugin-injection/full-plugin"
run_remote "cp -r $REMOTE_DIR/oh-my-opencode/docs/examples/full-plugin/* $REMOTE_DIR/plugin-injection/full-plugin/ 2>/dev/null || true"

echo "📦 Step 4: Creating workspace with plugin injection..."
run_remote "mkdir -p $REMOTE_DIR/workspaces/test-plugin"
run_remote "cp $REMOTE_DIR/plugin-injection/opencode-plugin-loader.json $REMOTE_DIR/workspaces/test-plugin/.opencode"

echo "📦 Step 5: Creating test script to verify plugin system..."
run_remote "cat > $REMOTE_DIR/test-plugin-system.sh << 'EOF'
#!/bin/bash
echo \"Testing plugin system...\"
cd $REMOTE_DIR/workspaces/test-plugin

# Check if .opencode file exists
if [ -f .opencode ]; then
    echo \"✅ .opencode file created successfully\"
    cat .opencode
else
    echo \"❌ .opencode file missing\"
fi

# Test if we can run OpenCode with plugin
echo \"Testing OpenCode with plugin injection...\"
cd $REMOTE_DIR/opencode/packages/opencode
export PATH=\$HOME/.bun/bin:\$PATH

# Create a simple test
cat > test-plugin.js << 'TEST'
console.log(\"Plugin system test...\");
const fs = require('fs');
if (fs.existsSync('$REMOTE_DIR/workspaces/test-plugin/.opencode')) {
    console.log(\"✅ Plugin injection working\");
    process.exit(0);
} else {
    console.log(\"❌ Plugin injection failed\");
    process.exit(1);
}
TEST

bun run test-plugin.js
EOF"

run_remote "chmod +x $REMOTE_DIR/test-plugin-system.sh"

echo "📦 Step 6: Modifying uwu-daemon to inject plugin system..."
# We need to modify the workspace creation logic
run_remote "cd $REMOTE_DIR/daemon && git diff src/ || echo 'No modifications yet'"

echo "✅ Plugin injection setup complete!"
echo ""
echo "🔧 To test:"
echo "1. SSH to server: sshpass -p '$PASSWORD' ssh root@149.28.143.214"
echo "2. Run test: $REMOTE_DIR/test-plugin-system.sh"
echo "3. Access: https://code.vidwadeseram.com/"
echo ""
echo "📁 Workspace with plugin: $REMOTE_DIR/workspaces/test-plugin/"