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

function gateway_test_capabilities($handler) {
    ob_start();
    try {
        $handler->process();
    } catch (TestBridgeCapabilitiesEnded $error) {}
    $result = json_decode(ob_get_clean(), true);
    return $result['data']['capabilities'] ?? array();
}

function gateway_assert_capability($capabilities, $capability, $expected) {
    $present = in_array($capability, $capabilities, true);
    if ($present !== $expected) {
        $state = $expected ? 'missing' : 'unexpectedly advertised';
        throw new RuntimeException("Capability {$capability} is {$state}");
    }
}

$durable_write_capabilities = array(
    'contacts.write.v1',
    'tags.write.v1',
    'calendar.write.v1',
    'saved_searches.write.v1',
);

// 1. Optional modules and durable writes must fail closed.
$caps = gateway_test_capabilities(new TestPingHandlerNoFeeds());
gateway_assert_capability($caps, 'feeds.read.v1', false);
gateway_assert_capability($caps, 'user_config.durable.v1', false);
foreach ($durable_write_capabilities as $capability) {
    gateway_assert_capability($caps, $capability, false);
}
gateway_assert_capability($caps, 'mail.read.v1', true);
gateway_assert_capability($caps, 'accounts.read.v1', true);

// 2. Supported optional modules and safe durable storage may advertise writes.
$caps = gateway_test_capabilities(new TestPingHandlerWithFeedsAndDurable());
gateway_assert_capability($caps, 'feeds.read.v1', true);
gateway_assert_capability($caps, 'user_config.durable.v1', true);
foreach ($durable_write_capabilities as $capability) {
    gateway_assert_capability($caps, $capability, true);
}

class TestPingHandlerAccountModules extends Hm_Handler_gateway_ping {
    public $user_config;
    public function __construct() { $this->user_config = new TestMockDurableConfig(); }
    public function module_is_supported($name) {
        return in_array($name, array('imap', 'smtp', 'profiles'), true);
    }
}
$caps = gateway_test_capabilities(new TestPingHandlerNoFeeds());
gateway_assert_capability($caps, 'accounts.test.v1', false);
gateway_assert_capability($caps, 'accounts.write.v1', false);
$caps = gateway_test_capabilities(new TestPingHandlerAccountModules());
gateway_assert_capability($caps, 'accounts.test.v1', true);
gateway_assert_capability($caps, 'accounts.write.v1', true);

echo "Bridge capability present/missing behavioral test ok\n";
