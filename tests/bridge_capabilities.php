<?php

function hm_exists($name) { return function_exists($name); }
class TestBridgeCapabilitiesEnded extends RuntimeException {}
class Hm_Functions {
    public static function cease() { throw new TestBridgeCapabilitiesEnded(); }
}
class Hm_Handler_Module {}

require_once __DIR__.'/../cypht-module/gateway/functions.php';
require_once __DIR__.'/../cypht-module/gateway/handler_modules.php';

class TestMockDurableConfig {
    public function gateway_storage_is_safe() { return true; }
}

class TestPingHandlerNoFeeds extends Hm_Handler_gateway_ping {
    public $user_config;
    public function module_is_supported($name) { return false; }
}

class TestPingHandlerWithFeedsAndDurable extends Hm_Handler_gateway_ping {
    public $user_config;
    public function __construct() {
        $this->user_config = new TestMockDurableConfig();
    }
    public function module_is_supported($name) { return $name === 'feeds'; }
}

// 1. Test without feeds and without durable config
ob_start();
try {
    (new TestPingHandlerNoFeeds())->process();
} catch (TestBridgeCapabilitiesEnded $error) {}
$result = json_decode(ob_get_clean(), true);
$data = $result['data'] ?? array();
$caps = $data['capabilities'] ?? array();

if (in_array('feeds.read.v1', $caps, true)) {
    throw new RuntimeException('Feeds capability was unexpectedly advertised when module is disabled');
}
if (in_array('user_config.durable.v1', $caps, true) || in_array('contacts.write.v1', $caps, true)) {
    throw new RuntimeException('Write capabilities were unexpectedly advertised without durable user-config');
}
if (!in_array('mail.read.v1', $caps, true) || !in_array('accounts.read.v1', $caps, true)) {
    throw new RuntimeException('Basic read capabilities are missing');
}

// 2. Test with feeds and with durable config
ob_start();
try {
    (new TestPingHandlerWithFeedsAndDurable())->process();
} catch (TestBridgeCapabilitiesEnded $error) {}
$result = json_decode(ob_get_clean(), true);
$data = $result['data'] ?? array();
$caps = $data['capabilities'] ?? array();

if (!in_array('feeds.read.v1', $caps, true)) {
    throw new RuntimeException('Feeds capability was not advertised when module is supported');
}
if (!in_array('user_config.durable.v1', $caps, true) || !in_array('contacts.write.v1', $caps, true) || !in_array('tags.write.v1', $caps, true)) {
    throw new RuntimeException('Durable write capabilities were not advertised when storage is safe');
}

echo "Bridge capability present/missing behavioral test ok\n";
