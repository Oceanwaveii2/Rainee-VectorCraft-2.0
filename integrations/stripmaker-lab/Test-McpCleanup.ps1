[CmdletBinding()]
param([int]$Port=7987,[string]$Cli='D:\DevData\Rainee\build\editor\debug\vectorcraft-cli.exe',[string]$OutputRoot='D:\DevData\Rainee\verification\editable-cleanup-final')
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$start=[Diagnostics.ProcessStartInfo]::new($Cli)
$start.ArgumentList.Add('mcp');$start.ArgumentList.Add('--connect');$start.ArgumentList.Add("127.0.0.1:$Port")
$start.UseShellExecute=$false;$start.CreateNoWindow=$true;$start.RedirectStandardInput=$true;$start.RedirectStandardOutput=$true
$process=[Diagnostics.Process]::Start($start)
$script:taskRpcId=0
function Invoke-CleanupMcp([string]$Method,$Params) {
    $script:taskRpcId++
    $process.StandardInput.WriteLine((@{jsonrpc='2.0';id=$script:taskRpcId;method=$Method;params=$Params}|ConvertTo-Json -Compress -Depth 60));$process.StandardInput.Flush()
    do {
        $pending=$process.StandardOutput.ReadLineAsync()
        if (-not $pending.Wait(60000)) {throw 'MCP reply deadline exceeded'}
        $line=$pending.Result
        if ($null -eq $line) {throw 'MCP process disconnected'}
        $reply=$line|ConvertFrom-Json -Depth 100
    } while ($reply.id -ne $script:taskRpcId)
    if ($reply.error) {throw ($reply.error|ConvertTo-Json -Compress)}
    return $reply.result
}
function Invoke-CleanupMcpTool([string]$Tool,$Arguments) {
    $result=Invoke-CleanupMcp 'tools/call' @{name=$Tool;arguments=$Arguments}
    if ($result.isError) {throw ($result.content|ConvertTo-Json -Compress)}
    $text=(@($result.content)|Where-Object type -eq 'text'|ForEach-Object text)-join ''
    return $text|ConvertFrom-Json -Depth 100
}
function Invoke-CleanupMcpCommand([string]$Command,$Params) {Invoke-CleanupMcpTool 'run_command' @{command=$Command;params=$Params}}
function Wait-CleanupPreview {
    $watch=[Diagnostics.Stopwatch]::StartNew()
    do {
        Start-Sleep -Milliseconds 30
        $status=Invoke-CleanupMcpCommand 'sketch.cleanup.status' @{}
        if ($watch.Elapsed.TotalSeconds -gt 40) {throw 'MCP cleanup deadline exceeded'}
    } while ($status.state -eq 'running')
    if ($status.state -ne 'ready') {throw ($status|ConvertTo-Json -Compress -Depth 20)}
    return $status
}
try {
    $handshake=Invoke-CleanupMcp 'initialize' @{protocolVersion='2025-11-25';capabilities=@{};clientInfo=@{name='Rainee cleanup acceptance';version='1'}}
    $process.StandardInput.WriteLine('{"jsonrpc":"2.0","method":"notifications/initialized"}');$process.StandardInput.Flush()
    $tools=Invoke-CleanupMcp 'tools/list' @{}
    $commands=Invoke-CleanupMcpTool 'list_commands' @{filter='sketch.cleanup'}
    Invoke-CleanupMcpCommand 'file.new' @{width=240;height=160}|Out-Null
    Invoke-CleanupMcpCommand 'paint.setFill' @{none=$true}|Out-Null
    Invoke-CleanupMcpCommand 'paint.setStroke' @{color='#202020'}|Out-Null
    Invoke-CleanupMcpCommand 'stroke.set' @{weight=2}|Out-Null
    $ids=@();foreach ($path in @('M20 50 L40 46 L60 44 L80 46 L100 50 L120 56','M20 51 L40 47 L60 45 L80 47 L100 51 L120 57','M120 55 L100 49 L80 45 L60 43 L40 45 L20 49')) {
        $ids+=(Invoke-CleanupMcpCommand 'path.create' @{d=$path}).id
    }
    Invoke-CleanupMcpCommand 'sketch.cleanup.start' @{ids=$ids}|Out-Null
    $watch=[Diagnostics.Stopwatch]::StartNew()
    $cancelled=Invoke-CleanupMcpCommand 'sketch.cleanup.cancel' @{}
    if ($cancelled.state -ne 'cancelled') {throw 'Cancellation was not acknowledged'}
    Invoke-CleanupMcpCommand 'sketch.cleanup.start' @{ids=$ids}|Out-Null
    $status=Wait-CleanupPreview
    Invoke-CleanupMcpCommand 'path.create' @{d='M180 20 L190 40'}|Out-Null
    $stale=Invoke-CleanupMcpCommand 'sketch.cleanup.status' @{}
    if ($stale.state -ne 'stale') {throw 'Content edit did not revoke cleanup preview'}
    Invoke-CleanupMcpCommand 'edit.undo' @{}|Out-Null
    Invoke-CleanupMcpCommand 'sketch.cleanup.start' @{ids=$ids}|Out-Null
    $status=Wait-CleanupPreview
    $rejected=Invoke-CleanupMcpCommand 'sketch.cleanup.reject' @{}
    if ($rejected.state -ne 'rejected') {throw 'Rejection was not acknowledged'}
    Invoke-CleanupMcpCommand 'sketch.cleanup.start' @{ids=$ids}|Out-Null
    $status=Wait-CleanupPreview
    $accepted=Invoke-CleanupMcpCommand 'sketch.cleanup.accept' @{job=$status.job}
    Invoke-CleanupMcpCommand 'edit.undo' @{}|Out-Null
    Invoke-CleanupMcpCommand 'edit.redo' @{}|Out-Null
    $inspection=Invoke-CleanupMcpCommand 'document.inspect' @{}
    @{handshake=$handshake;toolCount=@($tools.tools).Count;commands=$commands;accepted=$accepted;document=$inspection;cancelled=$cancelled.state;stale=$stale.state;rejected=$rejected.state;seconds=$watch.Elapsed.TotalSeconds}|ConvertTo-Json -Depth 50|Set-Content -LiteralPath (Join-Path $OutputRoot 'mcp-live-results.json') -Encoding utf8
    Write-Output "Live MCP handshake and reversible native cleanup passed on port $Port"
} finally {
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(5000)) {$process.Kill();$process.WaitForExit()}
    $process.Dispose()
}
