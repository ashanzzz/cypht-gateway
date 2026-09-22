<?php

if (!hm_exists('gateway_bridge_version')) {
function gateway_bridge_version() {
    $path = __DIR__.'/VERSION';
    if (!is_readable($path)) return 'unknown';
    return trim((string)file_get_contents($path));
}}

if (!hm_exists('gateway_json_ok')) {
function gateway_json_ok($data) {
    header('Content-Type: application/json; charset=utf-8');
    header('Cache-Control: no-store');
    echo json_encode(array('ok' => true, 'data' => $data), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    Hm_Functions::cease();
}}

if (!hm_exists('gateway_json_error')) {
function gateway_json_error($message, $status = 400) {
    http_response_code($status);
    header('Content-Type: application/json; charset=utf-8');
    header('Cache-Control: no-store');
    echo json_encode(array('ok' => false, 'error' => $message), JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    Hm_Functions::cease();
}}

if (!hm_exists('gateway_payload')) {
function gateway_payload($request) {
    $raw = isset($request->post['payload']) ? (string)$request->post['payload'] : '';
    if ($raw === '') gateway_json_error('payload is required', 400);
    $payload = json_decode($raw, true);
    if (!is_array($payload)) gateway_json_error('payload must be valid JSON', 400);
    return $payload;
}}

if (!hm_exists('gateway_safe_account')) {
function gateway_safe_account($id, $server, $can_send = false) {
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
        'can_send' => (bool)$can_send
    );
}}

if (!hm_exists('gateway_folder_role')) {
function gateway_folder_role($name, $details) {
    if (isset($details['special']) && is_string($details['special']) && $details['special']) {
        return strtolower(ltrim($details['special'], '\\'));
    }
    $lower = strtolower((string)$name);
    foreach (array('inbox', 'sent', 'drafts', 'trash', 'junk', 'archive', 'scheduled') as $role) {
        if ($lower === $role || substr($lower, -strlen('/'.$role)) === '/'.$role) return $role;
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
    if (!is_array($node)) return;
    $filename = null;
    foreach (array('filename', 'name', 'file_name', 'description') as $key) {
        if (isset($node[$key]) && is_string($node[$key]) && $node[$key] !== '') {
            $filename = $node[$key];
            break;
        }
    }
    if (!$filename && isset($node['attributes']) && is_array($node['attributes'])) {
        $filename = $node['attributes']['filename'] ?? ($node['attributes']['name'] ?? null);
    }
    $disposition = isset($node['disposition']) ? strtolower((string)$node['disposition']) : '';
    if ($filename || $disposition === 'attachment' || $disposition === 'inline') {
        $type = null;
        if (isset($node['content-type'])) $type = $node['content-type'];
        elseif (isset($node['content_type'])) $type = $node['content_type'];
        elseif (isset($node['type']) && isset($node['subtype'])) $type = $node['type'].'/'.$node['subtype'];
        elseif (isset($node['type'])) $type = $node['type'];
        $part = isset($node['part']) ? $node['part'] : (isset($node['id']) ? $node['id'] : (isset($node['part_id']) ? $node['part_id'] : ''));
        if ((string)$part !== '') {
            $result[] = array(
                'part' => (string)$part,
                'filename' => $filename ? (string)$filename : null,
                'content_type' => $type ? (string)$type : null,
                'size' => isset($node['size']) && is_numeric($node['size']) ? (int)$node['size'] : null,
                'inline' => $disposition === 'inline'
            );
        }
    }
    foreach ($node as $value) if (is_array($value)) gateway_walk_structure($value, $result);
}}

if (!hm_exists('gateway_header_text')) {
function gateway_header_text($value) {
    return trim(str_replace(array("\r", "\n"), ' ', (string)$value));
}}

if (!hm_exists('gateway_address_list')) {
function gateway_address_list($value) {
    if (is_string($value)) return gateway_header_text($value);
    if (!is_array($value)) return '';
    $result = array();
    foreach ($value as $item) {
        if (is_string($item)) {
            $candidate = gateway_header_text($item);
            if ($candidate !== '') $result[] = $candidate;
            continue;
        }
        if (!is_array($item)) continue;
        $email = isset($item['email']) ? gateway_header_text($item['email']) : '';
        if (!filter_var($email, FILTER_VALIDATE_EMAIL)) continue;
        $name = isset($item['name']) ? gateway_header_text($item['name']) : '';
        $result[] = $name !== '' ? sprintf('"%s" <%s>', str_replace('"', '', $name), $email) : $email;
    }
    return implode(', ', $result);
}}

if (!hm_exists('gateway_profile_imap_id')) {
function gateway_profile_imap_id($profile) {
    if (isset($profile['imap_id']) && $profile['imap_id'] !== '') return (string)$profile['imap_id'];
    if (isset($profile['user']) && isset($profile['server'])) {
        $server = Hm_IMAP_List::fetch($profile['user'], $profile['server']);
        if (is_array($server) && isset($server['id'])) return (string)$server['id'];
    }
    return null;
}}

if (!hm_exists('gateway_profiles')) {
function gateway_profiles($handler) {
    Hm_SMTP_List::init($handler->user_config, $handler->session);
    Hm_Profiles::init($handler);
    $profiles = Hm_Profiles::getAll();
    return is_array($profiles) ? $profiles : array();
}}

if (!hm_exists('gateway_safe_profiles')) {
function gateway_safe_profiles($handler) {
    $result = array();
    foreach (gateway_profiles($handler) as $id => $profile) {
        if (!is_array($profile)) continue;
        $internal_id = isset($profile['id']) ? (string)$profile['id'] : (string)$id;
        $result[] = array(
            'id' => $internal_id,
            'name' => isset($profile['name']) ? (string)$profile['name'] : 'Profile',
            'address' => isset($profile['address']) ? (string)$profile['address'] : '',
            'reply_to' => isset($profile['replyto']) ? (string)$profile['replyto'] : '',
            'signature' => isset($profile['sig']) ? (string)$profile['sig'] : '',
            'account_id' => gateway_profile_imap_id($profile),
            'default' => !empty($profile['default'])
        );
    }
    return $result;
}}

if (!hm_exists('gateway_pick_profile')) {
function gateway_pick_profile($handler, $profile_id) {
    $profiles = gateway_profiles($handler);
    if ($profile_id !== null && $profile_id !== '') {
        foreach ($profiles as $id => $profile) {
            $candidate = isset($profile['id']) ? (string)$profile['id'] : (string)$id;
            if (hash_equals($candidate, (string)$profile_id)) { $profile['id'] = $candidate; return $profile; }
        }
        gateway_json_error('unknown profile', 404);
    }
    foreach ($profiles as $id => $profile) { if (!empty($profile['default'])) { $profile['id'] = isset($profile['id']) ? $profile['id'] : (string)$id; return $profile; } }
    $id = array_key_first($profiles);
    $first = reset($profiles);
    if (is_array($first)) { $first['id'] = isset($first['id']) ? $first['id'] : (string)$id; return $first; }
    gateway_json_error('no sending profile configured', 409);
}}

if (!hm_exists('gateway_uploads')) {
function gateway_uploads($handler) {
    $uploads = $handler->session->get('gateway_uploads', array());
    return is_array($uploads) ? $uploads : array();
}}

if (!hm_exists('gateway_resolve_uploads')) {
function gateway_resolve_uploads($handler, $ids) {
    if (!is_array($ids)) return array();
    $known = gateway_uploads($handler);
    $result = array();
    foreach ($ids as $id) {
        $id = (string)$id;
        if (!array_key_exists($id, $known) || !is_array($known[$id])) gateway_json_error('unknown or expired upload', 404);
        $filename = isset($known[$id]['filename']) ? (string)$known[$id]['filename'] : '';
        if ($filename === '' || !is_file($filename) || !is_readable($filename)) {
            unset($known[$id]);
            $handler->session->set('gateway_uploads', $known);
            gateway_json_error('unknown or expired upload', 404);
        }
        $result[] = $known[$id];
    }
    return $result;
}}

if (!hm_exists('gateway_remove_uploads')) {
function gateway_remove_uploads($handler, $ids) {
    if (!is_array($ids) || !$ids) return;
    $uploads = gateway_uploads($handler);
    foreach ($ids as $id) {
        $id = (string)$id;
        if (!isset($uploads[$id])) continue;
        if (isset($uploads[$id]['filename'])) @unlink($uploads[$id]['filename']);
        unset($uploads[$id]);
    }
    $handler->session->set('gateway_uploads', $uploads);
}}

if (!hm_exists('gateway_validate_schedule')) {
function gateway_validate_schedule($value) {
    $value = gateway_header_text($value);
    if ($value === '') return '';
    if (strlen($value) > 100) gateway_json_error('schedule_at is too long', 400);
    try {
        $date = new DateTime($value);
        $now = new DateTime('now', $date->getTimezone());
        if ($date <= $now) gateway_json_error('schedule_at must be in the future', 400);
        return $date->format(DateTime::ATOM);
    } catch (Exception $e) {
        gateway_json_error('schedule_at must be a valid date/time', 400);
    }
}}

if (!hm_exists('gateway_build_mime')) {
function gateway_build_mime($handler, $payload, $profile, $schedule = '') {
    $to = gateway_address_list($payload['to'] ?? array());
    $cc = gateway_address_list($payload['cc'] ?? array());
    $bcc = gateway_address_list($payload['bcc'] ?? array());
    $subject = gateway_header_text($payload['subject'] ?? '');
    $text = isset($payload['body']['text']) ? (string)$payload['body']['text'] : '';
    $html = isset($payload['body']['html']) ? (string)$payload['body']['html'] : '';
    $body = $html !== '' ? $html : $text;
    $body_type = $html !== '';
    $from = isset($profile['address']) && $profile['address'] !== '' ? $profile['address'] : '';
    $reply_to = isset($profile['replyto']) ? $profile['replyto'] : '';
    $from_name = isset($profile['name']) ? $profile['name'] : '';
    $in_reply_to = gateway_header_text($payload['in_reply_to'] ?? '');
    $delivery_receipt = !empty($payload['delivery_receipt']);
    list($from, $reply_to) = outbound_address_check($handler, $from, $reply_to);
    $profile_id = isset($profile['id']) ? (string)$profile['id'] : '';
    $mime = new Hm_MIME_Msg($to, $subject, $body, $from, $body_type, $cc, $bcc, $in_reply_to, $from_name, $reply_to, $delivery_receipt, $schedule, $profile_id);
    $upload_ids = isset($payload['attachment_ids']) && is_array($payload['attachment_ids']) ? $payload['attachment_ids'] : array();
    $mime->add_attachments(gateway_resolve_uploads($handler, $upload_ids));
    return array($mime, $from, $upload_ids);
}}

if (!hm_exists('gateway_send_now')) {
function gateway_send_now($handler, $payload, $profile) {
    $smtp_id = isset($profile['smtp_id']) ? (string)$profile['smtp_id'] : '';
    if ($smtp_id === '') gateway_json_error('profile has no SMTP server', 409);
    $smtp_details = Hm_SMTP_List::dump($smtp_id, true);
    if (!$smtp_details) gateway_json_error('SMTP server is unavailable', 409);
    smtp_refresh_oauth2_token_on_send($smtp_details, $handler, $smtp_id);
    $smtp = Hm_SMTP_List::connect($smtp_id, false);
    if (!$smtp || !$smtp->authed()) gateway_json_error('SMTP authentication failed', 502);

    list($mime, $from, $upload_ids) = gateway_build_mime($handler, $payload, $profile, '');
    $recipients = $mime->get_recipient_addresses();
    if (!$recipients) gateway_json_error('no valid recipients found', 400);
    $message = $mime->get_mime_msg();
    $error = $smtp->send_message($from, $recipients, $message, !empty($payload['delivery_receipt']));
    if ($error) gateway_json_error((string)$error, 502);

    $imap_id = gateway_profile_imap_id($profile);
    $saved_uid = null;
    $saved_folder = null;
    if ($imap_id !== null) {
        $imap_details = Hm_IMAP_List::dump($imap_id);
        $imap = Hm_IMAP_List::get_connected_mailbox($imap_id, $handler->cache);
        if ($imap && $imap->authed() && $imap_details) {
            $mime->set_original_bcc_header();
            list($saved_uid, $saved_folder) = save_sent_msg($handler, $imap_id, $imap, $imap_details, $mime->get_mime_msg(), $mime->get_headers()['Message-Id'], false);
        }
    }
    gateway_remove_uploads($handler, $upload_ids);
    return array(
        'status' => 'sent',
        'message_id_header' => $mime->get_headers()['Message-Id'] ?? null,
        'account_id' => $imap_id,
        'folder' => $saved_folder,
        'uid' => $saved_uid ? (string)$saved_uid : null,
        'scheduled' => false
    );
}}

if (!hm_exists('gateway_store_draft')) {
function gateway_store_draft($handler, $payload, $profile, $schedule = '') {
    if ($schedule !== '' && !$handler->module_is_supported('scheduled_sends')) {
        gateway_json_error('scheduled_sends module is not enabled in Cypht', 409);
    }
    $imap_id = gateway_profile_imap_id($profile);
    if ($imap_id === null) gateway_json_error('profile has no associated mailbox', 409);
    $imap = Hm_IMAP_List::get_connected_mailbox($imap_id, $handler->cache);
    if (!$imap || !$imap->authed()) gateway_json_error('mailbox authentication failed', 502);
    list($mime, , $upload_ids) = gateway_build_mime($handler, $payload, $profile, $schedule);
    $folder = $schedule !== '' ? 'Scheduled' : null;
    if ($folder === null) {
        $special = get_special_folders($handler, $imap_id);
        $folder = isset($special['draft']) && $special['draft'] ? $special['draft'] : null;
        if (!$folder) {
            $auto = $imap->get_special_use_mailboxes('drafts');
            $folder = is_array($auto) && isset($auto['drafts']) ? $auto['drafts'] : 'Drafts';
        }
    }
    if (!$imap->folder_exists($folder) && !$imap->create_folder($folder)) gateway_json_error('draft/scheduled folder is unavailable', 409);
    $uid = $imap->store_message($folder, $mime->get_mime_msg(), false, true);
    if (!$uid) gateway_json_error('failed to store draft', 502);
    gateway_remove_uploads($handler, $upload_ids);
    return array(
        'status' => $schedule !== '' ? 'scheduled' : 'draft',
        'message_id_header' => $mime->get_headers()['Message-Id'] ?? null,
        'account_id' => $imap_id,
        'folder' => $folder,
        'uid' => (string)$uid,
        'scheduled' => $schedule !== ''
    );
}}
