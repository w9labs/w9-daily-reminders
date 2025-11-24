#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
ENV_FILE="$ROOT_DIR/.env"

if [ -f "$ENV_FILE" ]; then
  echo "[w9] Loading environment overrides from $ENV_FILE"
  # shellcheck disable=SC1090
  set -a
  source "$ENV_FILE"
  set +a
fi

SERVICE_NAME="${SERVICE_NAME:-w9-daily-reminders}"
INSTALL_DIR="${INSTALL_DIR:-/opt/$SERVICE_NAME}"
DATA_DIR="${DATA_DIR:-$INSTALL_DIR/data}"
DOMAIN="${DOMAIN:-reminder.w9.nu}"
FRONTEND_DIST="${FRONTEND_DIST:-/var/www/$DOMAIN}"
PORT="${PORT:-8787}"
NEXT_PUBLIC_API_BASE="${NEXT_PUBLIC_API_BASE:-https://$DOMAIN}"

if [ "$(id -u)" -eq 0 ]; then
  SUDO=""
else
  SUDO="sudo"
  $SUDO -n true >/dev/null 2>&1 || { echo "This script requires sudo privileges"; exit 1; }
fi

log() {
  printf "[w9] %s\n" "$1"
}

require_pkg() {
  if ! dpkg -s "$1" >/dev/null 2>&1; then
    log "Installing $1"
    $SUDO apt-get update -qq >/dev/null 2>&1 || true
    $SUDO apt-get install -y "$1" >/dev/null 2>&1
  fi
}

log "Checking system packages"
for pkg in build-essential pkg-config libssl-dev curl nginx ufw; do
  require_pkg "$pkg"
done

ensure_node() {
  if command -v node >/dev/null 2>&1; then
    local major
    major=$(node -v | sed 's/v//' | cut -d. -f1)
    if [ "$major" -ge 18 ]; then
      return 0
    fi
  fi
  log "Installing Node.js 20.x"
  curl -fsSL https://deb.nodesource.com/setup_20.x | $SUDO bash - >/dev/null
  $SUDO apt-get install -y nodejs >/dev/null 2>&1
}

ensure_rust() {
  if ! command -v cargo >/dev/null 2>&1; then
    log "Installing Rust toolchain"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1
    source "$HOME/.cargo/env"
  else
    if [ -f "$HOME/.cargo/env" ]; then
      source "$HOME/.cargo/env"
    fi
  fi
}

ensure_node
ensure_rust

log "Building backend"
cd "$ROOT_DIR/backend"
cargo build --release

log "Building frontend"
cd "$ROOT_DIR/frontend"
if [ -f package-lock.json ]; then
  if ! npm ci --prefer-offline --no-audit; then
    echo "[w9] npm ci failed (likely lockfile drift). Falling back to npm install..."
    npm install --prefer-offline --no-audit || {
      echo "[w9] npm install also failed"
      exit 1
    }
  fi
else
  npm install --prefer-offline --no-audit
fi
NEXT_PUBLIC_API_BASE="$NEXT_PUBLIC_API_BASE" \
NEXT_PUBLIC_MAIL_API_BASE="${NEXT_PUBLIC_MAIL_API_BASE:-${W9_MAIL_API_BASE:-https://w9.nu/api}}" \
NEXT_PUBLIC_TURNSTILE_SITE_KEY="${NEXT_PUBLIC_TURNSTILE_SITE_KEY:-}" \
npm run build
if [ ! -d "out" ]; then
  echo "Missing out/ directory after Next.js build" >&2
  exit 1
fi

log "Stopping existing service"
$SUDO systemctl stop $SERVICE_NAME 2>/dev/null || true

log "Deploying backend binary"
$SUDO mkdir -p "$INSTALL_DIR" "$DATA_DIR"
$SUDO cp "$ROOT_DIR/backend/target/release/w9-daily-reminders" "$INSTALL_DIR/w9-daily-reminders"
$SUDO chmod 750 "$INSTALL_DIR/w9-daily-reminders"
$SUDO chown root:root "$INSTALL_DIR/w9-daily-reminders"

