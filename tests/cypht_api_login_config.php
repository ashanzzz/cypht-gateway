<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/lib/version.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to the Cypht v2.12.0 source directory\n");
    exit(1);
}
require $source.'/lib/version.php';
if (CYPHT_VERSION !== '2.12.0') {
    throw new RuntimeException('API login config test requires Cypht 2.12.0');
}

function env($key, $default = null) {
    return getenv($key) ?: $default;
}

$key = str_repeat('ci-only-key-', 4);
putenv('API_LOGIN_KEY='.$key);
$settings = array_merge(
    require $source.'/config/app.php',
    require __DIR__.'/../docker/cypht-gateway-config.php'
);
if (($settings['api_login_key'] ?? null) !== $key) {
    throw new RuntimeException('Cypht API login key is not loaded from the runtime environment');
}

echo "Cypht 2.12.0 API login config merge ok\n";
