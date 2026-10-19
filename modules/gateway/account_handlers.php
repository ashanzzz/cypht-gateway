<?php

/**
 * Cohesive account bundle mutations for the private Cypht Gateway bridge.
 *
 * Invariants:
 * - candidate connectivity tests never persist configuration;
 * - IMAP + SMTP + Profile mutations share one fresh user-config snapshot;
 * - successful writes perform exactly one durable commit and fresh readback;
 * - deletion fails closed when any bundle resource is shared;
 * - secrets are accepted only on the private Bridge boundary and are never returned.
 */
if (!defined('DEBUG_MODE')) { die(); }

if (!defined('GATEWAY_ACCOUNT_WRITE_SECTIONS')) {
    define('GATEWAY_ACCOUNT_WRITE_SECTIONS', array('imap_servers', 'profiles', 'smtp_servers'));
}

if (!hm_exists('gateway_account_internal_id')) {
function gateway_account_internal_id($prefix) {
    return $prefix.'_'.bin2hex(random_bytes(16));
}}

if (!hm_exists('gateway_account_scalar_text')) {
function gateway_account_scalar_text($value, $field, $max = 512, $allow_empty = false) {
    if (!is_string($value) && !is_int($value)) gateway_json_error($field.' must be text', 400);
    $value = trim((string)$value);
    if ((!$allow_empty && $value === '') || strlen($value) > $max || strpos($value, "\0") !== false) {
        gateway_json_error('invalid '.$field, 400);
    }
    return $value;
}}

if (!hm_exists('gateway_account_endpoint')) {
function gateway_account_endpoint($value, $protocol, $require_secret = true) {
    if (!is_array($value)) gateway_json_error(strtoupper($protocol).' endpoint is required', 400);
    $host = gateway_account_scalar_text($value['host'] ?? '', $protocol.' host', 253);
    if (preg_match('/[\x00-\x20\\\/]/', $host)) gateway_json_error('invalid '.$protocol.' host', 400);
    $username = gateway_account_scalar_text($value['username'] ?? '', $protocol.' username', 512);
    $port = filter_var($value['port'] ?? null, FILTER_VALIDATE_INT, array(
        'options' => array('min_range' => 1, 'max_range' => 65535)
    ));
    if ($port === false) gateway_json_error('invalid '.$protocol.' port', 400);
    $security = gateway_account_scalar_text($value['security'] ?? '', $protocol.' security', 32);
    if (!in_array($security, array('tls', 'start_tls'), true)) {
        // Cypht 2.12/2.13 automatically attempts STARTTLS on non-TLS sockets.
        // Until upstream exposes an explicit "never STARTTLS" primitive, plain
        // "none" would be a false contract and is intentionally rejected.
        gateway_json_error('unsupported '.$protocol.' security mode', 400);
    }
    $password = '';
    if ($require_secret) {
        $password = (string)($value['password'] ?? '');
        if ($password === '' || strlen($password) > 4096 || strpos($password, "\0") !== false) {
            gateway_json_error('invalid '.$protocol.' credential', 400);
        }
    }
    return array(
        'protocol' => $protocol,
        'host' => $host,
        'port' => (int)$port,
        'security' => $security,
        'username' => $username,
        'password' => $password
    );
}}

if (!hm_exists('gateway_account_identity')) {
function gateway_account_identity($payload, $fallback_email) {
    $value = isset($payload['identity']) && is_array($payload['identity']) ? $payload['identity'] : array();
    $email = gateway_account_scalar_text($value['email'] ?? ($payload['email'] ?? $fallback_email), 'identity email', 320);
    if (!filter_var($email, FILTER_VALIDATE_EMAIL)) gateway_json_error('invalid identity email', 400);
    $name = gateway_account_scalar_text($value['name'] ?? ($payload['name'] ?? $email), 'identity name', 200);
    $reply_to = gateway_account_scalar_text($value['reply_to'] ?? '', 'identity reply_to', 320, true);
    if ($reply_to !== '' && !filter_var($reply_to, FILTER_VALIDATE_EMAIL)) gateway_json_error('invalid identity reply_to', 400);
    $signature = gateway_account_scalar_text($value['signature'] ?? '', 'identity signature', 16384, true);
    return array('name' => $name, 'email' => $email, 'reply_to' => $reply_to, 'signature' => $signature);
}}

