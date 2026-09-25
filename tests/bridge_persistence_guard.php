<?php

define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }

class TestBridgeResponseEnded extends RuntimeException {}
class Hm_Functions {
    public static function cease() { throw new TestBridgeResponseEnded(); }
}

require __DIR__.'/../cypht-module/gateway/functions.php';

function expect_persistence_unavailable($handler) {
    http_response_code(200);
    ob_start();
    try {
        gateway_require_durable_user_config($handler, 'contacts');
    } catch (TestBridgeResponseEnded $error) {
    }
    $body = json_decode(ob_get_clean(), true);
    if (http_response_code() !== 501 || ($body['error'] ?? null) !== 'durable user-config writes are unavailable') {
        throw new RuntimeException('Unverified user-config storage did not fail closed');
    }
}

class TestConfigSession {
    public $values = array('user_data' => array('theme' => 'dark'));
    public $ended = false;
    public function get($key, $default = null) { return $this->values[$key] ?? $default; }
    public function set($key, $value) { $this->values[$key] = $value; }
    public function auth($username, $password) { return $username === 'alice' && $password === 'bridge-test-secret'; }
    public function is_active() { return !$this->ended; }
    public function end() { $this->ended = true; }
}

class TestDurableConfig {
    public $begin_args = null;
    public $committed_section = null;
    public $aborted = false;
    public function gateway_storage_is_safe() { return true; }
    public function gateway_begin_write($username, $password, $section, $snapshot) {
        $this->begin_args = array($username, $password, $section, $snapshot);
        return true;
    }
    public function gateway_commit_write($section) {
        $this->committed_section = $section;
        return array(array('id' => 'persisted-contact'));
    }
    public function gateway_abort_write() { $this->aborted = true; }
}

$missing = new stdClass();
expect_persistence_unavailable($missing);

$unsupported = new class {
    public function gateway_storage_is_safe() { return false; }
    public function gateway_begin_write(...$args) { return true; }
};
$handler = new stdClass();
$handler->user_config = $unsupported;
expect_persistence_unavailable($handler);

$broken = new class {
    public function gateway_storage_is_safe() { throw new RuntimeException('private storage error'); }
    public function gateway_begin_write(...$args) { return true; }
};
$handler->user_config = $broken;
expect_persistence_unavailable($handler);

$session = new TestConfigSession();
$handler->session = $session;
$handler->request = (object)array('server' => array('HTTP_X_CYPHT_GATEWAY_CONFIG_KEY' => 'bridge-test-secret'));
$handler->user_config = new TestDurableConfig();
$handler->session = $session;
$handler->session->values['username'] = 'alice';
$snapshot = gateway_require_durable_user_config($handler, 'contacts');
if ($snapshot !== array('theme' => 'dark') ||
    $handler->user_config->begin_args !== array('alice', 'bridge-test-secret', 'contacts', array('theme' => 'dark'))) {
    throw new RuntimeException('Durable write was not bound to the authenticated user session');
}
gateway_commit_durable_user_config($handler, 'contacts', $snapshot);
if ($handler->user_config->committed_section !== 'contacts' ||
    $session->get('user_data') !== array('theme' => 'dark', 'contacts' => array(array('id' => 'persisted-contact')))) {
    throw new RuntimeException('Verified commit did not preserve unrelated session data');
}
if ($handler->user_config->aborted) {
    throw new RuntimeException('Successful commit was incorrectly aborted');
}

http_response_code(200);
echo "Contacts and Tags persistence guard ok\n";