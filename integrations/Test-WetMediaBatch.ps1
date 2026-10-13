[CmdletBinding()]
param([int]$Port=8011,[string]$OutputRoot='D:\DevData\Rainee\verification\wet-media-2026-10-11')
$ErrorActionPreference='Stop'
$resolved=[IO.Path]::GetFullPath($OutputRoot)
if(-not $resolved.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Verification output must remain on D'}
if(Test-Path -LiteralPath $resolved){throw 'Choose a fresh output directory'}
function Send([string]$Method,$Params){
 $c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
 try{$c.ReceiveTimeout=60000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 100 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result}finally{$c.Dispose()}
}
function Cmd([string]$Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
function Wait-Jobs{$watch=[Diagnostics.Stopwatch]::StartNew();do{Start-Sleep -Milliseconds 50;$state=Send 'ui.inspect' @{};if($watch.Elapsed.TotalSeconds -gt 60){throw 'Background job deadline'}}while(@($state.background).Count -gt 0)}
if(@((Send 'ui.inspect' @{}).documents).Count -gt 0){throw 'Use a fresh editor, never owner documents'}
New-Item -ItemType Directory -Path $resolved|Out-Null
Cmd 'file.new' @{name='Editable Wet Media';width=640;height=320}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'stroke.set' @{weight=24}|Out-Null
$ids=@();foreach($i in 0..2){
 Cmd 'select.none' @{}|Out-Null
 Cmd 'paint.setStroke' @{color=@('#df3030','#3048df','#168846')[$i]}|Out-Null
 $y=60+85*$i;$id=(Cmd 'path.create' @{d="M40 $y C180 $($y-40) 380 $($y+40) 570 $y"}).id;$ids+=$id
 Cmd 'effect.apply' @{ids=@($id);item=$null;effect='paint.wetMedia';params=@{spread=12;water=1.5;drying=0.025;paper=0.5;steps=48}}|Out-Null
}
$before=(Cmd 'document.node' @{id=$ids[0]}).kind|ConvertTo-Json -Depth 100 -Compress
Send 'ui.render' @{path=(Join-Path $resolved 'wet.png');scale=1.5}|Out-Null
Cmd 'effect.setParams' @{ids=@($ids[0]);index=0;item=$null;params=@{water=0}}|Out-Null
Send 'ui.render' @{path=(Join-Path $resolved 'dry-bypass.png');scale=1.5}|Out-Null
if((Get-FileHash (Join-Path $resolved 'wet.png')).Hash -eq (Get-FileHash (Join-Path $resolved 'dry-bypass.png')).Hash){throw 'Wet simulation made no visible change'}
Cmd 'edit.undo' @{}|Out-Null
Cmd 'stroke.set' @{ids=@($ids[0]);weight=30}|Out-Null
if($before -ne ((Cmd 'document.node' @{id=$ids[0]}).kind|ConvertTo-Json -Depth 100 -Compress)){throw 'Effect or width edit changed source geometry'}
Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
$path=Join-Path $resolved 'editable-wet-media.vectorcraft'
Send 'app.save' @{path=$path}|Out-Null;Wait-Jobs
foreach($format in @('svg','pdf')){Send 'app.export' @{path=(Join-Path $resolved "editable-wet-media.$format");format=$format}|Out-Null};Wait-Jobs
Send 'app.open' @{path=$path}|Out-Null;Wait-Jobs
if(((Cmd 'document.node' @{id=$ids[0]})|ConvertTo-Json -Depth 100 -Compress) -notmatch 'paint.wetMedia'){throw 'Wet effect missing after reopen'}
Cmd 'file.new' @{name='Grouped Pigment Mixing';width=320;height=240}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'stroke.set' @{weight=36}|Out-Null
Cmd 'paint.setStroke' @{color='#e03030'}|Out-Null;$red=(Cmd 'path.create' @{d='M50 120 L270 120'}).id
Cmd 'select.none' @{}|Out-Null
Cmd 'paint.setStroke' @{color='#3048e0'}|Out-Null;$blue=(Cmd 'path.create' @{d='M160 30 L160 210'}).id
$redColor=(Cmd 'document.node' @{id=$red}).appearance.items[1].paint.color
$blueColor=(Cmd 'document.node' @{id=$blue}).appearance.items[1].paint.color
if($redColor.r -le $redColor.b -or $blueColor.b -le $blueColor.r){throw 'Mixing fixture must contain distinct red and blue pigments'}
Cmd 'select.set' @{ids=@($red,$blue)}|Out-Null;$group=(Cmd 'object.group' @{}).id
Cmd 'effect.apply' @{ids=@($group);item=$null;effect='paint.wetMedia';params=@{spread=12;water=1.5;drying=0.015;paper=0.5;steps=64}}|Out-Null
Send 'ui.render' @{path=(Join-Path $resolved 'grouped-pigment-mixing.png');scale=2}|Out-Null
Send 'app.save' @{path=(Join-Path $resolved 'grouped-pigment-mixing.vectorcraft')}|Out-Null;Wait-Jobs
Cmd 'file.new' @{name='Pressure Tilt Cleanup';width=240;height=160}|Out-Null
Cmd 'paint.setFill' @{none=$true}|Out-Null;Cmd 'stroke.set' @{weight=5}|Out-Null
Cmd 'brush.setCurrent' @{name='Rainee Expressive Pen'}|Out-Null
$targets=@();foreach($offset in 0..2){
 $points=@(@(20,(50+$offset)),@(50,(45+$offset)),@(80,(47+$offset)),@(120,(55+$offset)))
 $samples=@(@{pressure=0.2;tilt=@(20,10)},@{pressure=0.9;tilt=@(35,15)},@{pressure=0.7;tilt=@(-20,25)},@{pressure=0.25;tilt=@(10,15)})
 $targets+=(Cmd 'path.freehand' @{style='brush';points=$points;penSamples=$samples;expressive=@{pressureWidth=$true;tiltNib=$true};fidelity=0.1}).id
}
Cmd 'select.set' @{ids=$targets}|Out-Null
$preflight=Cmd 'sketch.cleanup.inspect' @{};if(-not $preflight.eligible){throw "Pressure/tilt refused: $($preflight.reason)"}
Cmd 'sketch.cleanup.start' @{}|Out-Null;$watch=[Diagnostics.Stopwatch]::StartNew()
do{Start-Sleep -Milliseconds 50;$status=Cmd 'sketch.cleanup.status' @{};if($watch.Elapsed.TotalSeconds -gt 50){throw 'Cleanup deadline'}}while($status.state -eq 'running')
if($status.state -ne 'ready' -or $status.bundleCount -lt 1){throw 'Pressure/tilt cleanup produced no safe bundle'}
$accepted=Cmd 'sketch.cleanup.accept' @{job=$status.job}
foreach($id in $accepted.ids){$node=Cmd 'document.node' @{id=$id};$text=$node|ConvertTo-Json -Depth 100 -Compress;if($text -notmatch '"profile"' -or $text -notmatch '"nib"'){throw 'Cleanup lost pressure/tilt'}}
Cmd 'edit.undo' @{}|Out-Null;Cmd 'edit.redo' @{}|Out-Null
Send 'app.save' @{path=(Join-Path $resolved 'pressure-tilt-cleanup.vectorcraft')}|Out-Null;Wait-Jobs
Send 'ui.render' @{path=(Join-Path $resolved 'pressure-tilt-cleanup.png');scale=2}|Out-Null
@{wetIds=$ids;cleanup=$status;accepted=$accepted;sourcePreserved=$true;limits='Offline diffusion, bounded coarse grid; not full fluid/spectral wet paint'}|ConvertTo-Json -Depth 60|Set-Content -LiteralPath (Join-Path $resolved 'results.json') -Encoding utf8
Write-Output "Wet media and pressure/tilt native checks passed: $resolved"
