# Runs only against an explicitly selected disposable editor instance. Does not touch driver settings.
[CmdletBinding()]
param([int]$Port=7986,[string]$OutputRoot='D:\DevData\Rainee\verification\editable-cleanup')
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
function Invoke-CleanupControl([string]$Method,$Params) {
    $client=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
    try {
        $client.ReceiveTimeout=60000
        $stream=$client.GetStream()
        $writer=[IO.StreamWriter]::new($stream,[Text.UTF8Encoding]::new($false),4096,$true)
        $reader=[IO.StreamReader]::new($stream,[Text.UTF8Encoding]::new($false),$false,4096,$true)
        $writer.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Compress -Depth 50));$writer.Flush()
        $result=$reader.ReadLine()|ConvertFrom-Json -Depth 100
        if (-not $result.ok) {throw "$Method failed: $($result.error)"}
        return $result.result
    } finally {$client.Dispose()}
}
function Invoke-CleanupCommand([string]$Command,$Params) {Invoke-CleanupControl 'engine.execute' @{command=$Command;params=$Params}}
$inspect=Invoke-CleanupControl 'ui.inspect' @{}
if (@($inspect.documents).Count -gt 0) {throw 'Use a fresh --no-prefs test instance with no open owner documents.'}
$commands=Invoke-CleanupControl 'engine.commands' @{}
foreach ($name in @('start','status','accept','reject','cancel')) {
    if (-not ($commands.id -contains "sketch.cleanup.$name")) {throw "Missing cleanup command $name"}
}
$results=[Collections.Generic.List[object]]::new()
$cases=@(
    @{name='overlap-reversed';expected=1;paths=@('M20 50 L40 46 L60 44 L80 46 L100 50 L120 56','M20 51 L40 47 L60 45 L80 47 L100 51 L120 57','M120 55 L100 49 L80 45 L60 43 L40 45 L20 49')},
    @{name='intentional-gaps';expected=3;paths=@('M20 40 L35 35 L50 30 L65 25 L80 20','M100 40 L115 35 L130 30 L145 25 L160 20','M20 110 L35 105 L50 100 L65 95 L80 90')},
    @{name='hatching';expected=3;paths=@('M20 30 L40 50 L60 70','M20 60 L40 80 L60 100','M20 90 L40 110 L60 130')},
    @{name='sharp-corner';paths=@('M30 100 L80 40 L130 100','M31 100 L81 40 L131 100','M29 100 L79 40 L129 100')},
    @{name='curved-bundle';paths=@('M20 80 C40 20 100 20 130 80','M20 81 C40 21 100 21 130 81','M130 79 C100 19 40 19 20 79')},
    @{name='branched-hair';paths=@('M30 120 C40 60 60 30 90 20','M30 120 C50 75 95 50 130 45','M30 120 C60 100 100 100 150 110')}
)
foreach ($case in $cases) {
    Invoke-CleanupCommand 'file.new' @{width=240;height=160}|Out-Null
    # Use existing style commands, not a private test-only path into the engine.
    Invoke-CleanupCommand 'paint.setFill' @{none=$true}|Out-Null
    Invoke-CleanupCommand 'paint.setStroke' @{color='#202020'}|Out-Null
    Invoke-CleanupCommand 'stroke.set' @{weight=2}|Out-Null
    $ids=@();foreach ($path in $case.paths) {$ids+= (Invoke-CleanupCommand 'path.create' @{d=$path}).id}
    Invoke-CleanupCommand 'sketch.cleanup.start' @{ids=$ids;maxDeviation=4;timeout=30}|Out-Null
    $watch=[Diagnostics.Stopwatch]::StartNew()
    do {
        Start-Sleep -Milliseconds 30
        $status=Invoke-CleanupCommand 'sketch.cleanup.status' @{}
        if ($watch.Elapsed.TotalSeconds -gt 40) {throw 'Editor cleanup polling deadline exceeded'}
    } while ($status.state -eq 'running')
    $count=if ($null -eq $status.proposal) {0} else {@($status.proposal.groups).Count}
    if ($case.ContainsKey('expected') -and ($status.state -ne 'ready' -or $count -ne $case.expected)) {throw "Unexpected result for $($case.name): $($status|ConvertTo-Json -Compress -Depth 30)"}
    $results.Add(@{fixture=$case.name;state=$status.state;groups=$count;seconds=$watch.Elapsed.TotalSeconds;message=$status.message})
    if ($case.name -eq 'overlap-reversed') {
        Invoke-CleanupCommand 'window.sketchCleanup' @{open=$true}|Out-Null
        Invoke-CleanupControl 'ui.screenshot' @{path=(Join-Path $OutputRoot 'preview.png')}|Out-Null
        $accepted=Invoke-CleanupCommand 'sketch.cleanup.accept' @{job=$status.job}
        $fitNode=Invoke-CleanupCommand 'document.node' @{id=$accepted.ids[0]}
        Invoke-CleanupCommand 'edit.undo' @{}|Out-Null
        Invoke-CleanupCommand 'edit.redo' @{}|Out-Null
        $saved=Join-Path $OutputRoot 'accepted.vectorcraft'
        Invoke-CleanupControl 'app.save' @{path=$saved}|Out-Null
        Invoke-CleanupControl 'app.export' @{path=(Join-Path $OutputRoot 'accepted.svg');format='svg'}|Out-Null
        Invoke-CleanupControl 'app.export' @{path=(Join-Path $OutputRoot 'accepted.pdf');format='pdf'}|Out-Null
        $saveWatch=[Diagnostics.Stopwatch]::StartNew()
        do {
            Start-Sleep -Milliseconds 50
            $writing=Invoke-CleanupControl 'ui.inspect' @{}
            if ($saveWatch.Elapsed.TotalSeconds -gt 20) {throw 'Save/export deadline exceeded'}
        } while (@($writing.background).Count -gt 0)
        foreach ($name in @('accepted.vectorcraft','accepted.svg','accepted.pdf')) {
            if (-not (Test-Path -LiteralPath (Join-Path $OutputRoot $name))) {throw "Missing persisted artifact $name"}
        }
        Invoke-CleanupControl 'app.open' @{path=$saved}|Out-Null
        $reopened=Invoke-CleanupCommand 'document.node' @{id=$accepted.ids[0]}
        if (($fitNode|ConvertTo-Json -Depth 50 -Compress) -ne ($reopened|ConvertTo-Json -Depth 50 -Compress)) {throw 'Reopened fitted geometry changed'}
    } else {Invoke-CleanupCommand 'sketch.cleanup.reject' @{}|Out-Null}
}
$results|ConvertTo-Json -Depth 12|Set-Content -LiteralPath (Join-Path $OutputRoot 'artwork-results.json') -Encoding utf8
$results|ForEach-Object {[pscustomobject]$_}|Format-Table fixture,state,groups,seconds,message
