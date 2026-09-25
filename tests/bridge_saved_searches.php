<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/modules/saved_searches/modules.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to Cypht 2.12.0 source\n");
    exit(1);
}
require $source.'/lib/version.php';
if (CYPHT_VERSION !== '2.12.0') throw new RuntimeException('Saved-search Bridge test requires Cypht 2.12.0');

define('DEBUG_MODE', true);
define('DEFAULT_SEARCH_SINCE', '-1 week');
define('DEFAULT_SEARCH_FLD', 'TEXT');
function hm_exists($name) { return function_exists($name); }
class TestSavedSearchResponseEnded extends RuntimeException {}
class Hm_Functions {
    public static function cease($message = '') { throw new TestSavedSearchResponseEnded($message); }
}
class Hm_Output_Module {}
class Hm_IMAP_List { public static function dump($id = null) { return array('7' => array()); } }
class Hm_Handler_Module {
    public $request;
    public $session;
    public $user_config;
    public function module_is_supported($name) { return in_array($name, array('saved_searches', 'imap'), true); }
}
require $source.'/modules/saved_searches/modules.php';
require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/saved_search_handlers.php';

class TestSavedSearchSession {
    private $data;
    private $active = true;
    public function __construct($searches) {
        $this->data = array('username' => 'alice', 'user_data' => array('saved_searches' => $searches));
    }
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
    public function auth($username, $password) { return $username === 'alice' && $password === 'config-key'; }
    public function is_active() { return $this->active; }
    public function end() { $this->active = false; }
}

class TestSavedSearchConfig {
    private $values;
    private $require_write_first;
    private $write_started = false;
    public function __construct($searches, $require_write_first = false) {
        $this->values = array('saved_searches' => $searches);
        $this->require_write_first = $require_write_first;
    }
    public function get($key, $default = null) {
        if ($key === 'saved_searches' && $this->require_write_first && !$this->write_started) {
            throw new RuntimeException('saved searches were read before the durable write lock');
        }
        return $this->values[$key] ?? $default;
    }
    public function set($key, $value) { $this->values[$key] = $value; }
    public function gateway_storage_is_safe() { return true; }
    public function gateway_begin_write($username, $key, $section, $snapshot) {
        if ($username !== 'alice' || $key !== 'config-key' || $section !== 'saved_searches') return false;
        $this->write_started = true;
        return true;
    }
    public function gateway_commit_write($section) { return $this->values[$section] ?? array(); }
    public function gateway_abort_write() {}
    public function stored() { return $this->values['saved_searches'] ?? array(); }
}
class TestDisabledSavedSearchHandler extends Hm_Handler_gateway_saved_searches {
    public function module_is_supported($name) { return false; }
}
function dispatch_saved_search_handler($class, $searches, $payload) {
    http_response_code(200);
    $write = in_array($class, array('Hm_Handler_gateway_saved_search_create', 'Hm_Handler_gateway_saved_search_update', 'Hm_Handler_gateway_saved_search_delete'), true);
    $config = new TestSavedSearchConfig($searches, $write);
    $session = new TestSavedSearchSession($searches);
    $handler = new $class();
    $handler->user_config = $config;
    $handler->session = $session;
    $handler->request = (object)array(
        'server' => array('HTTP_X_CYPHT_GATEWAY_CONFIG_KEY' => 'config-key'),
        'post' => array('payload' => json_encode($payload))
    );
    ob_start();
    try { $handler->process(); } catch (TestSavedSearchResponseEnded $error) {}
    $body = json_decode(ob_get_clean(), true);
    return array(http_response_code(), $body, $config->stored());
}

$advanced = array(
    'terms' => array(array('term' => 'invoice', 'condition' => false)),
    'targets' => array(array('target' => 'TEXT', 'orig' => 'TEXT', 'condition' => false)),
    'sources' => array(array('source' => 'imap_7_494e424f58', 'label' => '<script>alert(1)</script>', 'allFolders' => false, 'subFolders' => false)),
    'times' => array(array('from' => '2026-01-01', 'to' => '2026-12-31')),
    'other' => array('limit' => '100', 'flags' => array(), 'charset' => '')
);
[$status, $body, $stored] = dispatch_saved_search_handler(
    'Hm_Handler_gateway_saved_search_create',
    array(),
    array('name' => 'Advanced', 'type' => 'advanced', 'advanced' => $advanced)
);
if ($status !== 200 || ($stored['Advanced']['data']['sources'][0]['label'] ?? null) !== '<script>alert(1)</script>') {
    throw new RuntimeException('Advanced Cypht source label was not preserved as JSON text');
}
[$status, $body, $stored] = dispatch_saved_search_handler(
    'Hm_Handler_gateway_saved_search_create',
    array(),
    array('name' => 'New', 'type' => 'simple', 'query' => 'invoice', 'since' => '-1 week', 'field' => 'SUBJECT')
);
if ($status !== 200 || !isset($stored['New']) || $stored['New'] !== array('invoice', '-1 week', 'SUBJECT', 'New')) {
    throw new RuntimeException('Simple saved search creation did not map to the Cypht repository');
}
$simple = array('invoice', '-1 week', 'TEXT', 'Invoices');
$existing = array('Invoices' => $simple, 'Work' => array('tasks', 'any', 'SUBJECT', 'Work'));
[$status, $body, $stored] = dispatch_saved_search_handler(
    'Hm_Handler_gateway_saved_search_update',
    $existing,
    array('search_name' => 'Invoices', 'name' => 'Work')
);
if ($status !== 409 || ($body['error'] ?? null) !== 'saved search name already exists' || $stored !== $existing) {
    throw new RuntimeException('Rename collision overwrote an existing saved search');
}

[$status, $body, $stored] = dispatch_saved_search_handler(
    'Hm_Handler_gateway_saved_search_update',
    $existing,
    array('search_name' => 'Invoices', 'name' => 'Billing')
);
if ($status !== 200 || !isset($stored['Billing']) || isset($stored['Invoices']) ||
    $stored['Billing'][0] !== 'invoice' || $stored['Billing'][3] !== 'Billing') {
    throw new RuntimeException('Durable saved-search rename failed');
}

[$status, $body, $stored] = dispatch_saved_search_handler(
    'Hm_Handler_gateway_saved_search_delete',
    $existing,
    array('search_name' => 'Invoices', 'confirm' => false)
);
if ($status !== 400 || !isset($stored['Invoices'])) {
    throw new RuntimeException('Saved-search deletion did not require confirmation');
}

[$status, $body, $stored] = dispatch_saved_search_handler(
    'Hm_Handler_gateway_saved_search_delete',
    $existing,
    array('search_name' => 'Invoices', 'confirm' => true)
);
if ($status !== 200 || isset($stored['Invoices'])) {
    throw new RuntimeException('Confirmed saved-search deletion failed');
}

$disabled = new TestDisabledSavedSearchHandler();
$disabled->user_config = new TestSavedSearchConfig($existing);
$disabled->session = new TestSavedSearchSession($existing);
$disabled->request = (object)array('server' => array(), 'post' => array());
http_response_code(200);
ob_start();
try { $disabled->process(); } catch (TestSavedSearchResponseEnded $error) {}
$disabledBody = json_decode(ob_get_clean(), true);
if (http_response_code() !== 501 || ($disabledBody['error'] ?? null) !== 'saved searches capability is unavailable') {
    throw new RuntimeException('Disabled saved-search module did not fail as unavailable');
}
echo "Cypht 2.12.0 Saved-search Bridge conflict and confirmation tests ok\n";
