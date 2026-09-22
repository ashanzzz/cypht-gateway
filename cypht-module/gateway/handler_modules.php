<?php

/**
 * All bridge handlers deliberately return JSON and stop dispatch immediately.
 * No secrets from Hm_IMAP_List::dump(..., true) are ever exposed.
 */
class Hm_Handler_gateway_guard extends Hm_Handler_Module {
    public function process() {
        $configured = env('GATEWAY_BRIDGE_KEY', '');
        $provided = isset($this->request->server['HTTP_X_CYPHT_GATEWAY_KEY'])
            ? $this->request->server['HTTP_X_CYPHT_GATEWAY_KEY'] : '';
        if (!$configured || !$provided || !hash_equals((string)$configured, (string)$provided)) {
            gateway_json_error('bridge authentication failed', 403);
        }
    }
}

class Hm_Handler_gateway_ping extends Hm_Handler_Module {
    public function process() {
        gateway_json_ok(array('status' => 'ok', 'bridge_version' => gateway_bridge_version()));
    }
}

class Hm_Handler_gateway_accounts extends Hm_Handler_Module {
    public function process() {
        $accounts = array();
        foreach (Hm_IMAP_List::dump() as $id => $server) {
            $accounts[] = gateway_safe_account($id, $server);
        }
        gateway_json_ok($accounts);
    }
}

class Hm_Handler_gateway_mailboxes extends Hm_Handler_Module {
    public function process() {
        $id = isset($this->request->get['account_id']) ? (string)$this->request->get['account_id'] : '';
        if ($id === '' || !Hm_IMAP_List::dump($id)) {
            gateway_json_error('unknown account', 404);
        }
        $mailbox = Hm_IMAP_List::get_connected_mailbox($id, $this->cache);
        if (!$mailbox || !$mailbox->authed()) {
            gateway_json_error('mail account authentication failed', 502);
        }
        $folders = $mailbox->get_folders(false);
        if (!is_array($folders)) {
            gateway_json_ok(array());
        }
        $result = array();
        foreach ($folders as $name => $details) {
            if (!is_array($details)) $details = array();
            $folder_name = is_string($name) ? $name : (isset($details['name']) ? $details['name'] : '');
            if ($folder_name === '') continue;
            $status = $mailbox->get_folder_status($folder_name, false);
            $result[] = array(
                'name' => (string)$folder_name,
                'display_name' => isset($details['name']) ? (string)$details['name'] : (string)$folder_name,
                'role' => gateway_folder_role($folder_name, $details),
                'total' => is_array($status) && isset($status['messages']) ? (int)$status['messages'] : null,
                'unread' => is_array($status) && isset($status['unseen']) ? (int)$status['unseen'] : null,
                'selectable' => !(isset($details['noselect']) && $details['noselect'])
            );
        }
        gateway_json_ok($result);
    }
}

class Hm_Handler_gateway_messages extends Hm_Handler_Module {
    public function process() {
        $id = isset($this->request->get['account_id']) ? (string)$this->request->get['account_id'] : '';
        $folder = isset($this->request->get['folder']) ? (string)$this->request->get['folder'] : 'INBOX';
        $offset = isset($this->request->get['offset']) ? max(0, (int)$this->request->get['offset']) : 0;
        $limit = isset($this->request->get['limit']) ? min(1100, max(1, (int)$this->request->get['limit'])) : 50;
        if ($id === '' || !Hm_IMAP_List::dump($id)) gateway_json_error('unknown account', 404);
        $mailbox = Hm_IMAP_List::get_connected_mailbox($id, $this->cache);
        if (!$mailbox || !$mailbox->authed()) gateway_json_error('mail account authentication failed', 502);
        list($total, $messages) = $mailbox->get_messages($folder, 'arrival', true, 'ALL', $offset, $limit, false, array(), true);
        $result = array();
        foreach ((array)$messages as $message) {
            $result[] = gateway_normalize_message($id, $folder, $message);
        }
        gateway_json_ok(array('total' => is_numeric($total) ? (int)$total : null, 'messages' => $result));
    }
}

class Hm_Handler_gateway_search extends Hm_Handler_Module {
    public function process() {
        $ids_raw = isset($this->request->get['account_ids']) ? (string)$this->request->get['account_ids'] : '';
        $ids = array_values(array_filter(explode(',', $ids_raw), 'strlen'));
        $folder = isset($this->request->get['folder']) ? (string)$this->request->get['folder'] : 'INBOX';
        $query = isset($this->request->get['query']) ? trim((string)$this->request->get['query']) : '';
        $limit = isset($this->request->get['limit']) ? min(100, max(1, (int)$this->request->get['limit'])) : 50;
        if ($query === '') gateway_json_error('query cannot be empty', 400);
        if (!$ids) $ids = array_keys(Hm_IMAP_List::dump());
        $known = Hm_IMAP_List::dump();
        $result = array();
        foreach ($ids as $id) {
            if (!array_key_exists($id, $known)) continue;
            $mailbox = Hm_IMAP_List::get_connected_mailbox($id, $this->cache);
            if (!$mailbox || !$mailbox->authed()) continue;
            list(, $messages) = $mailbox->get_messages($folder, 'arrival', true, 'ALL', 0, $limit, $query, array(), true);
            foreach ((array)$messages as $message) {
                $result[] = gateway_normalize_message($id, $folder, $message);
            }
        }
        usort($result, function($a, $b) { return (int)($b['timestamp'] ?: 0) <=> (int)($a['timestamp'] ?: 0); });
        if (count($result) > $limit) $result = array_slice($result, 0, $limit);
        gateway_json_ok(array('total' => count($result), 'messages' => $result));
    }
}

class Hm_Handler_gateway_message extends Hm_Handler_Module {
    public function process() {
        $id = isset($this->request->get['account_id']) ? (string)$this->request->get['account_id'] : '';
        $folder = isset($this->request->get['folder']) ? (string)$this->request->get['folder'] : '';
        $uid = isset($this->request->get['uid']) ? (string)$this->request->get['uid'] : '';
        if ($id === '' || $folder === '' || $uid === '') gateway_json_error('account_id, folder and uid are required', 400);
        if (!Hm_IMAP_List::dump($id)) gateway_json_error('unknown account', 404);
        $mailbox = Hm_IMAP_List::get_connected_mailbox($id, $this->cache);
        if (!$mailbox || !$mailbox->authed()) gateway_json_error('mail account authentication failed', 502);
        list($structure, , $text) = $mailbox->get_structured_message($folder, $uid, false, true, true);
        $headers = $mailbox->get_message_headers($folder, $uid);
        gateway_json_ok(array(
            'uid' => $uid,
            'account_id' => $id,
            'folder' => $folder,
            'headers' => is_array($headers) ? $headers : array(),
            'body_text' => is_string($text) ? $text : null,
            'body_html' => null,
            'structure' => $structure,
            'attachments' => gateway_extract_attachments($structure)
        ));
    }
}
