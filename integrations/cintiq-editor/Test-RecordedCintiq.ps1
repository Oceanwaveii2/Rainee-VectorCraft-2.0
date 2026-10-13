[CmdletBinding()]
param([string]$OutputRoot)
$ErrorActionPreference='Stop'
$taskRepo=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$taskFixtures=Join-Path $taskRepo 'integrations/cintiq-input-probe/fixtures'
$taskHashes=@('E07C1F11D2BED9C82098944D6C367CA52830C53095BACA71E632C47587101AA9','6C07D6E37FCF1EBD421F0C1F3B082E8606331549CF76417779A45518CAF76FE6')
for($i=1;$i -le 2;$i++){
    $taskPath=Join-Path $taskFixtures "cintiq-2026-10-11-session-$i.json"
    if((Get-FileHash -LiteralPath $taskPath).Hash -ne $taskHashes[$i-1]){throw "Recording fixture changed: $taskPath"}
}
if(-not $OutputRoot){$OutputRoot=Join-Path 'D:\DevData\Rainee\verification' ('cintiq-canvas-replay-'+[Guid]::NewGuid().ToString('N'))}
$taskResolved=[IO.Path]::GetFullPath($OutputRoot)
if(-not $taskResolved.StartsWith('D:\DevData\Rainee\verification\',[StringComparison]::OrdinalIgnoreCase)){throw 'Replay output must remain inside D:\DevData\Rainee\verification'}
if(Test-Path -LiteralPath $taskResolved){throw 'Choose a fresh replay output directory; existing results are preserved.'}
New-Item -ItemType Directory -Path $taskResolved|Out-Null
$taskOldFixtures=$env:VECTORCRAFT_CINTIQ_FIXTURES
$taskOldOutput=$env:VECTORCRAFT_CINTIQ_REPLAY_OUTPUT
Push-Location $taskRepo
try{
    . ./integrations/storage/Use-DDrive.ps1 -Target Editor
    $env:VECTORCRAFT_CINTIQ_FIXTURES=$taskFixtures
    $env:VECTORCRAFT_CINTIQ_REPLAY_OUTPUT=$taskResolved
    cargo test -p vectorcraft-ui-egui --lib recorded_cintiq --offline -- --ignored --nocapture 2>&1 | Tee-Object -FilePath (Join-Path $taskResolved 'replay.log')
    if($LASTEXITCODE -ne 0){throw "Recorded replay failed; inspect $taskResolved\replay.log"}
    Write-Output "Replay checks passed. Editable documents and log: $taskResolved"
}finally{
    $env:VECTORCRAFT_CINTIQ_FIXTURES=$taskOldFixtures
    $env:VECTORCRAFT_CINTIQ_REPLAY_OUTPUT=$taskOldOutput
    Pop-Location
}
