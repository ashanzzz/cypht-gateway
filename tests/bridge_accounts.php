<?php

define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }
class TestAccountResponseEnded extends RuntimeException {}
class Hm_Functions { public static function cease($message = '') { throw new TestAccountResponseEnded($message); } }
class Hm_Handler_Module {}

require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/account_handlers.php';

function assert_true($value, $message) {
    if (!$value) throw new RuntimeException($message);
}

$profiles = array(
    'owned' => array('id' => 'owned', 'imap_id' => 'imap-a', 'smtp_id' => 'smtp-a', 'default' => true),
    'other' => array('id' => 'other', 'imap_id' => 'imap-b', 'smtp_id' => 'smtp-b', 'default' => false),
);
assert_true(gateway_account_profile_dependencies($profiles, 'imap-a', 'smtp-a', array('owned')) === array(), 'exclusive bundle was treated as shared');
$profiles['shared'] = array('id' => 'shared', 'imap_id' => 'imap-c', 'smtp_id' => 'smtp-a', 'default' => false);
$deps = gateway_account_profile_dependencies($profiles, 'imap-a', 'smtp-a', array('owned'));
assert_true(count($deps) === 1 && $deps[0]['resource'] === 'smtp', 'shared SMTP dependency was not detected');

$remaining = array(
    'a' => array('id' => 'a', 'default' => false),
    'b' => array('id' => 'b', 'default' => false),
);
gateway_account_ensure_default_profile($remaining);
$defaults = array_values(array_filter($remaining, function($profile) { return !empty($profile['default']); }));
assert_true(count($defaults) === 1, 'delete normalization did not preserve exactly one default profile');

// Security mapping is deliberately conservative: plain "none" must not be
// silently represented as STARTTLS-capable Cypht configuration.
http_response_code(200);
ob_start();
try {
    gateway_account_endpoint(array(
        'host' => 'mail.example.test',
        'port' => 143,
        'security' => 'none',
        'username' => 'alice@example.test',
        'password' => 'secret',
    ), 'imap');
    throw new RuntimeException('unsupported plain IMAP security was accepted');
} catch (TestAccountResponseEnded $error) {
}
$body = json_decode(ob_get_clean(), true);
assert_true(http_response_code() === 400 && ($body['ok'] ?? true) === false, 'unsupported security did not fail closed');

echo "Bridge account bundle dependency and validation tests ok\n";
