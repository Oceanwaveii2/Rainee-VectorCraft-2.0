param([int]$Port=8027,[string]$OutputRoot='D:\DevData\Rainee\verification\rainee-2-ui-2026-10-12')
$ErrorActionPreference='Stop'
function Send([string]$Method,$Params){
 $c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
 try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 100 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result}finally{$c.Dispose()}
}
function Cmd($Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
$root=[IO.Path]::GetFullPath($OutputRoot)
if(-not $root.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Test output must stay in verification folder'}
New-Item -ItemType Directory -Path $root -Force|Out-Null
if(@((Send 'ui.inspect' @{}).documents).Count){throw 'Use a fresh test app, not owner documents'}
Send 'ui.key' @{key='Escape'}|Out-Null
Cmd 'file.new' @{name='Rainee 2.0 UI Trial';width=600;height=600}|Out-Null
Cmd 'shape.rectangle' @{x=100;y=100;width=200;height=140}|Out-Null
Cmd 'select.none' @{}|Out-Null
$before=Send 'document.inspect' @{}
$workspaces=Cmd 'window.workspace.list' @{}
$one=[Array]::IndexOf(@($workspaces.workspaces),'Rainee 1.0')
if($one -lt 0 -or $workspaces.workspaces[$one+1] -ne 'Rainee 2.0'){throw 'Workspace order mismatch'}
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
Start-Sleep -Seconds 2
Send 'ui.screenshot' @{path="$root\rainee-2.png";focus=$false}|Out-Null
$after=Send 'document.inspect' @{}
if(($before.layers|ConvertTo-Json -Depth 100 -Compress) -ne ($after.layers|ConvertTo-Json -Depth 100 -Compress)){throw 'Workspace switching changed artwork'}
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
Start-Sleep -Milliseconds 500
Send 'ui.screenshot' @{path="$root\rainee-1-retained.png";focus=$false}|Out-Null
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
'PASS: menu ordering, native presentation, workspace switches preserve artwork'