log "Deploying frontend"
$SUDO mkdir -p "$FRONTEND_DIST"
$SUDO rm -rf "$FRONTEND_DIST"/*
$SUDO cp -r "$ROOT_DIR/frontend/out"/* "$FRONTEND_DIST"/
$SUDO chown -R www-data:www-data "$FRONTEND_DIST"

log "Writing environment file"
$SUDO tee /etc/default/$SERVICE_NAME >/dev/null <<ENV
HOST=0.0.0.0
PORT=$PORT
DATA_DIR=$DATA_DIR
CEREBRAS_API_KEY=
CEREBRAS_MODEL=zai-glm-4.6
GOOGLE_CLIENT_ID=${GOOGLE_CLIENT_ID:-}
GOOGLE_CLIENT_SECRET=${GOOGLE_CLIENT_SECRET:-}
GOOGLE_REDIRECT_URI=${GOOGLE_REDIRECT_URI:-https://$DOMAIN/google/callback}
NEXT_PUBLIC_API_BASE=$NEXT_PUBLIC_API_BASE
W9_MAIL_API_BASE=${W9_MAIL_API_BASE:-https://w9.nu/api}
W9_MAIL_SERVICE_TOKEN=${W9_MAIL_SERVICE_TOKEN:-}
TURNSTILE_SITE_KEY=${TURNSTILE_SITE_KEY:-}
TURNSTILE_SECRET=${TURNSTILE_SECRET:-}
NEXT_PUBLIC_MAIL_API_BASE=${NEXT_PUBLIC_MAIL_API_BASE:-https://w9.nu/api}
NEXT_PUBLIC_TURNSTILE_SITE_KEY=${NEXT_PUBLIC_TURNSTILE_SITE_KEY:-}
ENV

log "Configuring systemd unit"
$SUDO tee /etc/systemd/system/$SERVICE_NAME.service >/dev/null <<'UNIT'
[Unit]
Description=W9 Daily Reminders Backend
After=network.target

[Service]
EnvironmentFile=/etc/default/w9-daily-reminders
WorkingDirectory=/opt/w9-daily-reminders
ExecStart=/opt/w9-daily-reminders/w9-daily-reminders
Restart=always
RestartSec=2

[Install]
WantedBy=multi-user.target
UNIT

log "Preparing origin TLS certificate"
SSL_DIR=${ORIGIN_SSL_DIR:-/etc/nginx/ssl/$SERVICE_NAME}
SSL_CERT=${ORIGIN_SSL_CERT:-$SSL_DIR/cert.pem}
SSL_KEY=${ORIGIN_SSL_KEY:-$SSL_DIR/key.pem}

if [ ! -f "$SSL_CERT" ] || [ ! -f "$SSL_KEY" ]; then
  log "Generating self-signed certificate for $DOMAIN at $SSL_DIR"
  $SUDO mkdir -p "$SSL_DIR"
  $SUDO openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
    -keyout "$SSL_KEY" \
    -out "$SSL_CERT" \
    -subj "/CN=$DOMAIN" >/dev/null 2>&1
  $SUDO chmod 600 "$SSL_KEY"
  $SUDO chmod 644 "$SSL_CERT"
fi

log "Configuring nginx"
$SUDO tee /etc/nginx/sites-available/$SERVICE_NAME >/dev/null <<NGINX
server {
    listen 80;
    listen [::]:80;
    server_name $DOMAIN;
    return 301 https://\$host\$request_uri;
}

server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name $DOMAIN;

    ssl_certificate $SSL_CERT;
    ssl_certificate_key $SSL_KEY;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers on;

    root $FRONTEND_DIST;
    index index.html;

    location /api/ {
        proxy_pass http://127.0.0.1:$PORT;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
    }

    location / {
        try_files \$uri \$uri/ \$uri.html /index.html;
    }
}
NGINX
$SUDO ln -sf /etc/nginx/sites-available/$SERVICE_NAME /etc/nginx/sites-enabled/$SERVICE_NAME
$SUDO nginx -t
$SUDO systemctl reload nginx

log "Starting services"
$SUDO systemctl daemon-reload
$SUDO systemctl enable $SERVICE_NAME >/dev/null 2>&1 || true
$SUDO systemctl restart $SERVICE_NAME

log "Deployment complete"
