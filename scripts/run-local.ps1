param(
    [string]$EnvFile = "deploy/local-dev.env",
    [string]$Bind = "127.0.0.1:18080",
    [switch]$HealthOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$envPath = Join-Path $repositoryRoot $EnvFile
if (-not (Test-Path -LiteralPath $envPath -PathType Leaf)) {
    throw "Missing $EnvFile. Copy deploy/local-dev.env.example and fill its secret values."
}

foreach ($line in Get-Content -LiteralPath $envPath) {
    $trimmed = $line.Trim()
    if (-not $trimmed -or $trimmed.StartsWith("#")) { continue }
    if ($trimmed -notmatch '^([A-Z][A-Z0-9_]*)=(.*)$') { continue }

    $name = $Matches[1]
    $value = $Matches[2].Trim()
    if ($value.Length -ge 2) {
        $quote = $value[0]
        if (($quote -eq "'" -or $quote -eq '"') -and $value[$value.Length - 1] -eq $quote) {
            $value = $value.Substring(1, $value.Length - 2)
        }
    }
    [Environment]::SetEnvironmentVariable($name, $value, "Process")
}

$required = @("GATEWAY_MASTER_KEY", "CYPHT_BASE_URL", "CYPHT_API_LOGIN_KEY", "CYPHT_BRIDGE_KEY")
$missing = @($required | Where-Object { -not [Environment]::GetEnvironmentVariable($_, "Process") })
if ($HealthOnly) {
    if (-not $env:CYPHT_BASE_URL) {
        $env:CYPHT_BASE_URL = "http://192.168.8.11:8088/"
    }
    foreach ($name in @("GATEWAY_MASTER_KEY", "CYPHT_API_LOGIN_KEY", "CYPHT_BRIDGE_KEY")) {
        if (-not [Environment]::GetEnvironmentVariable($name, "Process")) {
            $bytes = [byte[]]::new(32)
            $generator = [System.Security.Cryptography.RandomNumberGenerator]::Create()
            try {
                $generator.GetBytes($bytes)
                [Environment]::SetEnvironmentVariable($name, [Convert]::ToBase64String($bytes), "Process")
            } finally {
                [Array]::Clear($bytes, 0, $bytes.Length)
                $generator.Dispose()
            }
        }
    }
    Write-Warning "Health-only mode uses temporary process keys. Cypht login will fail until the matching Bridge and secrets are installed."
} elseif ($missing.Count -gt 0) {
    throw "Missing required settings in ${EnvFile}: $($missing -join ', ')"
}

if (-not $env:LOCALAPPDATA) {
    throw "LOCALAPPDATA is not set. Set GATEWAY_DB_PATH in the environment file."
}
if ($HealthOnly) {
    $dataDirectory = Join-Path $env:TEMP "CyphtGateway-health-only"
} else {
    $dataDirectory = Join-Path $env:LOCALAPPDATA "CyphtGateway"
}
New-Item -ItemType Directory -Force -Path $dataDirectory | Out-Null
$env:GATEWAY_DB_PATH = Join-Path $dataDirectory "gateway.db"
$env:GATEWAY_BIND = $Bind

$gnuToolchain = $null
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    $rustupHome = if ($env:RUSTUP_HOME) { $env:RUSTUP_HOME } else { Join-Path $env:USERPROFILE ".rustup" }
    $toolchainsDirectory = Join-Path $rustupHome "toolchains"
    if (Test-Path -LiteralPath $toolchainsDirectory -PathType Container) {
        $gnuToolchain = Get-ChildItem -LiteralPath $toolchainsDirectory -Directory |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+-x86_64-pc-windows-gnu$' } |
            Sort-Object Name -Descending |
            Select-Object -First 1 -ExpandProperty Name
    }
}

$originalPath = $env:PATH
$originalTargetDir = $env:CARGO_TARGET_DIR
$mappedDrive = $null
$buildRoot = $repositoryRoot
$mingwBin = if ($env:MSYS2_ROOT) {
    Join-Path $env:MSYS2_ROOT "mingw64\bin"
} else {
    "C:\msys64\mingw64\bin"
}
if ($gnuToolchain -and (Test-Path (Join-Path $mingwBin "gcc.exe")) -and
    (Test-Path (Join-Path $mingwBin "dlltool.exe"))) {
    $env:PATH = "$mingwBin;$env:PATH"
    $env:CARGO_TARGET_DIR = Join-Path $env:TEMP "CyphtGateway-native-target"
    if ($repositoryRoot -match '[^\x00-\x7F]') {
        $driveLetter = @("Z", "Y", "X", "W", "V") |
            Where-Object { -not (Get-PSDrive -Name $_ -ErrorAction SilentlyContinue) } |
            Select-Object -First 1
        if (-not $driveLetter) {
            throw "No free drive letter is available for this non-ASCII workspace path."
        }
        $mappedDrive = "${driveLetter}:"
        & "$env:WINDIR\System32\subst.exe" $mappedDrive $repositoryRoot
        if ($LASTEXITCODE -ne 0) {
            throw "Could not create a temporary ASCII path for the Windows GNU toolchain."
        }
        $buildRoot = "${mappedDrive}\"
    }
    Write-Host "Using installed development toolchain $gnuToolchain for this local run. Release CI remains pinned to Rust 1.88."
}

Push-Location $buildRoot
try {
    if ($gnuToolchain) {
        & cargo "+$gnuToolchain" run --release -p gatewayd
    } else {
        & cargo run --release -p gatewayd
    }
    $exitCode = $LASTEXITCODE
} finally {
    Pop-Location
    if ($mappedDrive) {
        & "$env:WINDIR\System32\subst.exe" $mappedDrive /d | Out-Null
    }
    $env:PATH = $originalPath
    if ($null -eq $originalTargetDir) {
        Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
    } else {
        $env:CARGO_TARGET_DIR = $originalTargetDir
    }
}
exit $exitCode