if (!hm_exists('gateway_account_cypht_tls')) {
function gateway_account_cypht_tls($security) {
    return $security === 'tls' ? true : 'starttls';
}}

if (!hm_exists('gateway_account_close_mailbox')) {
function gateway_account_close_mailbox($mailbox) {
    try {
        if (!is_object($mailbox) || !method_exists($mailbox, 'get_connection')) return;
        $connection = $mailbox->get_connection();
        if (is_object($connection) && method_exists($connection, 'disconnect')) $connection->disconnect();
    } catch (Throwable $error) {
        // Connectivity tests are best effort on close; never leak provider errors.
    }
}}

if (!hm_exists('gateway_account_test_endpoint')) {
function gateway_account_test_endpoint($handler, $endpoint) {
    $id = 'gateway-test-'.$endpoint['protocol'].'-'.bin2hex(random_bytes(8));
    $config = array(
        'id' => $id,
        'server' => $endpoint['host'],
        'port' => $endpoint['port'],
        'tls' => gateway_account_cypht_tls($endpoint['security']),
        'type' => $endpoint['protocol'],
        'username' => $endpoint['username'],
        'password' => $endpoint['password']
    );
    $mailbox = new Hm_Mailbox($id, $handler->user_config, $handler->session, $config);
    try {
        $mailbox->connect();
        return $mailbox->authed() === true;
    } catch (Throwable $error) {
        return false;
    } finally {
        gateway_account_close_mailbox($mailbox);
    }
}}

if (!hm_exists('gateway_account_test_candidate')) {
function gateway_account_test_candidate($handler, $imap, $smtp) {
    $imap_ok = gateway_account_test_endpoint($handler, $imap);
    $smtp_ok = $smtp === null ? null : gateway_account_test_endpoint($handler, $smtp);
    return array(
        'status' => $imap_ok && ($smtp_ok === null || $smtp_ok) ? 'ok' : 'failed',
        'inbound' => array('reachable' => $imap_ok),
        'outbound' => $smtp === null ? null : array('reachable' => $smtp_ok)
    );
}}

if (!hm_exists('gateway_account_begin_bundle_write')) {
function gateway_account_begin_bundle_write($handler) {
    return gateway_require_durable_user_config($handler, GATEWAY_ACCOUNT_WRITE_SECTIONS);
}}

if (!hm_exists('gateway_account_stage_bundle')) {
function gateway_account_stage_bundle($handler, $imap_servers, $smtp_servers, $profiles) {
    foreach (array(
        'imap_servers' => $imap_servers,
        'smtp_servers' => $smtp_servers,
        'profiles' => $profiles
    ) as $section => $value) {
        if (!method_exists($handler->user_config, 'gateway_set_write_section') ||
            !$handler->user_config->gateway_set_write_section($section, $value)) {
            throw new RuntimeException('failed to stage account configuration');
        }
    }
}}

if (!hm_exists('gateway_account_commit_bundle')) {
function gateway_account_commit_bundle($handler, $snapshot) {
    return gateway_commit_durable_user_config($handler, GATEWAY_ACCOUNT_WRITE_SECTIONS, $snapshot);
}}

if (!hm_exists('gateway_account_safe_bundle_result')) {
function gateway_account_safe_bundle_result($imap_id, $smtp_id, $profile_ids, $readback) {
    $imap = $readback['imap_servers'][$imap_id] ?? null;
    $smtp = $smtp_id ? ($readback['smtp_servers'][$smtp_id] ?? null) : null;
    $profiles = array();
    foreach ($profile_ids as $profile_id) {
        if (isset($readback['profiles'][$profile_id])) {
            $profile = $readback['profiles'][$profile_id];
            unset($profile['sig']);
            $profiles[] = $profile;
        }
    }
    if (is_array($imap)) unset($imap['pass'], $imap['object'], $imap['connected']);
    if (is_array($smtp)) unset($smtp['pass'], $smtp['object'], $smtp['connected']);
    return array(
        'imap_id' => $imap_id,
        'smtp_id' => $smtp_id,
        'profile_ids' => array_values($profile_ids),
        'imap' => $imap,
        'smtp' => $smtp,
        'profiles' => $profiles
    );
}}

