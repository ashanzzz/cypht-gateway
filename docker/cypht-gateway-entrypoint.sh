#!/bin/sh
set -eu

: "${API_LOGIN_KEY:?Cypht API_LOGIN_KEY must be configured}"
: "${GATEWAY_BRIDGE_KEY:?Cypht GATEWAY_BRIDGE_KEY must be configured}"
if [ "$API_LOGIN_KEY" = "$GATEWAY_BRIDGE_KEY" ]; then
    echo "Cypht API_LOGIN_KEY and GATEWAY_BRIDGE_KEY must be different" >&2
    exit 1
fi

user_config_type="${USER_CONFIG_TYPE:-custom:Gateway_User_Config_File}"
user_settings_dir="${USER_SETTINGS_DIR:-/var/lib/hm3/users}"
if [ "$user_config_type" = "file" ]; then
    user_config_type="custom:Gateway_User_Config_File"
    export USER_CONFIG_TYPE="$user_config_type"
fi
if [ "$user_config_type" = "custom:Gateway_User_Config_File" ]; then
    mkdir -p "$user_settings_dir"
    chown www-data:www-data "$user_settings_dir"
fi
exec /usr/local/bin/docker-entrypoint.sh "$@"
