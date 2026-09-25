<?php

$source = getenv('CYPHT_SOURCE_PATH');
if (!$source || !is_file($source.'/modules/saved_searches/modules.php')) {
    fwrite(STDERR, "Set CYPHT_SOURCE_PATH to Cypht 2.12.0 source\n");
    exit(1);
}
require $source.'/lib/version.php';
if (CYPHT_VERSION !== '2.12.0') throw new RuntimeException('Saved Searches test requires Cypht 2.12.0');
define('DEBUG_MODE', true);
function hm_exists($name) { return function_exists($name); }
class Hm_Handler_Module {}
class Hm_Output_Module {}
require $source.'/modules/saved_searches/modules.php';

$repo = new Hm_Saved_Searches(array());
$simple = array('hello', 'any', 'subject', 'Work');
$advanced = array(
    'terms' => array(array('term' => 'hello', 'condition' => false)),
    'targets' => array(array('target' => 'TEXT', 'orig' => 'TEXT', 'condition' => false)),
    'sources' => array(array('source' => 'imap_7_494e424f58', 'label' => 'Mail > INBOX', 'subFolders' => false)),
    'times' => array(array('from' => '2026-01-01', 'to' => '2026-12-31')),
    'other' => array('limit' => '100', 'flags' => array(), 'charset' => '')
);
if (!$repo->add('Work', $simple) || !$repo->add_advanced('Complex', $advanced)) {
    throw new RuntimeException('Cypht search creation failed');
}
if ($repo->get('Work') !== $simple || $repo->get_advanced('Complex') !== $advanced || !$repo->is_advanced('Complex')) {
    throw new RuntimeException('Cypht search formats were not preserved');
}
if (!$repo->rename('Work', 'Renamed') || $repo->get('Work') !== false || $repo->get('Renamed') !== $simple) {
    throw new RuntimeException('Cypht rename semantics changed');
}
if (!$repo->rename('Renamed', 'Complex') || $repo->get('Complex') !== $simple) {
    throw new RuntimeException('Cypht overwrite-on-rename behavior changed');
}
if (!$repo->delete('Complex') || $repo->get('Complex') !== false) {
    throw new RuntimeException('Cypht search deletion failed');
}
echo "Cypht 2.12.0 Saved Searches repository behavior confirmed\n";