if (!hm_exists('gateway_account_profile_dependencies')) {
function gateway_account_profile_dependencies($profiles, $imap_id, $smtp_id, $owned_profile_ids) {
    $owned = array_fill_keys(array_map('strval', $owned_profile_ids), true);
    $dependencies = array();
    foreach ((array)$profiles as $id => $profile) {
        if (isset($owned[(string)$id]) || !is_array($profile)) continue;
        if (($profile['imap_id'] ?? null) === $imap_id) {
            $dependencies[] = array('type' => 'profile', 'id' => (string)$id, 'resource' => 'imap');
        }
        if ($smtp_id !== null && ($profile['smtp_id'] ?? null) === $smtp_id) {
            $dependencies[] = array('type' => 'profile', 'id' => (string)$id, 'resource' => 'smtp');
        }
    }
    return $dependencies;
}}

if (!hm_exists('gateway_account_validate_bundle_ownership')) {
function gateway_account_validate_bundle_ownership($profiles, $imap_id, $smtp_id, $profile_ids) {
    foreach ($profile_ids as $profile_id) {
        if (!isset($profiles[$profile_id]) || !is_array($profiles[$profile_id])) {
            gateway_json_error('account bundle profile not found', 409);
        }
        $profile = $profiles[$profile_id];
        if (($profile['imap_id'] ?? null) !== $imap_id ||
            ($smtp_id !== null && ($profile['smtp_id'] ?? null) !== $smtp_id)) {
            gateway_json_error('account bundle ownership mismatch', 409);
        }
    }
}}

if (!hm_exists('gateway_account_ensure_default_profile')) {
function gateway_account_ensure_default_profile(&$profiles) {
    if (!$profiles) return;
    $defaults = array();
    foreach ($profiles as $id => $profile) {
        if (is_array($profile) && !empty($profile['default'])) $defaults[] = $id;
    }
    if (count($defaults) === 1) return;
    foreach ($profiles as $id => &$profile) {
        if (is_array($profile)) $profile['default'] = false;
    }
    unset($profile);
    $first = array_key_first($profiles);
    if ($first !== null && is_array($profiles[$first])) $profiles[$first]['default'] = true;
}}

class Hm_Handler_gateway_account_bundle_test extends Hm_Handler_Module {
    public function process() {
        $payload = gateway_payload($this->request);
        $imap = gateway_account_endpoint($payload['inbound'] ?? null, 'imap');
        $smtp = isset($payload['outbound']) && $payload['outbound'] !== null
            ? gateway_account_endpoint($payload['outbound'], 'smtp') : null;
        gateway_json_ok(gateway_account_test_candidate($this, $imap, $smtp));
    }
}

