[CmdletBinding()]
param([int]$Port=8008,[string]$OutputRoot='D:\DevData\Rainee\verification\brush-ink-batch-2026-10-11')
$ErrorActionPreference='Stop'
if(Test-Path -LiteralPath $OutputRoot){throw 'Use a fresh output directory; previous brush results are preserved.'}
New-Item -ItemType Directory -Path $OutputRoot|Out-Null
function Send([string]$Method,$Params){$c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port);try{$c.ReceiveTimeout=60000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 100 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result}finally{$c.Dispose()}}
function Cmd([string]$Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
function Wait-Jobs { $t=[Diagnostics.Stopwatch]::StartNew(); do{Start-Sleep -Milliseconds 50;$s=Send 'ui.inspect' @{};if($t.Elapsed.TotalSeconds -gt 60){throw 'Background job deadline'}}while(@($s.background).Count -gt 0) }
function Save-Outputs([string]$Name){Send 'app.save' @{path=(Join-Path $OutputRoot "$Name.vectorcraft")}|Out-Null;Wait-Jobs;foreach($f in @('svg','pdf')){Send 'app.export' @{path=(Join-Path $OutputRoot "$Name.$f");format=$f}|Out-Null};Wait-Jobs;Send 'ui.render' @{path=(Join-Path $OutputRoot "$Name.png");scale=1.5}|Out-Null}
if(@((Send 'ui.inspect' @{}).documents).Count -gt 0){throw 'Use a fresh disposable editor instance, never owner documents.'}
Send 'app.open' @{path='D:\DevData\Rainee\verification\owner-art-brushes-2026-10-11\owner-art-brush-library.vectorcraft'}|Out-Null
Wait-Jobs
$definitions=@();foreach($i in 1..9){$definitions+=Cmd 'brush.get' @{name=('Owner Art Brush {0:00}' -f $i)}}
Cmd 'file.new' @{name='Owner Art Brush Curve Tilt Matrix';width=1000;height=1100}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'paint.setStroke' @{color='#202020'}|Out-Null;Cmd 'stroke.set' @{weight=0.6}|Out-Null
$sourceIds=@();$cases=@('straight','s-curve','corner','reversal')
foreach($i in 0..8){
 $def=$definitions[$i];Cmd 'brush.new' @{type='art';name=$def.name;params=$def}|Out-Null
 Cmd 'brush.setCurrent' @{name=$def.name}|Out-Null
 foreach($case in 0..3){
  $x=25+240*$case;$y=65+115*$i
  $points=switch($case){0{@(@($x,$y),@(($x+70),$y),@(($x+140),$y),@(($x+200),$y))};1{@(@($x,$y),@(($x+60),($y-25)),@(($x+140),($y+25)),@(($x+200),$y))};2{@(@($x,$y),@(($x+120),$y),@(($x+100),($y-40)),@(($x+200),($y-40)))};3{@(@($x,$y),@(($x+180),($y-20)),@(($x+70),($y+20)),@(($x+200),$y))}}
  $samples=@();for($j=0;$j -lt $points.Count;$j++){$samples+=@{pressure=if($j -eq 0 -or $j -eq $points.Count-1){0.25}else{0.9};tilt=@(35,15)}}
  $id=(Cmd 'path.freehand' @{style='brush';points=$points;penSamples=$samples;expressive=@{pressureWidth=$true;tiltNib=$true};fidelity=0.25}).id
  $n=Cmd 'document.node' @{id=$id};if(($n|ConvertTo-Json -Depth 100 -Compress) -notmatch [regex]::Escape($def.name)){throw 'Art brush identity lost'}
  $sourceIds+=$id
 }
}
Save-Outputs 'owner-art-curve-tilt-matrix'
$id=$sourceIds[1];$before=Cmd 'document.node' @{id=$id};$kind=$before.kind|ConvertTo-Json -Depth 100 -Compress
Cmd 'stroke.set' @{ids=@($id);weight=1.2}|Out-Null
if($kind -ne ((Cmd 'document.node' @{id=$id}).kind|ConvertTo-Json -Depth 100 -Compress)){throw 'Width edit changed source geometry'}
Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
Cmd 'file.new' @{name='Editable Ink Controls';width=900;height=500}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'paint.setStroke' @{color='#202020'}|Out-Null;Cmd 'stroke.set' @{weight=3}|Out-Null
$styles=@(@{name='Neutral Dry Ink';inkLoad=1.0;edgePooling=0;breakup=0;paperScale=1},@{name='Pooled Ink';inkLoad=1.4;edgePooling=1;breakup=0;paperScale=1},@{name='Broken Bristles';inkLoad=1.2;edgePooling=0;breakup=0.75;paperScale=1},@{name='Coarse Paper';inkLoad=1;edgePooling=0;breakup=0;paperScale=4})
$inkIds=@();foreach($i in 0..3){$style=$styles[$i];$params=@{media='dryInk';width=6;fibers=16;grain=0.5;seed=20261011;inkLoad=$style.inkLoad;edgePooling=$style.edgePooling;breakup=$style.breakup;paperScale=$style.paperScale};Cmd 'brush.new' @{type='natural';name=$style.name;params=$params}|Out-Null;$y=60+110*$i;$id=(Cmd 'path.create' @{d="M40 $y C240 $($y-80) 480 $($y+80) 800 $y"}).id;Cmd 'brush.apply' @{ids=@($id);name=$style.name}|Out-Null;$inkIds+=$id}
Save-Outputs 'editable-ink-controls'
$source=(Cmd 'document.node' @{id=$inkIds[1]}).kind|ConvertTo-Json -Depth 100 -Compress
Cmd 'brush.options' @{name='Pooled Ink';params=@{inkLoad=1.8;edgePooling=0.5;breakup=0.2;paperScale=2}}|Out-Null
if($source -ne ((Cmd 'document.node' @{id=$inkIds[1]}).kind|ConvertTo-Json -Depth 100 -Compress)){throw 'Ink control changed source path'}
Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
Cmd 'select.set' @{ids=@($inkIds[0])}|Out-Null
$pieces=Cmd 'path.eraseRegion' @{points=@(@(420,0),@(420,500));size=14;eraseAll=$true}
$pieceIds=if($null -ne $pieces.ids){@($pieces.ids)}else{@($pieces)}
if($pieceIds.Count -lt 2){throw 'Ink eraser failed to split paths'}
foreach($id in $pieceIds){if(-not (Cmd 'document.node' @{id=$id}).kind){throw 'Erased source missing'}}
Save-Outputs 'editable-ink-erased'
Cmd 'file.new' @{name='Cleanup Natural Brush Trial';width=240;height=160}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'stroke.set' @{weight=2}|Out-Null
$ids=@();foreach($d in @('M20 50 L40 46 L60 44 L80 46 L100 50 L120 56','M20 51 L40 47 L60 45 L80 47 L100 51 L120 57','M120 55 L100 49 L80 45 L60 43 L40 45 L20 49')){$ids+=(Cmd 'path.create' @{d=$d}).id}
Cmd 'brush.apply' @{name='Rainee Dry Ink';ids=$ids}|Out-Null;Cmd 'select.set' @{ids=$ids}|Out-Null
if(-not (Cmd 'sketch.cleanup.inspect' @{}).eligible){throw 'Uniform natural strokes refused'}
Cmd 'sketch.cleanup.start' @{}|Out-Null;$watch=[Diagnostics.Stopwatch]::StartNew();do{Start-Sleep -Milliseconds 50;$status=Cmd 'sketch.cleanup.status' @{};if($watch.Elapsed.TotalSeconds -gt 45){throw 'Cleanup deadline'}}while($status.state -eq 'running')
if($status.state -ne 'ready' -or $status.proposal.groups.Count -ne 1){throw 'Natural centerline bundle not consolidated'}
$accepted=Cmd 'sketch.cleanup.accept' @{job=$status.job}
foreach($id in $accepted.ids){if(((Cmd 'document.node' @{id=$id})|ConvertTo-Json -Depth 100 -Compress) -notmatch 'Rainee Dry Ink'){throw 'Cleanup lost natural brush'}}
Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
Save-Outputs 'cleanup-natural-brush'
@{ownerBrushes=9;curveCases=$cases;sourceIds=$sourceIds;inkIds=$inkIds;cleanup=$status;claims='Functional matrix and procedural controls, not an independent original-application oracle or wet-fluid simulation'}|ConvertTo-Json -Depth 50|Set-Content -LiteralPath (Join-Path $OutputRoot 'results.json') -Encoding utf8
Write-Output "Brush, ink, cleanup and persistence checks passed: $OutputRoot"
