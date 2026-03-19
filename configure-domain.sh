#!/bin/bash

# Configure domain code.vidwadeseram.com
set -e

SERVER="149.28.143.214"
USER="root"
PASSWORD="9_xEC279?85n}R{M"
DOMAIN="code.vidwadeseram.com"
REMOTE_DIR="/root/uwu-my-opencode"

echo "🌐 Configuring domain $DOMAIN..."

run_remote() {
    sshpass -p "$PASSWORD" ssh -o StrictHostKeyChecking=no "$USER@$SERVER" "$1"
}

echo "📦 Step 1: Checking current IP..."
CURRENT_IP=$(run_remote "curl -s ifconfig.me")
echo "Server IP: $CURRENT_IP"
echo "Domain to configure: $DOMAIN"

echo "📦 Step 2: Setting up Nginx for $DOMAIN..."
# Create Nginx config
run_remote "cat > /etc/nginx/sites-available/uwu-my-opencode << 'EOF'
server {
    listen 80;
    server_name $DOMAIN;

    # Daemon API
    location /api/ {
        proxy_pass http://127.0.0.1:18080/;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # Terminal (ttyd)
    location / {
        proxy_pass http://127.0.0.1:7681;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection \"upgrade\";
        proxy_read_timeout 86400;
    }

    # Health endpoint
    location /health {
        proxy_pass http://127.0.0.1:18080/health;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
    }
}
EOF"

echo "📦 Step 3: Enabling Nginx site..."
run_remote "ln -sf /etc/nginx/sites-available/uwu-my-opencode /etc/nginx/sites-enabled/"
run_remote "rm -f /etc/nginx/sites-enabled/default"
run_remote "nginx -t"
run_remote "systemctl restart nginx"

echo "📦 Step 4: Setting up SSL with Certbot..."
echo "Note: Certbot will prompt for email and agreement. Using non-interactive mode..."
run_remote "certbot --nginx -d $DOMAIN --non-interactive --agree-tos --email admin@$DOMAIN --redirect || echo 'Certbot may fail if DNS not pointing here yet'"

echo "📦 Step 5: Creating systemd service for uwu-daemon..."
run_remote "cat > /etc/systemd/system/uwu-daemon.service << 'EOF'
[Unit]
Description=uwu-my-opencode daemon
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$REMOTE_DIR/daemon
Environment=UWU_EXECUTE_COMMANDS=true
ExecStart=$REMOTE_DIR/daemon/target/release/uwu-daemon \\
  --host 127.0.0.1 \\
  --port 18080 \\
  --workspace-root $REMOTE_DIR/workspaces \\
  --state-file $REMOTE_DIR/.uwu-state.json \\
  --ttyd-port-start 7681 \\
  --tmux-bin $REMOTE_DIR/build/tmux/bin/tmux \\
  --opencode-repo $REMOTE_DIR/opencode \\
  --oh-my-opencode-repo $REMOTE_DIR/oh-my-opencode
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
EOF"

echo "📦 Step 6: Enabling and starting services..."
run_remote "systemctl daemon-reload"
run_remote "systemctl enable uwu-daemon"
run_remote "systemctl start uwu-daemon"

echo "✅ Domain configuration complete!"
echo ""
echo "📋 DNS Configuration Needed:"
echo "1. Go to your DNS provider (Namecheap, Cloudflare, etc.)"
echo "2. Add A record for $DOMAIN pointing to: $CURRENT_IP"
echo "3. Wait for DNS propagation (5-60 minutes)"
echo ""
echo "🔗 Once DNS propagates, access at:"
echo "   - https://$DOMAIN/ (terminal)"
echo "   - https://$DOMAIN/api/health (health check)"
echo "   - https://$DOMAIN/api/ (daemon API)"
echo ""
echo "🔧 To test locally before DNS:"
echo "   Add to /etc/hosts: $CURRENT_IP $DOMAIN"