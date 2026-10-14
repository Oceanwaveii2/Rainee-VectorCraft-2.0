param(
 [int]$Port=8046,
 [string]$OutputRoot='D:\DevData\Rainee\verification\rainee-2-studio-2026-10-13\native-pass2',
 [string]$Fixture='D:\DevData\Rainee\verification\rainee-theme-2026-10-13\glass-test.svg'
)
$ErrorActionPreference='Stop'
function Send([string]$Method,$Params){
 $c=[Net.Sockets.TcpClient]::new('127.0.0.1',$Port)
 try{$c.ReceiveTimeout=30000;$s=$c.GetStream();$w=[IO.StreamWriter]::new($s,[Text.UTF8Encoding]::new($false),4096,$true);$r=[IO.StreamReader]::new($s,[Text.UTF8Encoding]::new($false),$false,4096,$true);$w.WriteLine((@{method=$Method;params=$Params}|ConvertTo-Json -Depth 100 -Compress));$w.Flush();$v=$r.ReadLine()|ConvertFrom-Json -Depth 100;if(-not $v.ok){throw "$Method`: $($v.error)"};$v.result}finally{$c.Dispose()}
}
function Cmd($Name,$Params){Send 'engine.execute' @{command=$Name;params=$Params}}
function Inspect{Send 'ui.inspect' @{}}
function Idle{for($i=0;$i-lt 30;$i++){$s=Inspect;if($s.syntheticInputPending-eq 0 -and -not $s.pointerPrimaryDown){return};Start-Sleep -Milliseconds 100};throw 'Synthetic input did not settle'}
function ClickRect($Rect){if(@($Rect).Count-ne 4){throw 'Missing widget rectangle'};Idle;Send 'ui.click' @{x=$Rect[0]+$Rect[2]*0.5;y=$Rect[1]+$Rect[3]*0.5}|Out-Null;Start-Sleep -Milliseconds 150;Idle;Start-Sleep -Milliseconds 100}
function Shot($Name){Send 'ui.screenshot' @{path="$root\$Name.png";focus=$false}|Out-Null}
function SameRect($a,$b,$label){for($i=0;$i-lt 4;$i++){if([Math]::Abs($a[$i]-$b[$i])-gt 1){throw "$label moved/resized: $a -> $b"}}}
$root=[IO.Path]::GetFullPath($OutputRoot)
if(-not $root.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Use the verification directory'}
New-Item -ItemType Directory -Path $root -Force|Out-Null
if(@((Inspect).documents).Count){throw 'Use a fresh diagnostic process with no owner documents'}
Send 'ui.key' @{key='Escape'}|Out-Null
Send 'app.open' @{path=$Fixture}|Out-Null
Start-Sleep -Seconds 2
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
Cmd 'select.all' @{}|Out-Null
Shot 'workspace-selected'
Cmd 'select.none' @{}|Out-Null
$before=Cmd 'document.inspect' @{}
$name=@((Cmd 'brush.list' @{}).brushes|Where-Object type -eq 'calligraphic')[0].name
$original=Cmd 'brush.get' @{name=$name}
Send 'ui.brushStudio.open' @{name=$name}|Out-Null
Start-Sleep -Milliseconds 500
$start=Inspect
if(-not $start.brushStudio.open -or -not $start.brushStudio.rect){throw 'Studio failed to open'}
Shot 'studio-shape'
$samples=@()
foreach($section in $start.brushStudio.sections){
 $label=$section[0];$r=$section[1]
 ClickRect $r
 $now=Inspect
 if($now.brushStudio.section-ne $label){throw "Section did not open: $label"}
 SameRect $start.brushStudio.rect $now.brushStudio.rect $label
 $x=$r[0]+$r[2]*0.5;$y=$r[1]+$r[3]*0.5
 Send 'ui.touch' @{phase='start';x=$x;y=$y;force=0.6}|Out-Null
 Send 'ui.touch' @{phase='move';x=$x+1.5;y=$y+1;force=0.6}|Out-Null
 Send 'ui.touch' @{phase='end';x=$x+1.5;y=$y+1;force=0.6}|Out-Null
 Start-Sleep -Milliseconds 250
 $now=Inspect
 if(-not $now.touchInputDetected){throw 'Touch path was not exercised'}
 SameRect $start.brushStudio.rect $now.brushStudio.rect "Tablet subsection $label"
 SameRect @($start.window[0],$start.window[1],0,0) @($now.window[0],$now.window[1],0,0) 'Desktop viewport'
 if($now.brushStudio.section-ne $label){throw "Tablet tap lost subsection $label"}
 $samples+=@{section=$label;rect=$now.brushStudio.rect;touchDetected=$now.touchInputDetected}
 if($label-in @('Rendering','About this brush')){Shot ('studio-'+$label.Replace(' ','-').ToLowerInvariant())}
}
Start-Sleep -Seconds 1
SameRect $start.brushStudio.rect (Inspect).brushStudio.rect 'Idle Studio'
$s=(Inspect).brushStudio
$p=$s.pad
Send 'ui.drag' @{x=$p[0]+$p[2]*0.15;y=$p[1]+$p[3]*0.35;toX=$p[0]+$p[2]*0.75;toY=$p[1]+$p[3]*0.65;steps=8}|Out-Null
Start-Sleep -Milliseconds 500
if((Inspect).brushStudio.previewStrokes-lt 1){throw 'Preview pad did not draw'}
Shot 'studio-pad'
if(($before.layers|ConvertTo-Json -Depth 100 -Compress)-ne ((Cmd 'document.inspect' @{}).layers|ConvertTo-Json -Depth 100 -Compress)){throw 'Studio preview changed document art'}
ClickRect (Inspect).brushStudio.undoPreview
if((Inspect).brushStudio.previewStrokes-ne 0){throw 'Preview Undo failed'}
$s=(Inspect).brushStudio;$h=$s.header
Send 'ui.drag' @{x=$h[0]+80;y=$h[1]+20;toX=$h[0]+108;toY=$h[1]+36;steps=8}|Out-Null
Start-Sleep -Milliseconds 400
Idle
$moved=(Inspect).brushStudio.rect
if([Math]::Abs($moved[0]-$s.rect[0])+[Math]::Abs($moved[1]-$s.rect[1])-lt 8){throw 'Deliberate header drag failed'}
$s=(Inspect).brushStudio;$r=$s.rect
Send 'ui.drag' @{x=$r[0]+$r[2]-5;y=$r[1]+$r[3]-5;toX=$r[0]+$r[2]-85;toY=$r[1]+$r[3]-55;steps=12}|Out-Null
Start-Sleep -Milliseconds 400
Idle
$resized=(Inspect).brushStudio.rect
if([Math]::Abs($resized[2]-$r[2])-lt 20){throw 'Intentional corner resize failed'}
$pen=@((Inspect).brushStudio.sections|Where-Object {$_[0]-eq 'Pen input'})[0][1]
ClickRect $pen
SameRect $resized (Inspect).brushStudio.rect 'Section after deliberate resize'
Shot 'studio-manually-resized'
$shape=@((Inspect).brushStudio.sections|Where-Object {$_[0]-eq 'Shape'})[0][1]
ClickRect $shape
if((Inspect).brushStudio.section-ne 'Shape'){throw 'Shape did not settle after resize'}
$size=(Inspect).brushStudio.controls.size
Send 'ui.click' @{x=$size[0]+$size[2]*0.12;y=$size[1]+$size[3]*0.5}|Out-Null
Start-Sleep -Milliseconds 250
ClickRect (Inspect).brushStudio.done
if((Inspect).brushStudio.open){throw 'Done did not close Studio'}
$edited=Cmd 'brush.get' @{name=$name}
if($edited.size-eq $original.size){throw 'Brush size edit did not apply'}
Cmd 'edit.undo' @{}|Out-Null
if((Cmd 'brush.get' @{name=$name}|ConvertTo-Json -Depth 100 -Compress)-ne ($original|ConvertTo-Json -Depth 100 -Compress)){throw 'Undo did not restore the brush'}
Cmd 'window.workspace' @{name='Rainee 1.0'}|Out-Null
Cmd 'select.all' @{}|Out-Null
Shot 'workspace-1-retained'
Cmd 'window.workspace' @{name='Rainee 2.0'}|Out-Null
Cmd 'select.none' @{}|Out-Null
@{status='PASS';subsections=$samples;padIsolated=$true;headerDrag=$true;intentionalResize=$true;brushEditUndo=$true;initial=$start;final=(Inspect)}|ConvertTo-Json -Depth 100|Set-Content -LiteralPath "$root\telemetry.json" -Encoding utf8
'PASS: native subsection mouse/tablet stability, idle bounds, pad isolation/undo, header drag, intentional resize, brush edit/undo, both workspaces'
