<?php

define('DEBUG_MODE', true);

function hm_exists($name) { return function_exists($name); }
function handler_source($name) {}
function setup_base_ajax_page($name, $source) {}
function add_handler(...$args) {}

require __DIR__.'/../cypht-module/gateway/functions.php';
$filters = require __DIR__.'/../cypht-module/gateway/setup.php';

foreach (array(
    'HTTP_X_CYPHT_GATEWAY_KEY',
    'HTTP_X_CYPHT_GATEWAY_FILENAME',
    'HTTP_X_CYPHT_GATEWAY_CONTENT_TYPE',
    'CONTENT_LENGTH'
) as $header) {
    if (!isset($filters['allowed_server'][$header])) {
        throw new RuntimeException('Bridge header is not allowlisted: '.$header);
    }
}

class TestGatewaySession {
    public $saves = 0;
    private $active = true;

    public function is_active() { return $this->active; }

    public function end() {
        $this->saves++;
        $this->active = false;
    }
}

$session = new TestGatewaySession();
Hm_Gateway_Response::bind_session($session);
Hm_Gateway_Response::close_session();
Hm_Gateway_Response::close_session();
if ($session->saves !== 1) {
    throw new RuntimeException('Bridge session was not closed exactly once');
}

require __DIR__.'/../cypht-module/gateway/contacts.php';
$contactFields = gateway_contact_fields(array('name' => 'Alice', 'email' => 'alice@example.com'), true);
if ($contactFields['display_name'] !== 'Alice' || $contactFields['email_address'] !== 'alice@example.com') {
    throw new RuntimeException('Contact fields were mapped incorrectly');
}
$localContact = new class {
    public function value($name, $default = false) {
        return array('source' => 'local', 'display_name' => 'Alice', 'email_address' => 'alice@example.com')[$name] ?? $default;
    }
};
if (gateway_safe_contact('internal', $localContact)['email'] !== 'alice@example.com') {
    throw new RuntimeException('Local contact was not normalized');
}
foreach (array(array('42', '42'), array(42, '42'), array(true, null), array(array('NO', 'APPEND failed'), null)) as $case) {
    if (gateway_safe_stored_uid($case[0]) !== $case[1]) {
        throw new RuntimeException('Unsafe Cypht APPEND result became a public message UID');
    }
}require __DIR__.'/../cypht-module/gateway/tags.php';
class TestCapabilityResponseEnded extends RuntimeException {}
class Hm_Functions {
    public static function cease() { throw new TestCapabilityResponseEnded(); }
}
$disabledTags = new class {
    public function module_is_supported($name) { return false; }
};
ob_start();
try {
    gateway_tags_init($disabledTags);
} catch (TestCapabilityResponseEnded $error) {
}
$unavailable = json_decode(ob_get_clean(), true);
if (http_response_code() !== 501 || ($unavailable['error'] ?? null) !== 'tags capability is unavailable') {
    throw new RuntimeException('Disabled Tags module did not fail as capability unavailable');
}
http_response_code(200);
if (!gateway_action_succeeded(true) || !gateway_action_succeeded(array('status' => true)) ||
    gateway_action_succeeded(false) || gateway_action_succeeded(array('status' => false))) {
    throw new RuntimeException('Cypht action results were normalized incorrectly');
}
$move = array('status' => true, 'responses' => array(array('oldUid' => '42', 'newUid' => '99')));
if (gateway_action_new_uid($move, '42') !== '99' ||
    gateway_action_new_uid($move, '43') !== null ||
    gateway_action_new_uid(array('status' => true, 'responses' => array()), '42') !== null) {
    throw new RuntimeException('MOVE without a verified UID was treated as resolved');
}echo "Bridge filter and session contract ok\n";