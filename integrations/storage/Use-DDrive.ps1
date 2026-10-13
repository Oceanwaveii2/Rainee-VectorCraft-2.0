# Dot-source before a Rainee build: . ./integrations/storage/Use-DDrive.ps1 -Target Probe
# Changes this shell/process and its children only; no Windows-wide TEMP or PATH changes.
[CmdletBinding()]
param([ValidateSet('Probe','Editor')][string]$Target = 'Probe')
$ErrorActionPreference = 'Stop'
$taskCargo = 'D:\DevData\Rust\cargo'
$taskRustup = 'D:\DevData\Rust\rustup'
if (-not (Test-Path -LiteralPath (Join-Path $taskCargo 'bin\cargo.exe')) -or -not (Test-Path -LiteralPath (Join-Path $taskRustup 'settings.toml'))) {
    throw 'Verified Rust storage migration must finish before selecting D build paths.'
}
$taskLogRoot = 'D:\DevData\MigrationLogs\Rainee-2026-10-11'
$taskRequiredManifests = @('cargo-verified.json','rustup-verified.json') + $(if ($Target -eq 'Probe') { 'cintiq-build-verified.json' } else { 'Rainee-ArtCraft-target-verified.json' })
foreach ($taskManifest in $taskRequiredManifests) {
    if (-not (Test-Path -LiteralPath (Join-Path $taskLogRoot $taskManifest))) { throw "Migration is not yet verified: $taskManifest" }
}
$taskTemp = 'D:\DevData\Rainee\temp'
$taskBuild = if ($Target -eq 'Probe') {
    'D:\DevData\Rainee\build\Cintiq-Input-Probe-2026-10-11'
} else {
    'D:\DevData\Rainee\build\editor'
}
New-Item -ItemType Directory -Path $taskTemp,$taskBuild -Force | Out-Null
$env:CARGO_HOME = $taskCargo
$env:RUSTUP_HOME = $taskRustup
$env:CARGO_TARGET_DIR = $taskBuild
$env:TEMP = $taskTemp
$env:TMP = $taskTemp
$taskCargoBin = Join-Path $taskCargo 'bin'
if (($env:PATH -split ';') -notcontains $taskCargoBin) { $env:PATH = $taskCargoBin + ';' + $env:PATH }
Write-Output "Rainee $Target builds: $taskBuild"
Write-Output "Rust dependencies/toolchains: D:\DevData\Rust; process temporary files: $taskTemp"
