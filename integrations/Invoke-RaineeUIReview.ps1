param([int]$Port=8039,[string]$Method='ui.inspect',[string]$ParamsJson='{}')
$ErrorActionPreference='Stop'
$client=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
try {
    $client.ReceiveTimeout=30000
    $stream=$client.GetStream()
    $writer=[IO.StreamWriter]::new($stream,[Text.UTF8Encoding]::new($false),4096,$true)
    $reader=[IO.StreamReader]::new($stream,[Text.UTF8Encoding]::new($false),$false,4096,$true)
    $writer.WriteLine((@{method=$Method;params=($ParamsJson|ConvertFrom-Json)}|ConvertTo-Json -Depth 100 -Compress))
    $writer.Flush()
    $reply=$reader.ReadLine()|ConvertFrom-Json
    if(-not $reply.ok){throw "$Method`: $($reply.error)"}
    $reply.result|ConvertTo-Json -Depth 100
} finally { $client.Dispose() }
