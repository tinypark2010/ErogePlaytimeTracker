param(
    [Parameter(Mandatory = $false)]
    [ValidatePattern('^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$')]
    [string]$ExpectedVersion
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..\..')).Path
$packageJsonPath = Join-Path $repositoryRoot 'package.json'
$packageLockPath = Join-Path $repositoryRoot 'package-lock.json'
$cargoTomlPath = Join-Path $repositoryRoot 'src-tauri\Cargo.toml'
$cargoLockPath = Join-Path $repositoryRoot 'src-tauri\Cargo.lock'
$tauriConfigPath = Join-Path $repositoryRoot 'src-tauri\tauri.conf.json'

$packageJson = Get-Content -Raw -Encoding utf8 $packageJsonPath | ConvertFrom-Json
$packageLock = Get-Content -Raw -Encoding utf8 $packageLockPath
$tauriConfig = Get-Content -Raw -Encoding utf8 $tauriConfigPath | ConvertFrom-Json
$cargoToml = Get-Content -Raw -Encoding utf8 $cargoTomlPath
$cargoLock = Get-Content -Raw -Encoding utf8 $cargoLockPath

$packageLockMatch = [regex]::Match(
    $packageLock,
    '(?ms)^\s*\{\s*"name"\s*:\s*"eroge-playtime-tracker",\s*"version"\s*:\s*"(?<version>[^"]+)"'
)
$rootPackageMatch = [regex]::Match(
    $packageLock,
    '(?ms)"packages"\s*:\s*\{\s*""\s*:\s*\{\s*"name"\s*:\s*"eroge-playtime-tracker",\s*"version"\s*:\s*"(?<version>[^"]+)"'
)
$cargoTomlMatch = [regex]::Match(
    $cargoToml,
    '(?ms)^\[package\]\s*.*?^version\s*=\s*"(?<version>[^"]+)"'
)
$cargoLockMatch = [regex]::Match(
    $cargoLock,
    '(?ms)^\[\[package\]\]\r?\nname\s*=\s*"eroge-playtime-tracker"\r?\nversion\s*=\s*"(?<version>[^"]+)"'
)

if (-not $packageLockMatch.Success) {
    throw 'Could not find the top-level version in package-lock.json.'
}
if (-not $rootPackageMatch.Success) {
    throw 'Could not find the root package version in package-lock.json.'
}
if (-not $cargoTomlMatch.Success) {
    throw 'Could not find the application package version in src-tauri/Cargo.toml.'
}
if (-not $cargoLockMatch.Success) {
    throw 'Could not find the application package version in src-tauri/Cargo.lock.'
}

$versions = [ordered]@{
    'package.json' = [string]$packageJson.version
    'package-lock.json' = $packageLockMatch.Groups['version'].Value
    'package-lock.json root package' = $rootPackageMatch.Groups['version'].Value
    'src-tauri/Cargo.toml' = $cargoTomlMatch.Groups['version'].Value
    'src-tauri/Cargo.lock' = $cargoLockMatch.Groups['version'].Value
    'src-tauri/tauri.conf.json' = [string]$tauriConfig.version
}

$referenceVersion = if ($ExpectedVersion) { $ExpectedVersion } else { $versions['package.json'] }
$mismatches = @($versions.GetEnumerator() | Where-Object { $_.Value -ne $referenceVersion })

$versions.GetEnumerator() | ForEach-Object {
    Write-Output ("{0}: {1}" -f $_.Key, $_.Value)
}

if ($mismatches.Count -gt 0) {
    $details = $mismatches | ForEach-Object { "{0}={1}" -f $_.Key, $_.Value }
    throw "Release versions do not all equal $referenceVersion`: $($details -join ', ')"
}

Write-Output "Release version check passed: $referenceVersion"
