<?php


if (!hm_exists('gateway_bridge_version')) {
function gateway_bridge_version() {
    $path = __DIR__.'/VERSION';
    if (!is_readable($path)) return 'unknown';
    return trim((string)file_get_contents($path));
}}

/** @subpackage gateway/functions */
if (!hm_exists('gateway_json_ok')) {
function gateway_json_ok($data) {
    header('Content-Type: application/json; charset=utf-8');
    header('Cache-Control: no-store');
    echo json_encode(array('ok' => true, 'data' => $data), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    Hm_Functions::cease();
}}

/** @subpackage gateway/functions */
if (!hm_exists('gateway_json_error')) {
function gateway_json_error($message, $status = 400) {
    http_response_code($status);
    header('Content-Type: application/json; charset=utf-8');
    header('Cache-Control: no-store');
    echo json_encode(array('ok' => false, 'error' => $message), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    Hm_Functions::cease();
}}

if (!hm_exists('gateway_safe_account')) {
function gateway_safe_account($id, $server) {
    $protocol = strtolower(isset($server['type']) ? $server['type'] : 'imap');
    $email = null;
    if (isset($server['user']) && filter_var($server['user'], FILTER_VALIDATE_EMAIL)) {
        $email = $server['user'];
    }
    return array(
        'id' => (string)$id,
        'name' => isset($server['name']) ? (string)$server['name'] : sprintf('%s %s', strtoupper($protocol), $id),
        'email' => $email,
        'protocol' => $protocol,
        'server' => isset($server['server']) ? (string)$server['server'] : null,
        'can_send' => false
    );
}}

if (!hm_exists('gateway_folder_role')) {
function gateway_folder_role($name, $details) {
    if (isset($details['special']) && is_string($details['special']) && $details['special']) {
        return strtolower(ltrim($details['special'], '\\'));
    }
    $lower = strtolower((string)$name);
    foreach (array('inbox', 'sent', 'drafts', 'trash', 'junk', 'archive') as $role) {
        if ($lower === $role || substr($lower, -strlen('/'.$role)) === '/'.$role) {
            return $role;
        }
    }
    return null;
}}

if (!hm_exists('gateway_normalize_message')) {
function gateway_normalize_message($account_id, $folder, $message) {
    return array(
        'uid' => isset($message['uid']) ? (string)$message['uid'] : (isset($message['id']) ? (string)$message['id'] : ''),
        'account_id' => (string)$account_id,
        'folder' => (string)$folder,
        'subject' => isset($message['subject']) ? (string)$message['subject'] : '',
        'from' => isset($message['from']) ? $message['from'] : array(),
        'to' => isset($message['to']) ? $message['to'] : array(),
        'date' => isset($message['date']) ? (string)$message['date'] : null,
        'timestamp' => isset($message['timestamp']) && is_numeric($message['timestamp']) ? (int)$message['timestamp'] : null,
        'flags' => isset($message['flags']) ? (string)$message['flags'] : '',
        'content_type' => isset($message['content-type']) ? (string)$message['content-type'] : (isset($message['content_type']) ? (string)$message['content_type'] : null),
        'preview' => isset($message['preview']) ? (string)$message['preview'] : null
    );
}}

if (!hm_exists('gateway_extract_attachments')) {
function gateway_extract_attachments($structure) {
    $decoded = json_decode(json_encode($structure), true);
    $result = array();
    gateway_walk_structure($decoded, $result);
    $unique = array();
    foreach ($result as $item) {
        $key = $item['part'].'|'.($item['filename'] ?: '').'|'.($item['content_type'] ?: '');
        $unique[$key] = $item;
    }
    return array_values($unique);
}}

if (!hm_exists('gateway_walk_structure')) {
function gateway_walk_structure($node, &$result) {
    if (!is_array($node)) {
        return;
    }
    $filename = null;
    foreach (array('filename', 'name', 'file_name') as $key) {
        if (isset($node[$key]) && is_string($node[$key]) && $node[$key] !== '') {
            $filename = $node[$key];
            break;
        }
    }
    $disposition = isset($node['disposition']) ? strtolower((string)$node['disposition']) : '';
    if ($filename || $disposition === 'attachment') {
        $type = null;
        if (isset($node['content-type'])) $type = $node['content-type'];
        elseif (isset($node['content_type'])) $type = $node['content_type'];
        elseif (isset($node['type'])) $type = $node['type'];
        $part = isset($node['part']) ? $node['part'] : (isset($node['id']) ? $node['id'] : (isset($node['part_id']) ? $node['part_id'] : ''));
        if ((string)$part !== '') {
            $result[] = array(
                'part' => (string)$part,
                'filename' => $filename,
                'content_type' => $type ? (string)$type : null,
                'size' => isset($node['size']) && is_numeric($node['size']) ? (int)$node['size'] : null,
                'inline' => $disposition === 'inline'
            );
        }
    }
    foreach ($node as $value) {
        if (is_array($value)) {
            gateway_walk_structure($value, $result);
        }
    }
}}
