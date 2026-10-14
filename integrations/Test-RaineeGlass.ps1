param(
    [int]$Port=8041,
    [string]$Fixture='D:\DevData\Rainee\verification\rainee-theme-2026-10-13\glass-test.svg',
    [string]$OutputRoot='D:\DevData\Rainee\verification\rainee-theme-2026-10-13\glass-native'
)
$ErrorActionPreference='Stop'
function Send([string]$Method,$Params) {
    $json=$Params|ConvertTo-Json -Depth 100 -Compress
    (& "$PSScriptRoot\Invoke-RaineeUIReview.ps1" -Port $Port -Method $Method -ParamsJson $json)|ConvertFrom-Json -Depth 100
}
function Cmd($Name,$Params) { Send 'engine.execute' @{command=$Name;params=$Params} }
$root=[IO.Path]::GetFullPath($OutputRoot)
if(-not $root.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Output must remain in verification'}
if(@((Send 'ui.inspect' @{}).documents).Count){throw 'Use a fresh isolated test process without owner documents'}
New-Item -ItemType Directory -Path $root -Force|Out-Null
Send 'ui.key' @{key='Escape'}|Out-Null
Start-Sleep -Milliseconds 350
Send 'app.open' @{path=$Fixture}|Out-Null
Start-Sleep -Seconds 2
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
Cmd 'prefs.set' @{key='uiGlassIntensity';value=100}|Out-Null
Cmd 'select.all' @{}|Out-Null
$before=Send 'document.inspect' @{}
Send 'ui.screenshot' @{path="$root\workspace-2.png";focus=$false}|Out-Null
Cmd 'edit.preferences' @{category='User Interface'}|Out-Null
Start-Sleep -Milliseconds 700
$glass=Send 'ui.inspect' @{}
if(-not $glass.glass.enabled -or $glass.glass.surfaces -lt 1 -or $glass.glass.rebuilds -lt 1){throw "No native glass surface: $($glass.glass|ConvertTo-Json -Compress)"}
Send 'ui.screenshot' @{path="$root\glass-on.png";focus=$false}|Out-Null
Start-Sleep -Seconds 1
$idle=Send 'ui.inspect' @{}
if($idle.glass.rebuilds -ne $glass.glass.rebuilds){throw 'Idle glass rebuilt its scattering texture'}
Send 'ui.dialog.set' @{field='uiReduceTransparency';value=$true}|Out-Null
Start-Sleep -Milliseconds 350
$opaque=Send 'ui.inspect' @{}
if($opaque.glass.enabled){throw 'Reduced transparency did not disable glass'}
Send 'ui.screenshot' @{path="$root\glass-opaque.png";focus=$false}|Out-Null
Send 'ui.dialog.cancel' @{}|Out-Null
Start-Sleep -Milliseconds 350
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
Start-Sleep -Milliseconds 350
$legacy=Send 'ui.inspect' @{}
if($legacy.glass.enabled){throw 'Glass leaked into the legacy workspace'}
Send 'ui.screenshot' @{path="$root\workspace-1.png";focus=$false}|Out-Null
$after=Send 'document.inspect' @{}
if(($before.layers|ConvertTo-Json -Depth 100 -Compress) -ne ($after.layers|ConvertTo-Json -Depth 100 -Compress)){throw 'Theme changes mutated artwork'}
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
@{glass=$glass;idle=$idle;opaque=$opaque;legacy=$legacy;artworkUnchanged=$true}|ConvertTo-Json -Depth 100|Set-Content -LiteralPath "$root\telemetry.json" -Encoding utf8
'PASS: native glass, idle cache, opaque preview, legacy isolation, unchanged artwork'