class Hm_Handler_gateway_account_bundle_create extends Hm_Handler_Module {
    public function process() {
        $payload = gateway_payload($this->request);
        $name = gateway_account_scalar_text($payload['name'] ?? '', 'account name', 200);
        $imap = gateway_account_endpoint($payload['inbound'] ?? null, 'imap');
        $smtp = isset($payload['outbound']) && $payload['outbound'] !== null
            ? gateway_account_endpoint($payload['outbound'], 'smtp') : null;
        $identity = gateway_account_identity($payload, $imap['username']);
        if (!empty($payload['test_before_save'])) {
            $test = gateway_account_test_candidate($this, $imap, $smtp);
            if (($test['status'] ?? '') !== 'ok') gateway_json_error('account connectivity test failed', 422);
        }

        $imap_id = gateway_account_internal_id('imap');
        $smtp_id = $smtp === null ? null : gateway_account_internal_id('smtp');
        $profile_id = $smtp === null ? null : gateway_account_internal_id('prf');
        $profile_ids = $profile_id === null ? array() : array($profile_id);
        $snapshot = gateway_account_begin_bundle_write($this);
        try {
            $imap_servers = $this->user_config->get('imap_servers', array());
            $smtp_servers = $this->user_config->get('smtp_servers', array());
            $profiles = $this->user_config->get('profiles', array());
            if (!is_array($imap_servers) || !is_array($smtp_servers) || !is_array($profiles)) {
                throw new RuntimeException('invalid account repositories');
            }
            $imap_servers[$imap_id] = array(
                'id' => $imap_id,
                'name' => $name,
                'server' => $imap['host'],
                'port' => $imap['port'],
                'tls' => gateway_account_cypht_tls($imap['security']),
                'type' => 'imap',
                'user' => $imap['username'],
                'pass' => $imap['password']
            );
            if ($smtp !== null) {
                $smtp_servers[$smtp_id] = array(
                    'id' => $smtp_id,
                    'name' => $name,
                    'server' => $smtp['host'],
                    'port' => $smtp['port'],
                    'tls' => gateway_account_cypht_tls($smtp['security']),
                    'type' => 'smtp',
                    'user' => $smtp['username'],
                    'pass' => $smtp['password']
                );
                $profiles[$profile_id] = array(
                    'id' => $profile_id,
                    'default' => count($profiles) === 0,
                    'name' => $identity['name'],
                    'address' => $identity['email'],
                    'replyto' => $identity['reply_to'],
                    'smtp_id' => $smtp_id,
                    'imap_id' => $imap_id,
                    'sig' => $identity['signature'],
                    'rmk' => '',
                    'type' => 'imap',
                    'user' => $imap['username'],
                    'server' => $imap['host']
                );
            }
            gateway_account_stage_bundle($this, $imap_servers, $smtp_servers, $profiles);
            $readback = gateway_account_commit_bundle($this, $snapshot);
            if (!is_array($readback) || !isset($readback['imap_servers'][$imap_id]) ||
                ($smtp_id !== null && !isset($readback['smtp_servers'][$smtp_id])) ||
                ($profile_id !== null && !isset($readback['profiles'][$profile_id]))) {
                throw new RuntimeException('account bundle readback verification failed');
            }
            gateway_json_ok(gateway_account_safe_bundle_result($imap_id, $smtp_id, $profile_ids, $readback));
        } catch (Throwable $error) {
            Hm_Gateway_Response::abort_user_config_write();
            if (class_exists('Gateway_User_Config_Conflict', false) && $error instanceof Gateway_User_Config_Conflict) {
                gateway_json_error('user-config write conflict', 409);
            }
            gateway_json_error('account bundle create failed', 502);
        }
    }
}

class Hm_Handler_gateway_account_bundle_update extends Hm_Handler_Module {
    public function process() {
        $payload = gateway_payload($this->request);
        $imap_id = gateway_account_scalar_text($payload['imap_id'] ?? '', 'imap_id', 128);
        $smtp_id = isset($payload['smtp_id']) && $payload['smtp_id'] !== null
            ? gateway_account_scalar_text($payload['smtp_id'], 'smtp_id', 128) : null;
        $profile_ids = isset($payload['profile_ids']) && is_array($payload['profile_ids'])
            ? array_values(array_map('strval', $payload['profile_ids'])) : array();
        $snapshot = gateway_account_begin_bundle_write($this);
        try {
            $imap_servers = $this->user_config->get('imap_servers', array());
            $smtp_servers = $this->user_config->get('smtp_servers', array());
            $profiles = $this->user_config->get('profiles', array());
            if (!isset($imap_servers[$imap_id])) gateway_json_error('account bundle not found', 404);
            if ($smtp_id !== null && !isset($smtp_servers[$smtp_id])) gateway_json_error('account bundle not found', 404);
            gateway_account_validate_bundle_ownership($profiles, $imap_id, $smtp_id, $profile_ids);

            if (isset($payload['inbound'])) {
                $imap = gateway_account_endpoint($payload['inbound'], 'imap');
                $existing = $imap_servers[$imap_id];
                $existing['server'] = $imap['host'];
                $existing['port'] = $imap['port'];
                $existing['tls'] = gateway_account_cypht_tls($imap['security']);
                $existing['user'] = $imap['username'];
                $existing['pass'] = $imap['password'];
                $imap_servers[$imap_id] = $existing;
            }
            if ($smtp_id !== null && isset($payload['outbound'])) {
                $smtp = gateway_account_endpoint($payload['outbound'], 'smtp');
                $existing = $smtp_servers[$smtp_id];
                $existing['server'] = $smtp['host'];
                $existing['port'] = $smtp['port'];
                $existing['tls'] = gateway_account_cypht_tls($smtp['security']);
                $existing['user'] = $smtp['username'];
                $existing['pass'] = $smtp['password'];
                $smtp_servers[$smtp_id] = $existing;
            }
            if (isset($payload['identity']) && $profile_ids) {
                $fallback = $imap_servers[$imap_id]['user'] ?? '';
                $identity = gateway_account_identity($payload, $fallback);
                foreach ($profile_ids as $profile_id) {
                    $profiles[$profile_id]['name'] = $identity['name'];
                    $profiles[$profile_id]['address'] = $identity['email'];
                    $profiles[$profile_id]['replyto'] = $identity['reply_to'];
                    $profiles[$profile_id]['sig'] = $identity['signature'];
                }
            }
            gateway_account_stage_bundle($this, $imap_servers, $smtp_servers, $profiles);
            $readback = gateway_account_commit_bundle($this, $snapshot);
            gateway_json_ok(gateway_account_safe_bundle_result($imap_id, $smtp_id, $profile_ids, $readback));
        } catch (Throwable $error) {
            Hm_Gateway_Response::abort_user_config_write();
            if (class_exists('Gateway_User_Config_Conflict', false) && $error instanceof Gateway_User_Config_Conflict) {
                gateway_json_error('user-config write conflict', 409);
            }
            gateway_json_error('account bundle update failed', 502);
        }
    }
}

