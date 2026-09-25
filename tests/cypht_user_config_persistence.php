<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/lib/config.php') || !is_file($source.'/lib/version.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to the Cypht v2.12.0 source directory\n");
    exit(1);
}
require $source.'/lib/version.php';
if (CYPHT_VERSION !== '2.12.0') throw new RuntimeException('Persistence test requires Cypht 2.12.0');

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
require $source.'/lib/repository.php';
require $source.'/modules/contacts/hm-contacts.php';
require $source.'/modules/tags/hm-tags.php';
require __DIR__.'/../cypht-module/gateway/bootstrap.php';

class TestUserSettingsConfig {
    public $user_defaults = array();
    private $values;
    public function __construct($directory, $encrypted = true) {
        $this->values = array(
            'user_settings_dir' => $directory,
            'single_server_mode' => !$encrypted,
            'auth_type' => 'IMAP',
            'user_config_type' => 'custom:Gateway_User_Config_File'
        );
    }
    public function get($name, $default = false) { return $this->values[$name] ?? $default; }
}
class TestUserSettingsSession {
    private $data = array();
    public function get($name, $default = null) { return $this->data[$name] ?? $default; }
    public function set($name, $value) { $this->data[$name] = $value; }
}
class TestUserSettingsHandler {
    public $user_config;
    public $session;
    public function __construct($user_config, $session) {
        $this->user_config = $user_config;
        $this->session = $session;
    }
}
function test_directory($label) {
    $path = sys_get_temp_dir().'/cypht-gateway-'.$label.'-'.bin2hex(random_bytes(8));
    if (!mkdir($path, 0700)) throw new RuntimeException('Could not create temporary test directory');
    return $path;
}
function new_user_config($site, $password) {
    $config = load_user_config_object($site);
    if (!$config instanceof Gateway_User_Config_File || !$config->gateway_storage_is_safe()) {
        throw new RuntimeException('Cypht did not load the verified file storage adapter');
    }
    $config->load('alice', $password);
    return $config;
}
function begin_test_write($config, $session, $password, $section) {
    if (!$config->gateway_begin_write('alice', $password, $section, $session->get('user_data', array()))) {
        throw new RuntimeException('Could not start user-config transaction: '.$section);
    }
}
function finish_test_write($config, $session, $section) {
    $values = $config->gateway_commit_write($section);
    if (!is_array($values)) throw new RuntimeException('User-config commit readback failed: '.$section);
    $snapshot = $session->get('user_data', array());
    $snapshot[$section] = $values;
    $session->set('user_data', $snapshot);
    return $values;
}

$password = 'test-only-user-config-key';
$directory = test_directory('encrypted');
$site = new TestUserSettingsConfig($directory, true);
$config = new_user_config($site, $password);
$session = new TestUserSettingsSession();
$session->set('username', 'alice');
$session->set('user_data', $config->dump());
begin_test_write($config, $session, $password, 'contacts');
$store = new Hm_Contact_Store();
$store->init($config, $session);
$store->add_contact(array(
    'id' => 'persisted-contact-1',
    'source' => 'local',
    'display_name' => 'Alice',
    'email_address' => 'alice@example.com'
));
finish_test_write($config, $session, 'contacts');

$readback = new_user_config($site, $password);
$readStore = new Hm_Contact_Store();
$readStore->init($readback, new TestUserSettingsSession());
$contacts = array_map(function($contact) { return $contact->export(); }, $readStore->getAll());
if (!isset($contacts['persisted-contact-1']) || $contacts['persisted-contact-1']['display_name'] !== 'Alice') {
    throw new RuntimeException('Fresh encrypted config load did not read the saved Contact');
}

$tagSession = new TestUserSettingsSession();
$tagSession->set('username', 'alice');
$tagSession->set('user_data', $readback->dump());
begin_test_write($readback, $tagSession, $password, 'tags');
$tagHandler = new TestUserSettingsHandler($readback, $tagSession);
Hm_Tags::init($tagHandler);
$tagId = Hm_Tags::add(array('name' => 'Work', 'color' => Hm_Tags::defaultColor(), 'parent' => null, 'server' => array()));
finish_test_write($readback, $tagSession, 'tags');
$tagReadback = new_user_config($site, $password);
$storedTags = $tagReadback->get('tags', array());
if (!isset($storedTags[$tagId]) || $storedTags[$tagId]['name'] !== 'Work') {
    throw new RuntimeException('Fresh encrypted config load did not read the saved Tag');
}

$writerA = new_user_config($site, $password);
$writerB = new_user_config($site, $password);
$writerA->set('gateway_test_value', 'first');
$writerA->save('alice', $password);
$writerB->set('gateway_other_value', 'stale');
try {
    $writerB->save('alice', $password);
    throw new RuntimeException('A stale concurrent writer unexpectedly replaced settings');
} catch (Gateway_User_Config_Conflict $error) {
}
$conflictReadback = new_user_config($site, $password);
if ($conflictReadback->get('gateway_test_value') !== 'first' ||
    $conflictReadback->get('gateway_other_value', null) !== null) {
    throw new RuntimeException('A stale writer changed durable user settings');
}

$wrongKey = new_user_config($site, 'wrong-password');
if (!$wrongKey->decrypt_failed) throw new RuntimeException('Wrong user-config key was not detected');
try {
    $wrongKey->save('alice', 'wrong-password');
    throw new RuntimeException('A failed decryption was allowed to overwrite user settings');
} catch (Gateway_User_Config_Conflict $error) {
}

$plainDirectory = test_directory('plain');
$plainSite = new TestUserSettingsConfig($plainDirectory, false);
$plain = new_user_config($plainSite, false);
$plain->set('theme_setting', 'dark');
$plainReadback = new_user_config($plainSite, false);
if ($plainReadback->get('theme_setting') !== 'dark') {
    throw new RuntimeException('Unencrypted Cypht set() did not persist user settings');
}
$stalePlain = new_user_config($plainSite, false);
$freshPlain = new_user_config($plainSite, false);
$freshPlain->set('theme_setting', 'light');
try {
    $stalePlain->set('other_setting', 'stale');
    throw new RuntimeException('A stale unencrypted writer unexpectedly replaced settings');
} catch (Gateway_User_Config_Conflict $error) {
}
$stalePlain->reload($stalePlain->dump(), 'alice');
if ($stalePlain->get('theme_setting') !== 'light') {
    throw new RuntimeException('Unencrypted reload did not use the durable snapshot');
}

echo "Cypht 2.12.0 file user-config concurrency and readback tests ok\n";