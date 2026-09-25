<?php
$proxyUrl = getenv('HTTP_PROXY') ?: getenv('ALL_PROXY'); if (!$proxyUrl) { sleep(3600); exit(0); } $parsed = parse_url($proxyUrl); $proxyHost = $parsed['host'] ?? '127.0.0.1'; $proxyPort = $parsed['port'] ?? 7890;

$mappings = [
    993 => 'imap.gmail.com:993',
    465 => 'smtp.gmail.com:465',
    587 => 'smtp.gmail.com:587',
];
$servers = [];
foreach ($mappings as $port => $target) {
    $s = stream_socket_server("tcp://127.0.0.1:$port", $errno, $errstr);
    if ($s) {
        stream_set_blocking($s, false);
        $servers[(int)$s] = ['sock' => $s, 'target' => $target];
    }
}
$pairs = [];
while (true) {
    $read = array_column($servers, 'sock');
    foreach ($pairs as $p) {
        $read[] = $p['a'];
        $read[] = $p['b'];
    }
    $write = $except = null;
    if (stream_select($read, $write, $except, 1) > 0) {
        foreach ($read as $r) {
            $id = (int)$r;
            if (isset($servers[$id])) {
                $client = @stream_socket_accept($r, 0);
                if ($client) {
                    $target = $servers[$id]['target'];
                    $upstream = @stream_socket_client("tcp://$proxyHost:$proxyPort", $e, $es, 5);
                    if ($upstream) {
                        fwrite($upstream, "CONNECT $target HTTP/1.1\r\nHost: $target\r\n\r\n");
                        $hdr = '';
                        while (!str_contains($hdr, "\r\n\r\n") && !feof($upstream)) {
                            $hdr .= fread($upstream, 1024);
                        }
                        if (str_contains($hdr, "200")) {
                            stream_set_blocking($client, false);
                            stream_set_blocking($upstream, false);
                            $pairs[] = ['a' => $client, 'b' => $upstream];
                        } else {
                            fclose($client); fclose($upstream);
                        }
                    } else {
                        fclose($client);
                    }
                }
            } else {
                foreach ($pairs as $idx => $p) {
                    if ($r === $p['a'] || $r === $p['b']) {
                        $src = ($r === $p['a']) ? $p['a'] : $p['b'];
                        $dst = ($r === $p['a']) ? $p['b'] : $p['a'];
                        $data = @fread($src, 65536);
                        if ($data === '' || $data === false || feof($src)) {
                            @fclose($p['a']); @fclose($p['b']);
                            unset($pairs[$idx]);
                        } else {
                            @fwrite($dst, $data);
                        }
                        break;
                    }
                }
            }
        }
    }
}