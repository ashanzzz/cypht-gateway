<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/modules/calendar/hm-calendar.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to Cypht 2.12.0 source\n");
    exit(1);
}
require $source.'/lib/version.php';
if (CYPHT_VERSION !== '2.12.0') throw new RuntimeException('Calendar test requires Cypht 2.12.0');
define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }
class TestCalendarResponseEnded extends RuntimeException {}
class Hm_Functions { public static function cease($message = '') { throw new TestCalendarResponseEnded($message); } }
class Hm_Output_Module {}
class Hm_Handler_Module {
    public $request;
    public $session;
    public $user_config;
    public function module_is_supported($name) { return in_array($name, array('calendar'), true); }
}
require $source.'/modules/calendar/hm-calendar.php';
require __DIR__.'/../cypht-module/gateway/functions.php';
require __DIR__.'/../cypht-module/gateway/calendar_handlers.php';

class TestCalendarSession {
    private $data;
    private $active = true;
    public function __construct($events) { $this->data = array('username' => 'alice', 'user_data' => array('calendar_events' => $events)); }
    public function get($key, $default = null) { return $this->data[$key] ?? $default; }
    public function set($key, $value) { $this->data[$key] = $value; }
    public function auth($username, $password) { return $username === 'alice' && $password === 'calendar-key'; }
    public function is_active() { return $this->active; }
    public function end() { $this->active = false; }
}
class TestCalendarConfig {
    private $values;
    public function __construct($events) { $this->values = array('calendar_events' => $events); }
    public function get($key, $default = null) { return $this->values[$key] ?? $default; }
    public function set($key, $value) { $this->values[$key] = $value; }
    public function gateway_storage_is_safe() { return true; }
    public function gateway_begin_write($username, $key, $section, $snapshot) { return $username === 'alice' && $key === 'calendar-key' && $section === 'calendar_events'; }
    public function gateway_commit_write($section) { return $this->values[$section] ?? array(); }
    public function gateway_abort_write() {}
    public function stored() { return $this->values['calendar_events'] ?? array(); }
}
function dispatch_calendar($class, $events, $payload, $get = array()) {
    http_response_code(200);
    $config = new TestCalendarConfig($events);
    $handler = new $class();
    $handler->user_config = $config;
    $handler->session = new TestCalendarSession($events);
    $handler->request = (object)array(
        'server' => array('HTTP_X_CYPHT_GATEWAY_CONFIG_KEY' => 'calendar-key'),
        'get' => $get,
        'post' => array('payload' => json_encode($payload))
    );
    ob_start();
    try { $handler->process(); } catch (TestCalendarResponseEnded $error) {}
    return array(http_response_code(), json_decode(ob_get_clean(), true), $config->stored());
}

[$status, $body, $stored] = dispatch_calendar(
    'Hm_Handler_gateway_calendar_create',
    array(),
    array('title' => 'Team meeting', 'description' => 'Plain text', 'starts_at' => 1790240400, 'repeat_interval' => 'week')
);
if ($status !== 200 || count($stored) !== 1 || ($stored[0]['repeat_interval'] ?? null) !== 'week') {
    throw new RuntimeException('Calendar creation did not persist the Cypht event');
}
$eventId = $stored[0]['id'];
$duplicate = $stored;
$duplicate[] = $stored[0];
[$status, $body, $ignored] = dispatch_calendar(
    'Hm_Handler_gateway_calendar_delete',
    $duplicate,
    array('event_id' => $eventId, 'confirm' => true)
);
if ($status !== 409) throw new RuntimeException('Ambiguous calendar event identity was not rejected');
[$status, $body, $stored] = dispatch_calendar(
    'Hm_Handler_gateway_calendar_events',
    $stored,
    array(),
    array('start_at' => 1790240400, 'end_at' => 1790326800)
);
if ($status !== 200 || count($body['data'] ?? array()) !== 1 || ($body['data'][0]['id'] ?? null) !== $eventId) {
    throw new RuntimeException('Calendar range listing did not return the event');
}
[$status, $body, $stored] = dispatch_calendar(
    'Hm_Handler_gateway_calendar_delete',
    $stored,
    array('event_id' => $eventId, 'confirm' => false)
);
if ($status !== 400 || count($stored) !== 1) throw new RuntimeException('Calendar delete did not require confirmation');
[$status, $body, $stored] = dispatch_calendar(
    'Hm_Handler_gateway_calendar_delete',
    $stored,
    array('event_id' => $eventId, 'confirm' => true)
);
if ($status !== 200 || count($stored) !== 0) throw new RuntimeException('Calendar deletion failed');

echo "Cypht 2.12.0 Calendar create, range, persistence, and confirmation tests ok\n";