class Hm_Handler_gateway_account_bundle_delete extends Hm_Handler_Module {
    public function process() {
        $payload = gateway_payload($this->request);
        if (($payload['confirm'] ?? false) !== true) gateway_json_error('delete confirmation is required', 400);
        $imap_id = gateway_account_scalar_text($payload['imap_id'] ?? '', 'imap_id', 128);
        $smtp_id = isset($payload['smtp_id']) && $payload['smtp_id'] !== null
            ? gateway_account_scalar_text($payload['smtp_id'], 'smtp_id', 128) : null;
        $profile_ids = isset($payload['profile_ids']) && is_array($payload['profile_ids'])
            ? array_values(array_unique(array_map('strval', $payload['profile_ids']))) : array();
        $snapshot = gateway_account_begin_bundle_write($this);
        try {
            $imap_servers = $this->user_config->get('imap_servers', array());
            $smtp_servers = $this->user_config->get('smtp_servers', array());
            $profiles = $this->user_config->get('profiles', array());
            if (!isset($imap_servers[$imap_id]) || ($smtp_id !== null && !isset($smtp_servers[$smtp_id]))) {
                gateway_json_error('account bundle not found', 404);
            }
            gateway_account_validate_bundle_ownership($profiles, $imap_id, $smtp_id, $profile_ids);
            $dependencies = gateway_account_profile_dependencies($profiles, $imap_id, $smtp_id, $profile_ids);
            if ($dependencies) gateway_json_error('account_has_dependencies', 409);

            foreach ($profile_ids as $profile_id) unset($profiles[$profile_id]);
            if ($smtp_id !== null) unset($smtp_servers[$smtp_id]);
            unset($imap_servers[$imap_id]);
            gateway_account_ensure_default_profile($profiles);
            gateway_account_stage_bundle($this, $imap_servers, $smtp_servers, $profiles);
            $readback = gateway_account_commit_bundle($this, $snapshot);
            if (isset($readback['imap_servers'][$imap_id]) ||
                ($smtp_id !== null && isset($readback['smtp_servers'][$smtp_id]))) {
                throw new RuntimeException('account bundle delete readback failed');
            }
            foreach ($profile_ids as $profile_id) {
                if (isset($readback['profiles'][$profile_id])) throw new RuntimeException('account bundle delete readback failed');
            }
            gateway_json_ok(array(
                'status' => 'deleted',
                'imap_id' => $imap_id,
                'smtp_id' => $smtp_id,
                'profile_ids' => $profile_ids
            ));
        } catch (Throwable $error) {
            Hm_Gateway_Response::abort_user_config_write();
            if (class_exists('Gateway_User_Config_Conflict', false) && $error instanceof Gateway_User_Config_Conflict) {
                gateway_json_error('user-config write conflict', 409);
            }
            gateway_json_error('account bundle delete failed', 502);
        }
    }
}
