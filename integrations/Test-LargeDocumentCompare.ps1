[CmdletBinding()]
param([string]$Candidate='D:\DevData\Rainee\build\editor\debug\vectorcraft-cli.exe',
 [string]$Baseline='D:\DevData\Rainee\build\VectorCraft-Rainee-Ink-2026-10-11-Verified\vectorcraft-cli.exe',
 [string]$OutputRoot='D:\DevData\Rainee\verification\large-document-2026-10-11-wet')
$ErrorActionPreference='Stop'
$resolved=[IO.Path]::GetFullPath($OutputRoot)
if(-not $resolved.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Output must remain on D'}
if(Test-Path -LiteralPath $resolved){throw 'Choose fresh benchmark directory'}
New-Item -ItemType Directory -Path $resolved|Out-Null
$metadata=@{baseline=@{path=$Baseline;sha256=(Get-FileHash $Baseline).Hash};candidate=@{path=$Candidate;sha256=(Get-FileHash $Candidate).Hash};threads=4;paths=50000;protocol='Sequential runs on same Windows machine and four renderer threads; medians per existing CLI benchmark. Budget failures remain failures. Not sustained interactive latency.'}
$old=$env:VECTORCRAFT_RENDER_THREADS
try{
 $env:VECTORCRAFT_RENDER_THREADS='4'
 foreach($case in @(@{name='baseline';exe=$Baseline},@{name='candidate';exe=$Candidate})){
  & $case.exe perf --paths 50000 2>&1|Tee-Object -FilePath (Join-Path $resolved "$($case.name).log")
  $metadata[$case.name].exitCode=$LASTEXITCODE
 }
 $metadata|ConvertTo-Json -Depth 5|Set-Content -LiteralPath (Join-Path $resolved 'comparison.json') -Encoding utf8
}finally{$env:VECTORCRAFT_RENDER_THREADS=$old}
Write-Output "Benchmark logs and exit codes: $resolved"
