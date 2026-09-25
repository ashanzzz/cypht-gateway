<?php

// Keep the API login secret in the container environment, not in the image.
return array(
    'api_login_key' => env('API_LOGIN_KEY'),
);
