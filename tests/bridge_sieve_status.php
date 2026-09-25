<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/lib/version.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to Cypht 2.12.0 source\n");
    exit(1);
}
require $source.'/lib/version.php';
if (CYPHT_VERSION !== '2.12.0') throw new RuntimeException('Sieve status test requires Cypht 2.12.0');
define('DEBUG_MODE', true);
define('DEFAULT_ENABLE_SIEVE_FILTER', false);
function hm_exists($name) { return function_exists($name); }
class TestSieveResponseEnded extends RuntimeException {}
class Hm_Functions { public static function cease($message = '') { throw new TestSieveResponseEnded($message); } }
class Hm_Handler_Module {
    public $request;
    public $session;
    public $user_config;
    public function module_is_supported($name) { return $name === 'sievefilters'; }
}
class Hm_IMAP_List {
    public static function dump($id = null) {
        $accounts = array(
            'a1' => array('name' => 'Configured', 'type' => 'imap', 'sieve_config_host' => 'sieve.example.invalid'),
            'a2' => array('name' => 'NotConfigured', 'type' => 'imap')
        );
        return $id === null ? $accounts : ($accounts[$id] ?? false);
    }
}
require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/sieve_handlers.php';

$handler = new Hm_Handler_gateway_sieve_status();
$handler->user_config = new class {
    public function get($key, $default = null) { return $key === 'enable_sieve_filter_setting' ? true : $default; }
};
$handler->session = new class {
    public function is_active() { return true; }
    public function end() {}
};
$handler->request = (object)array('get' => array(), 'post' => array(), 'server' => array());
ob_start();
try { $handler->process(); } catch (TestSieveResponseEnded $error) {}
$result = json_decode(ob_get_clean(), true);
$data = $result['data'] ?? array();
if (($result['ok'] ?? false) !== true || count($data) !== 2) {
    throw new RuntimeException('Sieve status did not return all configured accounts');
}
if (($data[0]['configured'] ?? null) !== true || ($data[0]['enabled'] ?? null) !== true || array_key_exists('host', $data[0])) {
    throw new RuntimeException('Sieve status exposed the wrong data');
}
if (($data[1]['configured'] ?? null) !== false) {
    throw new RuntimeException('Sieve unconfigured state was not reported');
}
echo "Cypht 2.12.0 redacted Sieve status contract ok\n";
