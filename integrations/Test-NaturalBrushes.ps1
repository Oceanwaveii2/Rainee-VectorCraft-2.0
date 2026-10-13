[CmdletBinding()]
param([int]$Port=7995,[string]$OutputRoot='D:\DevData\Rainee\verification\codex-editable-natural-2026-10-11',[string]$PencilBrush='Rainee Pencil',[string]$InkBrush='Rainee Dry Ink')
$ErrorActionPreference='Stop'
New-Item -ItemType Directory -Force -Path $OutputRoot|Out-Null
function Send([string]$Method,$Params){$c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port);try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 60 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result}finally{$c.Dispose()}}
function Cmd([string]$Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
Cmd 'file.new' @{name='Editable Natural Brush Trial';width=360;height=260}|Out-Null
Cmd 'window.codexEditor' @{open=$false}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'paint.setStroke' @{color='#202020'}|Out-Null;Cmd 'stroke.set' @{weight=5}|Out-Null
Cmd 'tool.select' @{tool='paintbrush'}|Out-Null;Cmd 'tool.setOption' @{tool='paintbrush';values=@{pressureWidth=$true;tiltNib=$true}}|Out-Null
$ids=@();foreach($test in @(@{name=$PencilBrush;y=75},@{name=$InkBrush;y=180})){
 Cmd 'select.set' @{ids=@()}|Out-Null
 Cmd 'brush.setCurrent' @{name=$test.name}|Out-Null
 Cmd 'tool.gesture' @{events=@(@{kind='down';x=40;y=$test.y;pressure=0.2;tilt=@(10,15)},@{kind='drag';x=160;y=($test.y-35);pressure=0.9;tilt=@(25,35)},@{kind='up';x=280;y=$test.y;pressure=0.4;tilt=@(10,10)})}|Out-Null
 $doc=Send 'document.inspect' @{};$id=@($doc.selection)[0];if($null -eq $id){throw 'No committed natural stroke'};$ids+=$id
 $node=Cmd 'document.node' @{id=$id};if(($node|ConvertTo-Json -Depth 60 -Compress) -notmatch [regex]::Escape($test.name)){throw "Selected natural brush was replaced: $($test.name)"}
}
Send 'ui.render' @{path=(Join-Path $OutputRoot 'natural-before-width.png');scale=3}|Out-Null
$pathBefore=(Cmd 'document.node' @{id=$ids[0]}).kind|ConvertTo-Json -Depth 60 -Compress
Cmd 'stroke.set' @{ids=@($ids[0]);weight=9}|Out-Null
if($pathBefore -ne ((Cmd 'document.node' @{id=$ids[0]}).kind|ConvertTo-Json -Depth 60 -Compress)){throw 'Width edit changed centerline'}
Send 'ui.render' @{path=(Join-Path $OutputRoot 'natural-after-width.png');scale=3}|Out-Null
if((Get-FileHash (Join-Path $OutputRoot 'natural-before-width.png')).Hash -eq (Get-FileHash (Join-Path $OutputRoot 'natural-after-width.png')).Hash){throw 'Width edit did not regenerate appearance'}
Cmd 'select.set' @{ids=@($ids[1])}|Out-Null
$erased=Cmd 'path.eraseRegion' @{points=@(@(160,110),@(160,220));size=14}
$erasedIds=if($erased.ids){@($erased.ids)}else{@($erased)}
if($erasedIds.Count -lt 2){throw 'Eraser did not split centerline'}
foreach($id in $erasedIds){if(((Cmd 'document.node' @{id=$id})|ConvertTo-Json -Depth 60 -Compress) -notmatch [regex]::Escape($InkBrush)){throw 'Eraser discarded brush identity'}}
Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
$save=Join-Path $OutputRoot 'natural-editable.vectorcraft';Send 'app.save' @{path=$save}|Out-Null
$watch=[Diagnostics.Stopwatch]::StartNew();while((Send 'document.inspect' @{}).dirty){if($watch.Elapsed.TotalSeconds -gt 30){throw 'Save deadline'};Start-Sleep -Milliseconds 50}
Send 'app.open' @{path=$save}|Out-Null
foreach($format in @('svg','pdf')){Send 'app.export' @{path=(Join-Path $OutputRoot "natural-editable.$format");format=$format}|Out-Null}
Send 'ui.render' @{path=(Join-Path $OutputRoot 'natural-erased.png');scale=3}|Out-Null
@{sourceIds=$ids;erasedIds=$erasedIds;checks=@('current natural brush retained with pressure/tilt','width appearance regeneration','centerline preserved','eraser split brush paths','undo/redo','native save/reopen','SVG/PDF export');hardware='synthetic gestures, not Cintiq acceptance'}|ConvertTo-Json -Depth 30|Set-Content -LiteralPath (Join-Path $OutputRoot 'natural-results.json') -Encoding utf8
'Native natural brush checks completed'
