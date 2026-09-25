<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/modules/tags/hm-tags.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to the Cypht v2.12.0 source directory\n");
    exit(1);
}
require $source.'/lib/version.php';
if (CYPHT_VERSION !== '2.12.0') throw new RuntimeException('Bridge Tags test requires Cypht 2.12.0');
define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }
class TestBridgeResponseEnd extends RuntimeException {}
class Hm_Functions { public static function cease() { throw new TestBridgeResponseEnd(); } }
class TestTagConfig {
    private $data = array();
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
    public function dump() { return $this->data; }
}
class TestTagSession {
    private $data = array('username' => 'alice', 'user_data' => array());
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
    public function is_active() { return true; }
    public function end() {}
}
class Hm_Handler_Module {
    public $user_config;
    public $session;
    public $request;
    public function module_is_supported($name) { return $name === 'tags'; }
}
class TestTagHandler {
    public $user_config;
    public $session;
    public $request;
    public function __construct($config, $session) {
        $this->user_config = $config;
        $this->session = $session;
        $this->request = (object)array('server' => array());
    }
    public function module_is_supported($name) { return $name === 'tags'; }
}
require $source.'/lib/repository.php';
require $source.'/modules/tags/hm-tags.php';
require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/tags.php';
require __DIR__.'/../cypht-module/gateway/tag_handlers.php';

$config = new TestTagConfig();
$session = new TestTagSession();
$handler = new TestTagHandler($config, $session);
gateway_tags_init($handler);
$id = Hm_Tags::add(array('name' => 'Work', 'color' => Hm_Tags::defaultColor(), 'parent' => null, 'server' => array()));
if (!gateway_safe_tag($id, Hm_Tags::get($id)) || Hm_Tags::get($id)['name'] !== 'Work') {
    throw new RuntimeException('Tag normalization failed');
}
Hm_Tags::addMessage($id, 'account-a', 'INBOX', '42');
Hm_Tags::addMessage($id, 'account-b', 'INBOX', '42');
if (!gateway_remove_tag_from_message($id, 'account-a', 'INBOX', '42')) {
    throw new RuntimeException('Scoped tag removal failed');
}
$tag = Hm_Tags::get($id);
if (count($tag['server']['account-a']['INBOX']) !== 0 || $tag['server']['account-b']['INBOX'] !== array('42')) {
    throw new RuntimeException('Tag removal changed another account with the same UID');
}
$unsupported = new stdClass();
$guardHandler = new TestTagHandler($unsupported, $session);
ob_start();
try {
    gateway_require_durable_user_config($guardHandler, 'tags');
} catch (TestBridgeResponseEnd $error) {
}
$guard = json_decode(ob_get_clean(), true);
if (http_response_code() !== 501 || ($guard['error'] ?? null) !== 'durable user-config writes are unavailable') {
    throw new RuntimeException('Tag writes did not fail closed without durable storage');
}
http_response_code(200);
if (!Hm_Tags::edit($id, array('name' => 'Renamed')) || Hm_Tags::get($id)['name'] !== 'Renamed') {
    throw new RuntimeException('Tag repository update failed');
}
if (!Hm_Tags::del($id) || Hm_Tags::get($id)) {
    throw new RuntimeException('Tag repository deletion failed');
}
echo "Cypht 2.12.0 Bridge tag scope and write guard ok\n";