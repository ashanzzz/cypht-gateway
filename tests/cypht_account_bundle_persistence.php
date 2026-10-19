<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/lib/config.php') || !is_file($source.'/lib/version.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to the Cypht source directory\n");
    exit(1);
}
require $source.'/lib/version.php';
if (!in_array(CYPHT_VERSION, array('2.12.0', '2.13.0'), true)) {
    throw new RuntimeException('Account bundle persistence test requires Cypht 2.12.0 or 2.13.0');
}

define('VERSION', 0.1);
define('DEBUG_MODE', true);
define('LIBSODIUM', false);
define('DEFAULT_NO_PASSWORD_SAVE', false);
function hm_exists($name) { return function_exists($name); }
class Hm_Debug { public static function add(...$args) {} }
class Hm_Functions {
    public static function random_bytes($size) { return random_bytes($size); }
    public static function function_exists($name) { return function_exists($name); }
    public static function cease($message = '') { throw new RuntimeException($message); }
}
require $source.'/lib/config.php';
require $source.'/lib/format.php';
require $source.'/lib/crypt.php';
require $source.'/lib/crypt_sodium.php';
require __DIR__.'/../cypht-module/gateway/bootstrap.php';

class AccountBundleSiteConfig {
    public $user_defaults = array();
    private $values;
    public function __construct($directory) {
        $this->values = array(
            'user_settings_dir' => $directory,
            'single_server_mode' => false,
            'auth_type' => 'IMAP',
            'user_config_type' => 'custom:Gateway_User_Config_File'
        );
    }
    public function get($name, $default = false) { return $this->values[$name] ?? $default; }
}
class AccountBundleSession {
    private $data = array();
    public function get($name, $default = null) { return $this->data[$name] ?? $default; }
    public function set($name, $value) { $this->data[$name] = $value; }
}
function bundle_temp_dir() {
    $path = sys_get_temp_dir().'/cypht-gateway-account-bundle-'.bin2hex(random_bytes(8));
    if (!mkdir($path, 0700)) throw new RuntimeException('could not create account bundle test directory');
    return $path;
}
function bundle_config($site, $key) {
    $config = load_user_config_object($site);
    if (!$config instanceof Gateway_User_Config_File) throw new RuntimeException('Gateway user config adapter not loaded');
    $config->load('alice', $key);
    return $config;
}

$key = 'test-only-user-config-key';
$site = new AccountBundleSiteConfig(bundle_temp_dir());
$config = bundle_config($site, $key);
$session = new AccountBundleSession();
$session->set('username', 'alice');
$session->set('user_data', $config->dump());
$sections = array('imap_servers', 'profiles', 'smtp_servers');
$snapshot = $session->get('user_data', array());
if (!$config->gateway_begin_write('alice', $key, $sections, $snapshot)) throw new RuntimeException('multi-section begin failed');
$config->gateway_set_write_section('imap_servers', array('imap-a' => array('id' => 'imap-a', 'server' => 'imap.example.test')));
$config->gateway_set_write_section('smtp_servers', array('smtp-a' => array('id' => 'smtp-a', 'server' => 'smtp.example.test')));
$config->gateway_set_write_section('profiles', array('prf-a' => array('id' => 'prf-a', 'imap_id' => 'imap-a', 'smtp_id' => 'smtp-a')));
$readback = $config->gateway_commit_write($sections);
if (!is_array($readback) || !isset($readback['imap_servers']['imap-a'], $readback['smtp_servers']['smtp-a'], $readback['profiles']['prf-a'])) {
    throw new RuntimeException('multi-section commit readback failed');
}
$fresh = bundle_config($site, $key);
if (!isset($fresh->get('imap_servers', array())['imap-a']) ||
    !isset($fresh->get('smtp_servers', array())['smtp-a']) ||
    !isset($fresh->get('profiles', array())['prf-a'])) {
    throw new RuntimeException('fresh load did not observe complete account bundle');
}

// Stale snapshots must fail before any mutation is accepted.
$writerA = bundle_config($site, $key);
$writerB = bundle_config($site, $key);
$stale = $writerB->dump();
$writerA->set('smtp_servers', array('smtp-new' => array('id' => 'smtp-new')));
try {
    $writerB->gateway_begin_write('alice', $key, $sections, $stale);
    throw new RuntimeException('stale multi-section writer was accepted');
} catch (Gateway_User_Config_Conflict $error) {
}

echo "Cypht account bundle multi-section persistence and conflict tests ok\n";
