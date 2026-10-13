# Owner-specific development build, not a GPU-canvas or CUDA claim.
[CmdletBinding()]
param([ValidateRange(0,65535)][int]$ControlPort = 0, [switch]$NoPreferences)
$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
. (Join-Path $taskRoot 'integrations/storage/Use-DDrive.ps1') -Target Editor
$taskExe = 'D:\DevData\Rainee\build\editor\debug\vectorcraft.exe'
if (-not (Test-Path -LiteralPath $taskExe -PathType Leaf)) {
    throw 'Build first: cargo build --locked -p vectorcraft (after Use-DDrive.ps1 -Target Editor).'
}
if ($ControlPort -ne 0 -and (Get-NetTCPConnection -State Listen -LocalPort $ControlPort -ErrorAction SilentlyContinue)) {
    throw "Port $ControlPort is already in use; do not bind to an unrelated app instance."
}
$taskOldName = $env:WGPU_ADAPTER_NAME
$taskOldPower = $env:WGPU_POWER_PREF
$taskOldPrefs = $env:VECTORCRAFT_NO_PREFS
$taskOldPort = $env:VECTORCRAFT_CONTROL_PORT
try {
    $env:WGPU_ADAPTER_NAME = 'NVIDIA GeForce RTX 4080 SUPER'
    $env:WGPU_POWER_PREF = 'high'
    # Opt-in only: control is unauthenticated loopback access for trusted local clients.
    $env:VECTORCRAFT_CONTROL_PORT = $null
    if ($NoPreferences) { $env:VECTORCRAFT_NO_PREFS = '1' }
    $taskArguments = if ($ControlPort) { @('--control',"$ControlPort") } else { @() }
    $taskStart = @{ FilePath=$taskExe; WorkingDirectory=$taskRoot; PassThru=$true; WindowStyle='Normal' }
    if ($taskArguments.Count) { $taskStart.ArgumentList = $taskArguments }
    $taskProcess = Start-Process @taskStart
    [pscustomobject]@{ ProcessId=$taskProcess.Id; Executable=$taskExe; ControlPort=$ControlPort; RequestedAdapter=$env:WGPU_ADAPTER_NAME; Build='development' }
} finally {
    $env:WGPU_ADAPTER_NAME = $taskOldName
    $env:WGPU_POWER_PREF = $taskOldPower
    $env:VECTORCRAFT_NO_PREFS = $taskOldPrefs
    $env:VECTORCRAFT_CONTROL_PORT = $taskOldPort
}
