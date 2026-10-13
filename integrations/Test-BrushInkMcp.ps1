[CmdletBinding()]
param([int]$Port=8010,[string]$Package='D:\DevData\Rainee\build\VectorCraft-Rainee-Ink-2026-10-11-Verified',[string]$OutputRoot='D:\DevData\Rainee\verification\brush-ink-final-package-2026-10-11')
$ErrorActionPreference='Stop'
$taskStart=[Diagnostics.ProcessStartInfo]::new((Join-Path $Package 'vectorcraft-cli.exe'))
foreach($a in @('mcp','--connect',"127.0.0.1:$Port")){$taskStart.ArgumentList.Add($a)}
$taskStart.UseShellExecute=$false;$taskStart.CreateNoWindow=$true;$taskStart.RedirectStandardInput=$true;$taskStart.RedirectStandardOutput=$true
$taskProcess=[Diagnostics.Process]::Start($taskStart);$script:taskId=0
function Rpc([string]$Method,$Params){$script:taskId++;$taskProcess.StandardInput.WriteLine((@{jsonrpc='2.0';id=$script:taskId;method=$Method;params=$Params}|ConvertTo-Json -Compress -Depth 60));$taskProcess.StandardInput.Flush();do{$pending=$taskProcess.StandardOutput.ReadLineAsync();if(-not $pending.Wait(30000)){throw 'MCP deadline'};$reply=$pending.Result|ConvertFrom-Json -Depth 100}while($reply.id -ne $script:taskId);if($reply.error){throw ($reply.error|ConvertTo-Json -Compress)};$reply.result}
function Mcp([string]$Name,$Arguments){$r=Rpc 'tools/call' @{name=$Name;arguments=$Arguments};if($r.isError){throw ($r.content|ConvertTo-Json -Compress)};((@($r.content)|Where-Object type -eq 'text'|ForEach-Object text)-join '')|ConvertFrom-Json -Depth 100}
function Cmd([string]$Command,$Params){Mcp 'run_command' @{command=$Command;params=$Params}}
try{
 $init=Rpc 'initialize' @{protocolVersion='2025-11-25';capabilities=@{};clientInfo=@{name='Rainee ink package check';version='1'}}
 $taskProcess.StandardInput.WriteLine('{"jsonrpc":"2.0","method":"notifications/initialized"}');$taskProcess.StandardInput.Flush()
 $tools=Rpc 'tools/list' @{}
 $brushCommands=Mcp 'list_commands' @{filter='brush.'}
 if(($brushCommands|ConvertTo-Json -Depth 30 -Compress) -notmatch 'inkLoad'){throw 'Ink controls missing from discovery'}
 $cleanupCommands=Mcp 'list_commands' @{filter='sketch.cleanup.'}
 if(($cleanupCommands|ConvertTo-Json -Depth 30 -Compress) -notmatch 'sketch.cleanup.inspect'){throw 'Cleanup preflight missing from discovery'}
 Cmd 'file.new' @{name='MCP Ink Controls Trial';width=240;height=160}|Out-Null
 Cmd 'paint.setFill' @{none=$true}|Out-Null
 Cmd 'brush.new' @{type='natural';name='MCP Pooled Ink';params=@{media='dryInk';inkLoad=1.4;edgePooling=0.8;breakup=0.2;paperScale=2;grain=0.5}}|Out-Null
 $id=(Cmd 'path.create' @{d='M20 90 C60 20 140 20 200 90'}).id
 Cmd 'brush.apply' @{name='MCP Pooled Ink';ids=@($id)}|Out-Null
 $before=(Cmd 'document.node' @{id=$id})|ConvertTo-Json -Depth 60 -Compress
 Cmd 'brush.options' @{name='MCP Pooled Ink';params=@{inkLoad=1.8;edgePooling=1}}|Out-Null
 $invalid=Rpc 'tools/call' @{name='run_command';arguments=@{command='brush.options';params=@{name='MCP Pooled Ink';params=@{paperScale=0}}}}
 if(-not $invalid.isError){throw 'Invalid ink control was accepted'}
 if($before -ne ((Cmd 'document.node' @{id=$id})|ConvertTo-Json -Depth 60 -Compress)){throw 'Ink options changed the source node'}
 Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
 $effects=Cmd 'effect.list' @{}
 $wetDiscovered=@($effects.catalog|Where-Object id -eq 'paint.wetMedia').Count -eq 1
 if(-not $wetDiscovered){throw 'Wet media effect missing from MCP discovery'}
 Cmd 'effect.apply' @{ids=@($id);effect='paint.wetMedia';item=$null;params=@{spread=6;water=1.5;steps=24}}|Out-Null
 if(((Cmd 'document.node' @{id=$id})|ConvertTo-Json -Depth 100 -Compress) -notmatch 'paint.wetMedia'){throw 'MCP wet media edit missing'}
 Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
 $admission=Cmd 'sketch.cleanup.inspect' @{}
 if($admission.eligible){throw 'Singleton selection incorrectly admitted'}
 @{handshake=$init;tools=$tools.tools.Count;inkControlsDiscovered=$true;wetMediaDiscovered=$wetDiscovered;wetMediaEdited=$true;cleanupPreflightDiscovered=$true;realCanvasEdit=$true;sourceRetained=$true;invalidOptionsRejected=$true;undoRedo=$true;preflight=$admission}|ConvertTo-Json -Depth 30|Set-Content -LiteralPath (Join-Path $OutputRoot 'mcp-ink-results.json') -Encoding utf8
 Write-Output 'Packaged MCP discovery, ink editing, validation, undo/redo and preflight passed.'
}finally{$taskProcess.StandardInput.Close();if(-not $taskProcess.WaitForExit(5000)){$taskProcess.Kill()};$taskProcess.Dispose()}
