[CmdletBinding()]
param([int]$TimeoutSeconds=90)
$ErrorActionPreference='Stop'
if($TimeoutSeconds -lt 1 -or $TimeoutSeconds -gt 300){throw 'Timeout must be 1..300 seconds.'}
. (Join-Path $PSScriptRoot '..\storage\Use-DDrive.ps1') -Target Editor
$build='D:\DevData\Rainee\build\stripmaker-scip'
$sdk='D:\DevData\Rainee\tools\scip-10.1.0\scipoptsuite-10.1.0-win-x64'
$runtime="$sdk\bin;$build\msvc_19.41_cxx20_64_md_release"
$output=Join-Path $build ('checks-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $output | Out-Null
function Read-Capture([string]$Path) {
  $text=Get-Content -LiteralPath $Path -Raw
  $strokes=@([regex]::Matches($text,'(?s)\{(.*?)\}') | ForEach-Object {
    $block=$_.Groups[1].Value
    $header=[regex]::Match($block,'#(\d+)\t(\d+)')
    if(-not $header.Success){throw 'Missing stroke identity.'}
    $points=@([regex]::Matches($block,'(?m)^\s*([-+\d.eE]+)\t([-+\d.eE]+)\t([-+\d.eE]+)\s*$') | ForEach-Object {
      $sample=$_
      $values=@(1..3 | ForEach-Object { [double]::Parse($sample.Groups[$_].Value,[Globalization.CultureInfo]::InvariantCulture) })
      ,$values
    })
    if($points.Count -lt 2){throw 'Stroke has no usable geometry.'}
    foreach($point in $points){foreach($v in $point){if(-not[double]::IsFinite($v)){throw 'Nonfinite stroke sample.'}}}
    [pscustomobject]@{Id=[int]$header.Groups[1].Value;Group=[int]$header.Groups[2].Value;Points=$points}
  })
  return ,$strokes
}
function Invoke-CheckedProcess([string]$Executable,[string[]]$Arguments,[string]$Label) {
  $info=[Diagnostics.ProcessStartInfo]::new()
  $info.FileName=$Executable
  $info.WorkingDirectory=$output
  $info.UseShellExecute=$false
  $info.CreateNoWindow=$true
  $info.RedirectStandardOutput=$true
  $info.RedirectStandardError=$true
  $info.Environment['PATH']="$runtime;$env:PATH"
  foreach($arg in $Arguments){$info.ArgumentList.Add($arg)}
  $process=[Diagnostics.Process]::new()
  $process.StartInfo=$info
  $timer=[Diagnostics.Stopwatch]::StartNew()
  try {
    if(-not $process.Start()){throw 'Process launch failed.'}
    $stdout=$process.StandardOutput.ReadToEndAsync()
    $stderr=$process.StandardError.ReadToEndAsync()
    if(-not $process.WaitForExit($TimeoutSeconds*1000)){
      $process.Kill($true); $process.WaitForExit()
      throw "$Label exceeded $TimeoutSeconds seconds; child process tree stopped."
    }
    $out=$stdout.GetAwaiter().GetResult()
    $err=$stderr.GetAwaiter().GetResult()
    $out | Set-Content -LiteralPath (Join-Path $output "$Label.stdout.log") -Encoding utf8
    $err | Set-Content -LiteralPath (Join-Path $output "$Label.stderr.log") -Encoding utf8
    if($process.ExitCode -ne 0){throw "$Label failed ($($process.ExitCode)). Logs: $output"}
    [pscustomobject]@{Check=$Label;Seconds=$timer.Elapsed.TotalSeconds;ExitCode=$process.ExitCode;Output=$out.Trim()}
  } finally {$process.Dispose()}
}
$records=@()
$records+=Invoke-CheckedProcess "$build\Release\scip-adapter-tests.exe" @() 'adapter'
$fixture=Join-Path $PSScriptRoot 'scip\fixtures\bundle.scap'
$fitFolder=Join-Path $output 'fit'
New-Item -ItemType Directory -Path $fitFolder | Out-Null
$records+=Invoke-CheckedProcess "$build\author\Release\strokestrip.exe" @('-i',$fixture,'-o',$fitFolder) 'bundle-fit'
$svg=Join-Path $fitFolder 'bundle_fit.svg'
[xml]$image=Get-Content -LiteralPath $svg -Raw
$lines=$image.SelectNodes('//*[local-name()="path"]')
if($lines.Count -ne 1){throw 'Annotated bundle must yield one fitted path.'}
$pathData=$lines[0].GetAttribute('d')
if(($pathData -replace '[-+]?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?','') -cmatch '[^ML\s]'){throw 'Unexpected fitted path command.'}
$numbers=[regex]::Matches($pathData,'[-+]?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?')
if($numbers.Count -lt 4 -or $numbers.Count % 2){throw 'Invalid fitted point list.'}
foreach($n in $numbers){
  $v=[double]::Parse($n.Value,[Globalization.CultureInfo]::InvariantCulture)
  if(-not[double]::IsFinite($v) -or [Math]::Abs($v) -gt 10000){throw 'Invalid fitted coordinate.'}
}
$widthSvg=Join-Path $fitFolder 'bundle_fit_width.svg'
[xml]$widthImage=Get-Content -LiteralPath $widthSvg -Raw
if($widthImage.OuterXml -match 'NaN|Infinity'){throw 'Nonfinite width fitting output.'}
if($widthImage.SelectNodes('//*[local-name()="path"]').Count -lt 1){throw 'Width fitting output missing.'}
$records+=[pscustomobject]@{Check='bundle-fit-output';FittedPaths=1;WidthOutput=$widthSvg}
$fitted=Read-Capture (Join-Path $fitFolder 'bundle_fit.scap')
if($fitted.Count -ne 1){throw 'Expected one fitted SCAP curve.'}
$reference=(Read-Capture (Join-Path $PSScriptRoot 'scip\fixtures\bundle.scap'))[0].Points
$maxDeviation=0.0
foreach($point in $fitted[0].Points){
  $nearest=[double]::PositiveInfinity
  for($i=1;$i -lt $reference.Count;$i++){
    $start=$reference[$i-1]; $end=$reference[$i]
    $dx=$end[0]-$start[0]; $dy=$end[1]-$start[1]
    $t=[Math]::Clamp((($point[0]-$start[0])*$dx+($point[1]-$start[1])*$dy)/($dx*$dx+$dy*$dy),0.0,1.0)
    $distance=[Math]::Sqrt([Math]::Pow($point[0]-$start[0]-$t*$dx,2)+[Math]::Pow($point[1]-$start[1]-$t*$dy,2))
    $nearest=[Math]::Min($nearest,$distance)
  }
  $maxDeviation=[Math]::Max($maxDeviation,$nearest)
}
if($maxDeviation -gt 2.0){throw 'Fitted bundle deviates beyond two fixture units.'}
$records+=[pscustomobject]@{Check='bundle-fit-geometry';Samples=$fitted[0].Points.Count;MaximumDeviation=$maxDeviation;Tolerance=2.0}
foreach($name in @('bundle','separate')){
  $directory=Join-Path $output $name
  New-Item -ItemType Directory -Path $directory | Out-Null
  $fixture=Join-Path $PSScriptRoot "scip\fixtures\$name.scap"
  $records+=Invoke-CheckedProcess "$build\author\Release\clustering-solve.exe" @('-i',$fixture,'-o',$directory,'--interm',$directory) "$name-learned"
  $scap=Join-Path $directory ($name+'_out.scap')
  # Canvas header is outside braces; only stroke headers are included below.
  $strokeText=(Get-Content -LiteralPath $scap -Raw).Split('{') | Select-Object -Skip 1
  $groups=@($strokeText | ForEach-Object {
    $match=[regex]::Match($_,'#(\d+)\t(\d+)')
    if(-not $match.Success){throw 'Missing stroke identity.'}
    $match.Groups[2].Value
  } | Select-Object -Unique)
  if($strokeText.Count -ne 3){throw 'Original stroke count changed in grouped proposal.'}
  $expected=if($name -eq 'bundle'){1}else{3}
  if($groups.Count -ne $expected){throw "$name proposed $($groups.Count) groups; expected $expected. Logs: $directory"}
  $before=Read-Capture $fixture; $after=Read-Capture $scap
  for($i=0;$i -lt $before.Count;$i++){
    if($before[$i].Id -ne $after[$i].Id -or $before[$i].Points.Count -ne $after[$i].Points.Count){throw 'Original stroke identity/samples changed.'}
    for($j=0;$j -lt $before[$i].Points.Count;$j++){
      for($k=0;$k -lt 3;$k++){
        if([Math]::Abs($before[$i].Points[$j][$k]-$after[$i].Points[$j][$k]) -gt 1e-9){throw 'Original stroke geometry/timestamp changed.'}
      }
    }
  }
  $proposals=Read-Capture (Join-Path $directory 'final_prediction_fit.scap')
  if($proposals.Count -ne $expected){throw 'Fitted proposal count does not match learned groups.'}
  $records+=[pscustomobject]@{Check="$name-groups";OriginalStrokes=3;ProposedGroups=$groups.Count;Expected=$expected}
}
$artifactPaths=@("$build\Release\scip-adapter-tests.exe","$build\author\Release\clustering-solve.exe","$build\author\Release\clustering-endend.exe","$build\author\Release\strokestrip.exe",
  (Join-Path $PSScriptRoot 'scip\fixtures\bundle.scap'),(Join-Path $PSScriptRoot 'scip\fixtures\separate.scap'))
$hashes=@($artifactPaths | ForEach-Object {[pscustomobject]@{Path=$_;SHA256=(Get-FileHash -LiteralPath $_).Hash}})
$result=[pscustomobject]@{Solver='SCIP 10.1.0 + OSQP 1.0.0 (CPU)';Results=$records;Artifacts=$hashes;FittedSvg=$svg;OutputDirectory=$output;EditorAcceptance='pending'}
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output 'results.json') -Encoding utf8
$result | ConvertTo-Json -Depth 5
