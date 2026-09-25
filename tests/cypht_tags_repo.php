<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/modules/tags/hm-tags.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to the Cypht v2.12.0 source directory\n");
    exit(1);
}
require $source.'/lib/version.php';
if (!defined('CYPHT_VERSION') || CYPHT_VERSION !== '2.12.0') {
    throw new RuntimeException('Tags repository test requires Cypht 2.12.0');
}

define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }
require $source.'/lib/repository.php';
require $source.'/modules/tags/hm-tags.php';
require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/tags.php';

class TestTagConfig {
    private $data = array();
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
    public function dump() { return $this->data; }
}
class TestTagSession {
    private $data = array('username' => 'alice');
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
}
class TestTagHandler {
    public $user_config;
    public $session;
    public function __construct() {
        $this->user_config = new TestTagConfig();
        $this->session = new TestTagSession();
    }
    public function module_is_supported($name) { return $name === 'tags'; }
}

$handler = new TestTagHandler();
gateway_tags_init($handler);
$id = Hm_Tags::add(array('name' => 'Test', 'color' => Hm_Tags::defaultColor(), 'parent' => null, 'server' => array()));
if (!gateway_safe_tag($id, Hm_Tags::get($id)) || Hm_Tags::get($id)['name'] !== 'Test') {
    throw new RuntimeException('Tag creation or normalization failed');
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
if (!Hm_Tags::edit($id, array('name' => 'Renamed')) || Hm_Tags::get($id)['name'] !== 'Renamed') {
    throw new RuntimeException('Tag update failed');
}
if (!Hm_Tags::del($id) || Hm_Tags::get($id)) {
    throw new RuntimeException('Tag deletion failed');
}
echo "Cypht 2.12.0 Tags repository scoped CRUD ok\n";