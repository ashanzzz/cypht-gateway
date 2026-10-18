<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/lib/version.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to the Cypht source directory\n");
    exit(1);
}
require $source.'/lib/version.php';
if (!defined('CYPHT_VERSION') || !in_array(CYPHT_VERSION, array('2.12.0', '2.13.0'), true)) {
    throw new RuntimeException('Bridge ping test requires Cypht 2.12.0 or 2.13.0');
}

function hm_exists($name) { return function_exists($name); }
class TestBridgeResponseEnded extends RuntimeException {}
class Hm_Functions {
    public static function cease() { throw new TestBridgeResponseEnded(); }
}
class Hm_Handler_Module {}

require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/handler_modules.php';

ob_start();
try {
    (new Hm_Handler_gateway_ping())->process();
} catch (TestBridgeResponseEnded $error) {
    // The real Cypht response stops dispatch here.
}
$result = json_decode(ob_get_clean(), true);
if (!is_array($result) || ($result['ok'] ?? false) !== true) {
    throw new RuntimeException('Bridge ping response is not valid JSON');
}
$data = $result['data'] ?? array();
if (($data['status'] ?? null) !== 'ok' || !in_array($data['cypht_version'] ?? null, array('2.12.0', '2.13.0'), true)) {
    throw new RuntimeException('Bridge ping did not report the Cypht 2.12.0 baseline');
}
if (($data['bridge_version'] ?? null) !== trim(file_get_contents(__DIR__.'/../VERSION'))) {
    throw new RuntimeException('Bridge ping did not report the product version');
}
if (($data['durable_user_config'] ?? true) !== false) {
    throw new RuntimeException('Stock Cypht test handler unexpectedly advertised durable storage');
}
echo "Cypht 2.12.0 Bridge ping contract ok\n";