[CmdletBinding()]
param([int]$Port=7991,[string]$OutputRoot='D:\DevData\Rainee\verification\three-tools-2026-10-11')
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
function Invoke-Research([string]$Method,$Params){
 $taskClient=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
 try{$taskClient.ReceiveTimeout=30000;$taskStream=$taskClient.GetStream();$taskWriter=[IO.StreamWriter]::new($taskStream,[Text.UTF8Encoding]::new($false),4096,$true);$taskReader=[IO.StreamReader]::new($taskStream,[Text.UTF8Encoding]::new($false),$false,4096,$true)
 $taskWriter.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 60 -Compress));$taskWriter.Flush();$taskReply=$taskReader.ReadLine()|ConvertFrom-Json -Depth 100
 if(-not $taskReply.ok){throw "$Method failed: $($taskReply.error)"};return $taskReply.result
 }finally{$taskClient.Dispose()}
}
function Cmd([string]$Name,$Params){Invoke-Research 'engine.execute' @{command=$Name;params=$Params}}
function Wait-Research {
 $taskWatch=[Diagnostics.Stopwatch]::StartNew()
 do {Start-Sleep -Milliseconds 30;$taskStatus=Cmd 'assist.status' @{};if($taskWatch.Elapsed.TotalSeconds -gt 135){throw 'Research worker deadline exceeded'}}while($taskStatus.state -eq 'running')
 if($taskStatus.state -ne 'ready'){throw ($taskStatus|ConvertTo-Json -Depth 20 -Compress)};return $taskStatus
}
if(@((Invoke-Research 'ui.inspect' @{}).documents).Count -gt 0){throw 'Use a fresh disposable instance, never owner documents'}
$taskCommands=Invoke-Research 'engine.commands' @{}
foreach($taskName in @('assist.junction.preview','assist.brush.preview','assist.status','assist.telemetry','assist.accept','assist.cancel','assist.reject','assist.brush.release','window.researchTools')){if($taskCommands.id -notcontains $taskName){throw "Discovery missing $taskName"}}
Cmd 'file.new' @{name='Junction Integration Trial';width=240;height=160}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'paint.setStroke' @{color='#202020'}|Out-Null;Cmd 'stroke.set' @{weight=4}|Out-Null
$taskIds=@();foreach($taskPath in @('M20 60 L60 60','M64 60 L110 60')){$taskIds+=(Cmd 'path.create' @{d=$taskPath}).id}
Cmd 'select.set' @{ids=$taskIds}|Out-Null
$taskBefore=@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress
Cmd 'assist.junction.preview' @{ids=$taskIds;confidence=0;gap=8}|Out-Null
$taskJoin=Wait-Research
if(@($taskJoin.junctions).Count -lt 1){throw 'No junction classifier evaluations yielded suggestions'}
$taskAfterPreview=@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress
if($taskBefore -ne $taskAfterPreview){throw 'Junction preview modified original geometry'}
Cmd 'assist.accept' @{job=$taskJoin.job;indices=@(0)}|Out-Null
if(((Cmd 'assist.status' @{}).telemetry|ConvertTo-Json -Depth 20 -Compress) -ne ($taskJoin.telemetry|ConvertTo-Json -Depth 20 -Compress)){throw 'Accepted job lost backend telemetry'}
$taskAfter=@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress
Cmd 'edit.undo' @{}|Out-Null
if($taskBefore -ne (@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress)){throw 'Junction Undo failed'}
Cmd 'edit.redo' @{}|Out-Null
if($taskAfter -ne (@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress)){throw 'Junction Redo failed'}
Cmd 'assist.junction.preview' @{ids=$taskIds;confidence=0}|Out-Null
$taskRunning=Cmd 'assist.status' @{}
if($taskRunning.state -eq 'running' -and $null -ne $taskRunning.telemetry){throw 'Running job incorrectly inherited previous timing'}
Cmd 'path.create' @{d='M170 100 L200 110'}|Out-Null
if((Cmd 'assist.status' @{}).state -ne 'stale'){throw 'Content change did not revoke proposal'}
Cmd 'file.new' @{name='Neural Brush Integration Trial';width=240;height=160}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'paint.setStroke' @{color='#202020'}|Out-Null;Cmd 'stroke.set' @{weight=5}|Out-Null
$taskIds=@();foreach($taskPath in @('M30 90 C45 20 100 20 120 90','M30 110 L120 110')){$taskIds+=(Cmd 'path.create' @{d=$taskPath}).id}
Cmd 'select.set' @{ids=$taskIds}|Out-Null
$taskBefore=@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress
Cmd 'assist.brush.preview' @{ids=$taskIds;seed=594;color=@(0.1,0.25,0.65)}|Out-Null
$taskBrush=Wait-Research
if($taskBefore -ne (@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress)){throw 'Brush preview modified vector sources'}
Cmd 'window.researchTools' @{open=$true}|Out-Null
Invoke-Research 'ui.screenshot' @{path=(Join-Path $OutputRoot 'brush-preview.png');focus=$true}|Out-Null
$taskAccepted=Cmd 'assist.accept' @{job=$taskBrush.job}
$taskGroup=Cmd 'document.node' @{id=$taskAccepted.group}
Cmd 'edit.undo' @{}|Out-Null
if($taskBefore -ne (@($taskIds|ForEach-Object{Cmd 'document.node' @{id=$_}})|ConvertTo-Json -Depth 60 -Compress)){throw 'Brush Undo failed'}
Cmd 'edit.redo' @{}|Out-Null
$taskSaved=Join-Path $OutputRoot 'neural-appearance.vectorcraft'
Invoke-Research 'app.save' @{path=$taskSaved}|Out-Null
foreach($taskFormat in @('svg','pdf')){Invoke-Research 'app.export' @{path=(Join-Path $OutputRoot "neural-appearance.$taskFormat");format=$taskFormat}|Out-Null}
$taskWatch=[Diagnostics.Stopwatch]::StartNew()
while((Invoke-Research 'document.inspect' @{}).dirty){if($taskWatch.Elapsed.TotalSeconds -gt 30){throw 'Save did not complete'};Start-Sleep -Milliseconds 30}
Invoke-Research 'app.open' @{path=$taskSaved}|Out-Null
if(($taskGroup|ConvertTo-Json -Depth 60 -Compress) -ne ((Cmd 'document.node' @{id=$taskAccepted.group})|ConvertTo-Json -Depth 60 -Compress)){throw 'Reopened appearance/source group changed'}
Cmd 'assist.brush.release' @{id=$taskAccepted.group}|Out-Null
Cmd 'assist.brush.preview' @{ids=$taskIds}|Out-Null
if((Cmd 'assist.cancel' @{}).state -ne 'cancelled'){throw 'Cancellation acknowledgement failed'}
Cmd 'tool.setOption' @{tool='paintbrush';values=@{learnedPrediction=$true;predictionMs=14}}|Out-Null
Cmd 'tool.select' @{tool='paintbrush'}|Out-Null
if(-not (Invoke-Research 'ui.inspect' @{}).toolOptions.learnedPrediction){throw 'Learned control not enabled'}
Cmd 'tool.setOption' @{tool='paintbrush';values=@{learnedPrediction=$false;predictionMs=0}}|Out-Null
# Leave an unaccepted fresh brush preview available for the later combined owner test.
Cmd 'assist.brush.preview' @{ids=$taskIds;seed=594}|Out-Null
$taskReady=Wait-Research
$taskTelemetry=Cmd 'assist.telemetry' @{}
if(@($taskTelemetry.history).Count -gt 32 -or $taskTelemetry.limit -ne 32){throw 'Telemetry history must be bounded'}
if(@($taskTelemetry.history | Where-Object state -eq 'cancelled').Count -lt 1){throw 'Cancelled job missing from telemetry'}
if(@($taskTelemetry.history | Where-Object state -eq 'stale').Count -lt 1){throw 'Stale job missing from telemetry'}
Invoke-Research 'ui.screenshot' @{path=(Join-Path $OutputRoot 'ready-for-owner.png');focus=$true}|Out-Null
$taskReport=@{junction=$taskJoin;brush=$taskBrush;ready=$taskReady;telemetry=$taskTelemetry;checks=@('discovery','nonmutating preview','junction accept','undo/redo','stale document','raster/source retention','native save/reopen','SVG/PDF export','release','cancel','learned option routing','bounded telemetry','retained backend timing');cintiq_replay='deferred';physical_acceptance='pending'}
$taskReport|ConvertTo-Json -Depth 30|Set-Content -LiteralPath (Join-Path $OutputRoot 'results.json') -Encoding utf8
$taskReport|ConvertTo-Json -Depth 6
