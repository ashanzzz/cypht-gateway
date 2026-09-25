#!/bin/sh
set -eu

mkdir -p /var/lib/cypht-gateway

# 1. Manage persistent secrets: auto-generate unguessable keys if not supplied
if [ -z "${GATEWAY_MASTER_KEY:-}" ]; then
    KEY_FILE="/var/lib/cypht-gateway/master.key"
    if [ -f "$KEY_FILE" ] && [ -s "$KEY_FILE" ]; then
        GATEWAY_MASTER_KEY="$(cat "$KEY_FILE")"
    else
        GATEWAY_MASTER_KEY="$(head -c 32 /dev/urandom | base64 | tr -d '\n')"
        printf '%s' "$GATEWAY_MASTER_KEY" > "$KEY_FILE"
        chmod 0600 "$KEY_FILE"
    fi
    export GATEWAY_MASTER_KEY
fi

if [ -z "${API_LOGIN_KEY:-}" ]; then
    KEY_FILE="/var/lib/cypht-gateway/api_login.key"
    if [ -f "$KEY_FILE" ] && [ -s "$KEY_FILE" ]; then
        API_LOGIN_KEY="$(cat "$KEY_FILE")"
    else
        API_LOGIN_KEY="$(head -c 24 /dev/urandom | base64 | tr -d '\n/+=')"
        printf '%s' "$API_LOGIN_KEY" > "$KEY_FILE"
        chmod 0600 "$KEY_FILE"
    fi
    export API_LOGIN_KEY
fi
export CYPHT_API_LOGIN_KEY="$API_LOGIN_KEY"

if [ -z "${GATEWAY_BRIDGE_KEY:-}" ]; then
    KEY_FILE="/var/lib/cypht-gateway/bridge.key"
    if [ -f "$KEY_FILE" ] && [ -s "$KEY_FILE" ]; then
        GATEWAY_BRIDGE_KEY="$(cat "$KEY_FILE")"
    else
        GATEWAY_BRIDGE_KEY="$(head -c 24 /dev/urandom | base64 | tr -d '\n/+=')"
        printf '%s' "$GATEWAY_BRIDGE_KEY" > "$KEY_FILE"
        chmod 0600 "$KEY_FILE"
    fi
    export GATEWAY_BRIDGE_KEY
fi
export CYPHT_BRIDGE_KEY="$GATEWAY_BRIDGE_KEY"

export USER_CONFIG_TYPE="${USER_CONFIG_TYPE:-custom:Gateway_User_Config_File}"
export CYPHT_BASE_URL="${CYPHT_BASE_URL:-http://127.0.0.1:80/}"
export GATEWAY_BIND="${GATEWAY_BIND:-0.0.0.0:18080}"
export GATEWAY_DB_PATH="${GATEWAY_DB_PATH:-/var/lib/cypht-gateway/gateway.db}"

# 2. Append gateway services to supervisord configuration if not present
if ! grep -q "program:cypht-gateway" /etc/supervisord.conf; then
    printf '\n' >> /etc/supervisord.conf
    cat /etc/supervisor/conf.d/supervisord-gateway.conf >> /etc/supervisord.conf
fi

exec /usr/local/bin/cypht-gateway-entrypoint.sh "$@"