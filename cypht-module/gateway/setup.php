<?php

/**
 * Private bridge between Cypht and cypht-gateway.
 *
 * This module is not a public API. Every page requires a normal authenticated
 * Cypht session plus the private X-Cypht-Gateway-Key header.
 */
if (!defined('DEBUG_MODE')) { die(); }

handler_source('gateway');

$gateway_pages = array(
    'ajax_gateway_ping',
    'ajax_gateway_accounts',
    'ajax_gateway_mailboxes',
    'ajax_gateway_messages',
    'ajax_gateway_search',
    'ajax_gateway_message'
);

foreach ($gateway_pages as $page) {
    setup_base_ajax_page($page, 'core');
    add_handler($page, 'gateway_guard', true, 'gateway', 'load_user_data', 'after');
    add_handler($page, 'load_imap_servers_from_config', true, 'imap', 'gateway_guard', 'after');
}

add_handler('ajax_gateway_ping', 'gateway_ping', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_accounts', 'gateway_accounts', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_mailboxes', 'gateway_mailboxes', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_messages', 'gateway_messages', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_search', 'gateway_search', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_message', 'gateway_message', true, 'gateway', 'load_imap_servers_from_config', 'after');

return array(
    'allowed_pages' => $gateway_pages,
    'allowed_get' => array(
        'account_id' => FILTER_UNSAFE_RAW,
        'account_ids' => FILTER_UNSAFE_RAW,
        'folder' => FILTER_UNSAFE_RAW,
        'uid' => FILTER_UNSAFE_RAW,
        'query' => FILTER_UNSAFE_RAW,
        'offset' => FILTER_VALIDATE_INT,
        'limit' => FILTER_VALIDATE_INT
    )
);
