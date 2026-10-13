[CmdletBinding()]
param([int]$Port=7991,[string]$Cli='D:\DevData\Rainee\build\editor\debug\vectorcraft-cli.exe',[string]$OutputRoot='D:\DevData\Rainee\verification\three-tools-2026-10-11')
$ErrorActionPreference='Stop'
$taskStart=[Diagnostics.ProcessStartInfo]::new($Cli);$taskStart.ArgumentList.Add('mcp');$taskStart.ArgumentList.Add('--connect');$taskStart.ArgumentList.Add("127.0.0.1:$Port")
$taskStart.UseShellExecute=$false;$taskStart.CreateNoWindow=$true;$taskStart.RedirectStandardInput=$true;$taskStart.RedirectStandardOutput=$true
$taskProcess=[Diagnostics.Process]::Start($taskStart);$script:taskId=0
function Rpc([string]$Method,$Params){
 $script:taskId++;$taskProcess.StandardInput.WriteLine((@{jsonrpc='2.0';id=$script:taskId;method=$Method;params=$Params}|ConvertTo-Json -Compress -Depth 60));$taskProcess.StandardInput.Flush()
 do{$taskPending=$taskProcess.StandardOutput.ReadLineAsync();if(-not $taskPending.Wait(30000)){throw 'MCP response timed out'};$taskReply=$taskPending.Result|ConvertFrom-Json -Depth 100}while($taskReply.id -ne $script:taskId)
 if($taskReply.error){throw ($taskReply.error|ConvertTo-Json -Compress)};return $taskReply.result
}
function Mcp([string]$Name,$Arguments){$taskResult=Rpc 'tools/call' @{name=$Name;arguments=$Arguments};if($taskResult.isError){throw ($taskResult.content|ConvertTo-Json -Compress)};return ((@($taskResult.content)|Where-Object type -eq 'text'|ForEach-Object text)-join '')|ConvertFrom-Json -Depth 100}
function Cmd([string]$Command,$Params){Mcp 'run_command' @{command=$Command;params=$Params}}
try{
 $taskInit=Rpc 'initialize' @{protocolVersion='2025-11-25';capabilities=@{};clientInfo=@{name='Rainee research integration check';version='1'}}
 $taskProcess.StandardInput.WriteLine('{"jsonrpc":"2.0","method":"notifications/initialized"}');$taskProcess.StandardInput.Flush()
 $taskTools=Rpc 'tools/list' @{};$taskCommands=Mcp 'list_commands' @{filter='assist.'}
 Cmd 'file.new' @{name='MCP Junction Check';width=240;height=160}|Out-Null
 Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'stroke.set' @{weight=4}|Out-Null
 $taskIds=@();foreach($taskPath in @('M20 60 L60 60','M64 60 L110 60')){$taskIds+=(Cmd 'path.create' @{d=$taskPath}).id}
 Cmd 'assist.junction.preview' @{ids=$taskIds;gap=8;confidence=0}|Out-Null
 $taskWatch=[Diagnostics.Stopwatch]::StartNew();do{Start-Sleep -Milliseconds 30;$taskStatus=Cmd 'assist.status' @{};if($taskWatch.Elapsed.TotalSeconds -gt 30){throw 'MCP junction worker deadline'}}while($taskStatus.state -eq 'running')
 if($taskStatus.state -ne 'ready' -or @($taskStatus.junctions).Count -lt 1){throw ($taskStatus|ConvertTo-Json -Depth 20 -Compress)}
 $taskApplied=Cmd 'assist.accept' @{job=$taskStatus.job;indices=@(0)}
 Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
 Cmd 'tool.setOption' @{tool='paintbrush';values=@{learnedPrediction=$true;predictionMs=14}}|Out-Null
 Cmd 'tool.setOption' @{tool='paintbrush';values=@{learnedPrediction=$false;predictionMs=0}}|Out-Null
 Cmd 'assist.brush.preview' @{ids=$taskIds;seed=594}|Out-Null
 $taskWatch.Restart();do{Start-Sleep -Milliseconds 30;$taskBrush=Cmd 'assist.status' @{};if($taskWatch.Elapsed.TotalSeconds -gt 135){throw 'MCP natural brush deadline'}}while($taskBrush.state -eq 'running')
 if($taskBrush.state -ne 'ready'){throw ($taskBrush|ConvertTo-Json -Depth 20 -Compress)}
 $taskRaster=Cmd 'assist.accept' @{job=$taskBrush.job}
 Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
 Cmd 'assist.brush.release' @{id=$taskRaster.group}|Out-Null
 $taskTelemetry=Cmd 'assist.telemetry' @{}
 if(@($taskTelemetry.history | Where-Object { $_.kind -eq 'brush' -and $_.state -eq 'accepted' }).Count -lt 1){throw 'MCP brush acceptance not recorded'}
 $taskReport=@{handshake=$taskInit;tools=@($taskTools.tools).Count;commands=$taskCommands;modelStatus=$taskStatus;approvedEdit=$taskApplied;brush=$taskBrush;approvedRaster=$taskRaster;telemetry=$taskTelemetry;undoRedo=$true;learnedControls=$true;physical_replay='deferred'}
 New-Item -ItemType Directory -Force -Path $OutputRoot |Out-Null;$taskReport|ConvertTo-Json -Depth 30|Set-Content -Encoding utf8 -LiteralPath (Join-Path $OutputRoot 'mcp-results.json')
 $taskReport|ConvertTo-Json -Depth 4
}finally{$taskProcess.StandardInput.Close();if(-not $taskProcess.WaitForExit(5000)){$taskProcess.Kill()};$taskProcess.Dispose()}
