<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/lib/repository.php') || !is_file($source.'/modules/contacts/hm-contacts.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to the Cypht v2.12.0 source directory\n");
    exit(1);
}

require $source.'/lib/version.php';
if (!defined('CYPHT_VERSION') || CYPHT_VERSION !== '2.12.0') {
    throw new RuntimeException('Contact repository test requires Cypht 2.12.0');
}
define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }
require $source.'/lib/repository.php';
require $source.'/modules/contacts/hm-contacts.php';
require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/contacts.php';

class TestContactConfig {
    private $data = array();
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
    public function dump() { return $this->data; }
}
class TestContactSession {
    private $data = array('username' => 'alice');
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
}

$config = new TestContactConfig();
$session = new TestContactSession();
$store = new Hm_Contact_Store();
$store->init($config, $session);
$store->add_contact(array('id' => 'local-test-1', 'source' => 'local', 'display_name' => 'Alice', 'email_address' => 'alice@example.com'));
$contact = gateway_safe_contact('local-test-1', $store->get('local-test-1'));
if ($contact['name'] !== 'Alice' || $contact['email'] !== 'alice@example.com') {
    throw new RuntimeException('Cypht contact creation or Bridge normalization failed');
}
if (!$store->update_contact('local-test-1', array('display_name' => 'Alice Updated'))) {
    throw new RuntimeException('Cypht contact update failed');
}
if (gateway_safe_contact('local-test-1', $store->get('local-test-1'))['name'] !== 'Alice Updated') {
    throw new RuntimeException('Cypht contact update was not persisted');
}
if (!$store->delete('local-test-1') || $store->get('local-test-1')) {
    throw new RuntimeException('Cypht contact deletion failed');
}

if ($session->get('user_data') === null) {
    throw new RuntimeException('Cypht contact repository did not save user data to the session');
}
echo "Cypht v2.12.0 local contact repository CRUD ok\n";