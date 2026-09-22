<?php

/**
 * Private bridge between Cypht and cypht-gateway.
 *
 * Every page requires a normal authenticated Cypht session plus the private
 * X-Cypht-Gateway-Key header. Public clients must never call these pages.
 */
if (!defined('DEBUG_MODE')) { die(); }

handler_source('gateway');

$gateway_read_pages = array(
    'ajax_gateway_ping',
    'ajax_gateway_accounts',
    'ajax_gateway_profiles',
    'ajax_gateway_mailboxes',
    'ajax_gateway_messages',
    'ajax_gateway_search',
    'ajax_gateway_message',
    'ajax_gateway_attachment'
);

$gateway_write_pages = array(
    'ajax_gateway_upload',
    'ajax_gateway_send',
    'ajax_gateway_draft',
    'ajax_gateway_message_update',
    'ajax_gateway_message_move',
    'ajax_gateway_message_archive',
    'ajax_gateway_message_delete'
);

$gateway_pages = array_merge($gateway_read_pages, $gateway_write_pages);

foreach ($gateway_pages as $page) {
    setup_base_ajax_page($page, 'core');
    add_handler($page, 'gateway_guard', true, 'gateway', 'load_user_data', 'after');
    add_handler($page, 'load_imap_servers_from_config', true, 'imap', 'gateway_guard', 'after');
}

/* Write/profile pages need Cypht's existing SMTP and profile implementations loaded. */
foreach (array('ajax_gateway_accounts', 'ajax_gateway_profiles', 'ajax_gateway_upload', 'ajax_gateway_send', 'ajax_gateway_draft') as $page) {
    add_handler($page, 'load_smtp_servers_from_config', true, 'smtp', 'load_imap_servers_from_config', 'after');
    add_handler($page, 'compose_profile_data', true, 'profiles', 'load_smtp_servers_from_config', 'after');
}

add_handler('ajax_gateway_ping', 'gateway_ping', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_accounts', 'gateway_accounts', true, 'gateway', 'compose_profile_data', 'after');
add_handler('ajax_gateway_profiles', 'gateway_profiles', true, 'gateway', 'compose_profile_data', 'after');
add_handler('ajax_gateway_mailboxes', 'gateway_mailboxes', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_messages', 'gateway_messages', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_search', 'gateway_search', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_message', 'gateway_message', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_attachment', 'gateway_attachment', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_upload', 'gateway_upload', true, 'gateway', 'compose_profile_data', 'after');
add_handler('ajax_gateway_send', 'gateway_send', true, 'gateway', 'compose_profile_data', 'after');
add_handler('ajax_gateway_draft', 'gateway_draft', true, 'gateway', 'compose_profile_data', 'after');
add_handler('ajax_gateway_message_update', 'gateway_message_update', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_message_move', 'gateway_message_move', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_message_archive', 'gateway_message_archive', true, 'gateway', 'load_imap_servers_from_config', 'after');
add_handler('ajax_gateway_message_delete', 'gateway_message_delete', true, 'gateway', 'load_imap_servers_from_config', 'after');

return array(
    'allowed_pages' => $gateway_pages,
    'allowed_get' => array(
        'account_id' => FILTER_UNSAFE_RAW,
        'account_ids' => FILTER_UNSAFE_RAW,
        'folder' => FILTER_UNSAFE_RAW,
        'uid' => FILTER_UNSAFE_RAW,
        'part' => FILTER_UNSAFE_RAW,
        'query' => FILTER_UNSAFE_RAW,
        'offset' => FILTER_VALIDATE_INT,
        'limit' => FILTER_VALIDATE_INT
    ),
    'allowed_post' => array(
        'payload' => FILTER_UNSAFE_RAW
    )
);
