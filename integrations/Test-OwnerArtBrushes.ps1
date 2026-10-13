[CmdletBinding()]
param([int]$Port=8003,[string]$ShapeRoot='D:/DevData/Rainee/research/owner-brush-shapes/Sketch-File',[string]$OutputRoot='D:/DevData/Rainee/verification/owner-art-brushes-2026-10-11')
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
function Send([string]$Method,$Params){$c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port);try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 90 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result}finally{$c.Dispose()}}
function Cmd([string]$Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
$definitions=@()
foreach($index in 1..9){
 $svg=Join-Path $ShapeRoot ('brush-{0:00}.svg' -f $index)
 Send 'app.open' @{path=$svg} | Out-Null
 $watch=[Diagnostics.Stopwatch]::StartNew()
 do { $doc=Send 'document.inspect' @{};if($watch.Elapsed.TotalSeconds -gt 30){throw 'Open deadline'};Start-Sleep -Milliseconds 30 } while($doc.title -ne [IO.Path]::GetFileName($svg))
 Cmd 'select.all' @{} | Out-Null
 $name='Owner Art Brush {0:00}' -f $index
 Cmd 'brush.new' @{type='art';name=$name;params=@{scale=@{mode='stretch'};colorization=@{method='tints'}}} | Out-Null
 $definitions+=Cmd 'brush.get' @{name=$name}
 Send 'ui.render' @{path=(Join-Path $OutputRoot ('source-{0:00}.png' -f $index));scale=4} | Out-Null
}
Cmd 'file.new' @{name='Owner Vector Art Brush Candidates';width=800;height=1000} | Out-Null
Cmd 'paint.setFill' @{none=$true} | Out-Null
Cmd 'paint.setStroke' @{color='#202020'} | Out-Null
Cmd 'stroke.set' @{weight=1} | Out-Null
$ids=@()
foreach($index in 1..9){
 $definition=$definitions[$index-1]
 Cmd 'brush.new' @{type='art';name=$definition.name;params=$definition} | Out-Null
 $y=50+100*($index-1)
 $id=(Cmd 'path.create' @{d="M40 $y C180 $($y-35) 270 $($y+35) 400 $y C500 $($y-35) 660 $($y+35) 760 $y"}).id
 Cmd 'brush.apply' @{name=$definition.name;ids=@($id)} | Out-Null
 $ids+=$id
}
Send 'ui.render' @{path=(Join-Path $OutputRoot 'owner-art-candidates.png');scale=2} | Out-Null
Cmd 'stroke.set' @{ids=@($ids[4]);weight=2} | Out-Null
Cmd 'edit.undo' @{} | Out-Null
Cmd 'edit.redo' @{} | Out-Null
Send 'app.save' @{path=(Join-Path $OutputRoot 'owner-art-brush-library.vectorcraft')} | Out-Null
foreach($format in @('svg','pdf')){Send 'app.export' @{path=(Join-Path $OutputRoot "owner-art-brush-library.$format");format=$format}|Out-Null}
@{candidateCount=$definitions.Count;ids=$ids;source='local owner file silhouettes';claim='geometry candidates, not recovered dynamic settings or owner-approved matching'} | ConvertTo-Json -Depth 5
