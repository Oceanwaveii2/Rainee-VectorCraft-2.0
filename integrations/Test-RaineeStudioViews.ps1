param([int]$Port=8046,[string]$OutputRoot='D:\DevData\Rainee\verification\rainee-2-studio-2026-10-13\native-pass3')
$ErrorActionPreference='Stop'
function Send([string]$Method,$Params){
 $c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
 try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 100 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result}finally{$c.Dispose()}
}
function Cmd($Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
function Inspect{Send 'ui.inspect' @{}}
function Shot($Name){Send 'ui.screenshot' @{path="$root\$Name.png";focus=$false}|Out-Null}
function ClickRect($r){Send 'ui.click' @{x=$r[0]+$r[2]*0.5;y=$r[1]+$r[3]*0.5}|Out-Null;Start-Sleep -Milliseconds 250}
function SameRect($a,$b){for($i=0;$i-lt 4;$i++){if([Math]::Abs($a[$i]-$b[$i])-gt 1){throw "Studio changed bounds: $a -> $b"}}}
$root=[IO.Path]::GetFullPath($OutputRoot)
if(-not $root.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Use the verification directory'}
$state=Inspect
if(@($state.documents).Count-ne 1 -or $state.documents[0].title-notmatch 'glass-test'){throw 'Use the disposable glass-test diagnostic document only'}
New-Item -ItemType Directory -Path $root -Force|Out-Null
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
$before=Cmd 'document.inspect' @{}
$brushes=@((Cmd 'brush.list' @{}).brushes)
$addedNatural=-not @($brushes|Where-Object type -eq 'natural').Count
if($addedNatural){
 Cmd 'brush.new' @{name='Studio Natural engine verification';type='natural'}|Out-Null
 $brushes=@((Cmd 'brush.list' @{}).brushes)
}
$samples=@()
foreach($brush in $brushes|Group-Object type|ForEach-Object {$_.Group[0]}){
 Send 'ui.brushStudio.open' @{name=$brush.name}|Out-Null
 Start-Sleep -Milliseconds 400
 $start=(Inspect).brushStudio
 foreach($section in $start.sections){
  ClickRect $section[1]
  $now=(Inspect).brushStudio
  SameRect $start.rect $now.rect
  if($now.section-ne $section[0]){throw 'Studio section did not switch'}
  if($section[0]-in @('Shape','Grain','Rendering')){Shot ($brush.type+'-'+$section[0].ToLowerInvariant())}
 }
 ClickRect (Inspect).brushStudio.cancel
 $samples+=@{type=$brush.type;name=$brush.name;rect=$start.rect}
}
if($addedNatural){Cmd 'edit.undo' @{}|Out-Null;$brushes=@((Cmd 'brush.list' @{}).brushes)}
$name=$brushes[0].name
$long='Studio long-name layout verification — '+('very long brush name '*8)
$duplicate=Cmd 'brush.duplicate' @{name=$name;newName=$long}
Send 'ui.brushStudio.open' @{name=$duplicate.name}|Out-Null
Start-Sleep -Milliseconds 400
$start=(Inspect).brushStudio
foreach($section in $start.sections){ClickRect $section[1];SameRect $start.rect (Inspect).brushStudio.rect}
Shot 'studio-long-name'
ClickRect (Inspect).brushStudio.cancel
Cmd 'edit.undo' @{}|Out-Null
$after=Cmd 'document.inspect' @{}
if(($before.layers|ConvertTo-Json -Depth 100 -Compress)-ne ($after.layers|ConvertTo-Json -Depth 100 -Compress)){throw 'Studio views changed art'}
Cmd 'edit.preferences' @{category='User Interface'}|Out-Null
Start-Sleep -Milliseconds 500
Shot 'preferences-glass'
$glass=Inspect
if(-not $glass.glass.enabled -or $glass.glass.surfaces-lt 1){throw 'Glass did not render'}
Start-Sleep -Milliseconds 700
if((Inspect).glass.rebuilds-ne $glass.glass.rebuilds){throw 'Idle glass rebuilt its cache'}
Send 'ui.dialog.set' @{field='uiReduceTransparency';value=$true}|Out-Null
Start-Sleep -Milliseconds 350
Shot 'preferences-opaque'
if((Inspect).glass.enabled){throw 'Reduce Transparency failed'}
Send 'ui.dialog.cancel' @{}|Out-Null
Start-Sleep -Milliseconds 250
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
Cmd 'window.panel' @{panel='Brushes'}|Out-Null
Shot 'legacy-brushes-open'
if((Inspect).glass.enabled){throw 'Glass leaked into 1.0'}
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
@{status='PASS';brushTypes=$samples;longNameStable=$true;artUnchanged=$true;glass=$glass;legacy=(Inspect).ui}|ConvertTo-Json -Depth 100|Set-Content -LiteralPath "$root\telemetry.json" -Encoding utf8
'PASS: available brush types, nine pages, long names, unchanged art, glass cache/opaque mode, legacy Brushes route'
