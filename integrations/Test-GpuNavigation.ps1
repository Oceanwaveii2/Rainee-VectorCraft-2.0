[CmdletBinding()]
param([int]$Port=8021,[string]$OutputRoot='D:\DevData\Rainee\verification\gpu-navigation-2026-10-11')
$ErrorActionPreference='Stop'
$resolved=[IO.Path]::GetFullPath($OutputRoot)
if(-not $resolved.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Outputs must remain on D'}
if(Test-Path -LiteralPath $resolved){throw 'Use fresh output directory'}
function Send([string]$Method,$Params){$c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port);try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 100 -Compress));$w.Flush();$reply=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $reply.ok){throw "$Method`: $($reply.error)"};$reply.result}finally{$c.Dispose()}}
function Cmd([string]$Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
function Shot([string]$Name){Send 'ui.screenshot' @{path=(Join-Path $resolved "$Name.png");focus=$false}|Out-Null}
if(@((Send 'ui.inspect' @{}).documents).Count){throw 'Use a fresh app, never owner documents'}
New-Item -ItemType Directory -Path $resolved|Out-Null
Send 'ui.key' @{key='Escape'}|Out-Null
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
Cmd 'file.new' @{name='GPU Navigation Trial';width=700;height=700}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null
Cmd 'paint.setStroke' @{color='#245c92'}|Out-Null
$ops=@()
for($i=0;$i -lt 1000;$i++){$x=($i%32)*21;$y=[Math]::Floor($i/32)*21;$ops+=@{command='path.create';params=@{d="M$x $y l15 7 l-6 12"}}}
$made=Cmd 'command.batch' @{commands=$ops;label='Synthetic navigation fixture'}
Cmd 'select.none' @{}|Out-Null
Start-Sleep -Seconds 2
Send 'ui.tool.select' @{tool='eraser'}|Out-Null
$eraser=(Send 'ui.inspect' @{}).toolOptions
if(-not $eraser.eraseAll){throw 'Eraser default is not all editable paths'}
Send 'ui.tool.select' @{tool='selection'}|Out-Null
$initial=Send 'document.inspect' @{}
$runs=@()
foreach($enabled in @($false,$true,$false,$true)){
 Cmd 'view.gpuNavigation' @{enabled=$enabled}|Out-Null
 Cmd 'view.setZoom' @{zoom=100;center=@(350,350)}|Out-Null
 Start-Sleep -Milliseconds 500
 $before=Send 'ui.inspect' @{}
 if($enabled -and -not $before.gpuNavigation.available){throw 'No GPU compositor available'}
 $frames=@();$watch=[Diagnostics.Stopwatch]::StartNew()
 for($i=0;$i -lt 40;$i++){
  $zoom=100+($i%8)*5
  Cmd 'view.setZoom' @{zoom=$zoom;center=@((350+($i%10)*3),(350+($i%6)*2))}|Out-Null
  $state=Send 'ui.inspect' @{}
  $frames+=[double]$state.perf.frameMs
 }
 $watch.Stop()
 Cmd 'view.setZoom' @{zoom=150;center=@(380,370)}|Out-Null
 Start-Sleep -Seconds 1
 $after=Send 'ui.inspect' @{}
 $sorted=@($frames|Sort-Object)
 $runs+=@{enabled=$enabled;frameMedianMs=$sorted[20];frameP95Ms=$sorted[37];rpcWallMs=$watch.Elapsed.TotalMilliseconds;deferredFrames=$after.gpuNavigation.deferredFrames-$before.gpuNavigation.deferredFrames;submissions=$after.gpuNavigation.rasterSubmissions-$before.gpuNavigation.rasterSubmissions;uploads=$after.gpuNavigation.textureUploads-$before.gpuNavigation.textureUploads}
 Cmd 'select.none' @{}|Out-Null
 Start-Sleep -Milliseconds 300
 Shot ("settled-"+$enabled+"-"+$runs.Count)
}
if(-not @($runs|Where-Object {$_.enabled -and $_.deferredFrames -gt 0}).Count){throw 'GPU reuse was not exercised'}
if(($initial.layers|ConvertTo-Json -Depth 70 -Compress) -ne ((Send 'document.inspect' @{}).layers|ConvertTo-Json -Depth 70 -Compress)){throw 'Navigation altered artwork'}
Send 'app.save' @{path=(Join-Path $resolved 'navigation-trial.vectorcraft')}|Out-Null
$info=Send 'ui.inspect' @{}
@{runs=$runs;adapter=$info.graphicsAdapter;canvas=$info.canvasRect;window=$info.window;eraserDefault=$eraser;artworkUnchanged=$true;policy='Two alternating runs each mode; 40 native RPC navigation steps. Frame telemetry excludes GPU timestamp/input-to-photon measurement.'}|ConvertTo-Json -Depth 12|Set-Content -LiteralPath (Join-Path $resolved 'results.json') -Encoding utf8
$runs|ConvertTo-Json -Depth 6
