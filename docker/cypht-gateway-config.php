<?php

$mods = explode(',', env('CYPHT_MODULES', 'core,contacts,local_contacts,feeds,imap,smtp,account,idle_timer,calendar,themes,nux,developer,history,saved_searches,advanced_search,highlights,profiles,inline_message,imap_folders,keyboard_shortcuts,tags,brute_force'));
foreach (array('sievefilters', 'api_login', 'gateway') as $req) {
    if (!in_array($req, $mods, true)) {
        $mods[] = $req;
    }
}

return array(
    'modules' => $mods,
    'api_login_key' => env('API_LOGIN_KEY'),
);