#!/bin/bash
# Fix UI routing for uwu-my-opencode deployment
# Updates nginx to serve dashboard at / and terminal at /terminal

set -e

SERVER="149.28.143.214"
USER="root"
PASSWORD="9_xEC279?85n}R{M"
DOMAIN="code.vidwadeseram.com"

echo "🔧 Fixing UI routing for $DOMAIN..."

# Function to run remote commands
run_remote() {
    sshpass -p "$PASSWORD" ssh -o StrictHostKeyChecking=no "$USER@$SERVER" "$1"
}

echo "📋 Current nginx configuration:"
run_remote "cat /etc/nginx/sites-available/uwu-my-opencode | head -30"

echo ""
echo "🔄 Updating nginx configuration..."
run_remote "cat > /tmp/nginx-fixed.conf << 'EOF'
server {
    server_name $DOMAIN;

    # Dashboard/UI (daemon dashboard)
    location / {
        proxy_pass http://127.0.0.1:18080/;
        proxy_http_version 1.1;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    # Terminal (ttyd)
    location /terminal {
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

    listen 443 ssl;
    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;
}
server {
    if (\$host = $DOMAIN) {
        return 301 https://\$host\$request_uri;
    }

    listen 80;
    server_name $DOMAIN;
    return 404;
}
EOF"

run_remote "sudo cp /tmp/nginx-fixed.conf /etc/nginx/sites-available/uwu-my-opencode && sudo nginx -t && sudo systemctl reload nginx"

echo ""
echo "✅ Nginx configuration updated successfully!"
echo ""
echo "🔗 Test endpoints:"
echo "  Dashboard:  https://$DOMAIN/"
echo "  Terminal:   https://$DOMAIN/terminal"
echo "  Health:     https://$DOMAIN/health"
echo ""
echo "🔑 Terminal credentials: admin / admin"
echo ""
echo "📊 Dashboard features:"
echo "  • VM monitoring (CPU, RAM, disk)"
echo "  • Workspace management"
echo "  • Project management"
echo "  • Password reset"
echo "  • Commander chat interface"
echo ""
echo "🎯 Plugin system is already injected via workspace .opencode files"
echo "   Each workspace has its own plugin configuration"