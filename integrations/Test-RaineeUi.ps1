[CmdletBinding()]
param([int]$Port=8018,[string]$OutputRoot='D:\DevData\Rainee\verification\rainee-ui-restored-2026-10-11')
$ErrorActionPreference='Stop'
$resolved=[IO.Path]::GetFullPath($OutputRoot)
if(-not $resolved.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Test outputs must remain on D'}
if(Test-Path -LiteralPath $resolved){throw 'Use fresh output directory'}
function Send([string]$Method,$Params){$c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port);try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 100 -Compress));$w.Flush();$reply=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $reply.ok){throw "$Method`: $($reply.error)"};$reply.result}finally{$c.Dispose()}}
function Cmd([string]$Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
function Shot([string]$Name){Send 'ui.screenshot' @{path=(Join-Path $resolved "$Name.png");focus=$false}|Out-Null}
if(@((Send 'ui.inspect' @{}).documents).Count -gt 0){throw 'Use a fresh test app, never owner documents'}
New-Item -ItemType Directory -Path $resolved|Out-Null
Send 'ui.key' @{key='Escape'}|Out-Null
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
Start-Sleep -Milliseconds 300;Shot 'rainee-home'
Cmd 'file.new' @{name='Rainee Restored UI Trial';width=612;height=792}|Out-Null
$id=(Cmd 'path.create' @{d='M70 160 C180 60 260 270 370 160'}).id
$before=Cmd 'document.node' @{id=$id}|ConvertTo-Json -Depth 100 -Compress
Cmd 'window.workspace' @{name='Essentials'}|Out-Null
if((Send 'ui.inspect' @{}).ui.left_working_dock){throw 'Legacy workspace kept left dock'}
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
Cmd 'window.panel' @{panel='gradient'}|Out-Null
Start-Sleep -Milliseconds 100
if((Send 'ui.inspect' @{}).ui.left_appearance_tab -ne 2){throw 'Panel route missing'}
Cmd 'window.workspace.new' @{name='Restored Trial Saved UI'}|Out-Null
Cmd 'window.workspace' @{name='Essentials'}|Out-Null
Cmd 'window.workspace' @{name='Restored Trial Saved UI'}|Out-Null
if((Send 'ui.inspect' @{}).ui.left_appearance_tab -ne 2){throw 'Saved tabs missing'}
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
$experimental=@((Send 'ui.menu.list' @{})|Where-Object {$_.command -in @('window.sketchCleanup','window.researchTools')})
if($experimental.Count -ne 2){throw 'Experimental menu missing'}
foreach($entry in $experimental){if(($entry.path -join '/') -ne 'Window/Experimental'){throw 'Wrong menu location'};Cmd $entry.command @{open=$true}|Out-Null}
$state=Send 'ui.inspect' @{}
if(-not $state.sketchCleanupPanelOpen -or -not $state.researchToolsPanelOpen){throw 'Panels did not open'}
Shot 'experimental-panels'
foreach($entry in $experimental){Cmd $entry.command @{open=$false}|Out-Null}
if($before -ne ((Cmd 'document.node' @{id=$id})|ConvertTo-Json -Depth 100 -Compress)){throw 'UI operations changed artwork'}
Shot 'rainee-drawing'
Cmd 'app.home' @{}|Out-Null
Start-Sleep -Milliseconds 100;Shot 'home-with-document'
Send 'ui.click' @{x=83;y=254}|Out-Null
Start-Sleep -Milliseconds 100
if($null -ne (Send 'ui.inspect' @{}).ui.home){throw 'Back to document failed'}
Send 'app.save' @{path=(Join-Path $resolved 'restored-trial.vectorcraft')}|Out-Null
$watch=[Diagnostics.Stopwatch]::StartNew();do{Start-Sleep -Milliseconds 50;$state=Send 'ui.inspect' @{};if($watch.Elapsed.TotalSeconds -gt 30){throw 'Save deadline'}}while(@($state.background).Count -gt 0)
@{workspace=$state.ui.workspace;restored=$state.ui.left_working_dock;artUnchanged=$true;panelsRouted=$true;savedUiRetained=$true;homeReturn=$true;experimental=$experimental}|ConvertTo-Json -Depth 10|Set-Content -LiteralPath (Join-Path $resolved 'results.json') -Encoding utf8
Write-Output "Restored Rainee UI checks passed: $resolved"
