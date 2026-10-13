[CmdletBinding()]
param([int]$Port=7994,[string]$OutputRoot='D:\DevData\Rainee\verification\embedded-codex-2026-10-11')
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
function Send([string]$Method,$Params){
 $c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
 try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true)
 $w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 60 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result
 }finally{$c.Dispose()}
}
function Cmd([string]$Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
function Wait-Model([bool]$Approve){
 $watch=[Diagnostics.Stopwatch]::StartNew();$approvals=0
 do {
  $status=Cmd 'codex.status' @{}
  if($status.proposal){if(-not $Approve){throw 'Unexpected approval in trusted test'};if($status.proposal.arguments.command -ne 'path.create'){throw "Unexpected proposed command $($status.proposal.arguments.command)"};Cmd 'codex.approve' @{}|Out-Null;$approvals++}
  if($watch.Elapsed.TotalSeconds -gt 420){Cmd 'codex.cancel' @{}|Out-Null;throw 'Model deadline'}
  Start-Sleep -Milliseconds 100
 }while($status.running)
 if($status.state -notlike 'Model finished*'){throw ($status|ConvertTo-Json -Depth 15 -Compress)}
 @{status=$status;approvals=$approvals}
}
if(@((Send 'ui.inspect' @{}).documents).Count -gt 0){throw 'Fresh disposable instance required'}
Cmd 'file.new' @{name='Embedded Codex Approved Trial';width=240;height=160}|Out-Null
Cmd 'window.codexEditor' @{open=$true}|Out-Null
Cmd 'codex.request' @{prompt='Use inspect_editor and discover_commands (filter path.create). Create exactly one editable path by calling run_editor_command with command path.create and params {"d":"M20 20 C60 100 120 100 180 20"}. Do not change any other property. After the host approves it, call inspect_canvas to visually inspect the result and finish.';trusted=$false}|Out-Null
$approved=Wait-Model $true
if($approved.approvals -ne 1){throw 'Expected exactly one owner-approved operation'}
if(@($approved.status.history|Where-Object state -eq executed).Count -ne 1){throw 'Approved edit not executed'}
Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
Send 'app.save' @{path=(Join-Path $OutputRoot 'approved.vectorcraft')}|Out-Null
Cmd 'file.new' @{name='Embedded Codex Trusted Trial';width=240;height=160}|Out-Null
Cmd 'codex.request' @{prompt='Inspect the editor, discover path.create, then create exactly one editable path using run_editor_command with command path.create and params {"d":"M30 100 L180 100"}. No other changes. Call inspect_canvas afterward and finish.';trusted=$true}|Out-Null
$trusted=Wait-Model $false
if(@($trusted.status.history|Where-Object state -eq executed).Count -lt 2){throw 'Trusted operation not recorded'}
Send 'ui.screenshot' @{path=(Join-Path $OutputRoot 'embedded-panel.png');focus=$true}|Out-Null
@{approved=$approved;trusted=$trusted;physicalCintiq='deferred';replay='deferred'}|ConvertTo-Json -Depth 50 | Set-Content -LiteralPath (Join-Path $OutputRoot 'results.json') -Encoding utf8
'Embedded live approved/trusted operations completed'
