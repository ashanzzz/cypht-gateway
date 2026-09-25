<?php

define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }
class TestFeedResponseEnded extends RuntimeException {}
class Hm_Functions { public static function cease($message = '') { throw new TestFeedResponseEnded($message); } }
class Hm_Handler_Module {
    public $request;
    public $session;
    public $user_config;
    public function module_is_supported($name) { return $name === 'feeds'; }
}
class Hm_Feed_List {
    public static function init($config, $session) {}
    public static function dump($id = null) {
        $rows = array(
            '7' => array('id' => '7', 'name' => 'Example', 'url' => 'https://example.com/feed.xml', 'server' => 'hidden')
        );
        return $id === null ? $rows : ($rows[$id] ?? false);
    }
}
require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/feed_handlers.php';

$handler = new Hm_Handler_gateway_feeds();
$handler->request = (object)array('get' => array(), 'post' => array(), 'server' => array());
$handler->session = new class {
    public function is_active() { return true; }
    public function end() {}
};
$handler->user_config = new class {};
ob_start();
try { $handler->process(); } catch (TestFeedResponseEnded $error) {}
$result = json_decode(ob_get_clean(), true);
$data = $result['data'] ?? array();
if (($result['ok'] ?? false) !== true || count($data) !== 1) throw new RuntimeException('Feed metadata list failed');
if (($data[0]['id'] ?? null) !== '7' || ($data[0]['name'] ?? null) !== 'Example' || ($data[0]['url'] ?? null) !== 'https://example.com/feed.xml') {
    throw new RuntimeException('Feed metadata was mapped incorrectly');
}
if (array_key_exists('server', $data[0]) || array_key_exists('object', $data[0])) throw new RuntimeException('Feed runtime fields leaked');

echo "Cypht 2.12.0 Feed metadata Bridge contract ok\n";
