[CmdletBinding()]
param([string]$Destination='D:\DevData\Rainee\build\VectorCraft-Rainee-Ink-2026-10-11')
$ErrorActionPreference='Stop'
$taskRepo=Split-Path $PSScriptRoot -Parent
$taskOutput=[IO.Path]::GetFullPath($Destination)
if(-not $taskOutput.StartsWith('D:\DevData\Rainee\build\',[StringComparison]::OrdinalIgnoreCase)){throw 'Package must remain under D:\DevData\Rainee\build'}
if(Test-Path -LiteralPath $taskOutput){throw 'Choose a fresh package directory; never replace a running or prior build.'}
$taskTarget='D:\DevData\Rainee\build\editor\debug'
foreach($taskName in @('vectorcraft.exe','vectorcraft-cli.exe')){if(-not(Test-Path -LiteralPath (Join-Path $taskTarget $taskName))){throw "Missing built executable $taskName"}}
New-Item -ItemType Directory -Path $taskOutput,(Join-Path $taskOutput 'integrations'),(Join-Path $taskOutput 'toolkits')|Out-Null
foreach($taskName in @('vectorcraft.exe','vectorcraft-cli.exe')){Copy-Item -LiteralPath (Join-Path $taskTarget $taskName) -Destination (Join-Path $taskOutput $taskName)}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'codex_bridge.py') -Destination (Join-Path $taskOutput 'integrations/codex_bridge.py')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Launch-Consolidated.ps1') -Destination (Join-Path $taskOutput 'Launch.ps1')
$taskKit='D:\0\Noir Fiction Engine\documentation\toolkits\Codex_App_Bridge_Kit.zip'
Copy-Item -LiteralPath $taskKit -Destination (Join-Path $taskOutput 'toolkits/Codex_App_Bridge_Kit.zip')
$taskPython=(& python -c 'import sys; print(sys.executable)').Trim()
Push-Location $taskRepo
try{
 $taskSources=@(rg --files -g '*.rs' -g 'Cargo.toml' -g 'Cargo.lock' | ForEach-Object {@{path=$_;sha256=(Get-FileHash -LiteralPath $_).Hash}})
 $taskManifest=@{name='VectorCraft Rainee Ink';build='optimized development build for this Windows machine, not release-performance certified';sourceRevision=(git rev-parse HEAD);dirtySource=$true;sourceFiles=$taskSources;lockSha256=(Get-FileHash Cargo.lock).Hash;python=$taskPython;package=@();runtimeChecks=@();ownerLibrary='D:\DevData\Rainee\verification\owner-art-brushes-2026-10-11\owner-art-brush-library.vectorcraft';testPolicy='Use saved Cintiq recording replay; no routine repeated physical trial'}
 foreach($taskFile in @('vectorcraft.exe','vectorcraft-cli.exe','integrations/codex_bridge.py','toolkits/Codex_App_Bridge_Kit.zip','Launch.ps1')){$taskManifest.package+=@{path=$taskFile;sha256=(Get-FileHash -LiteralPath (Join-Path $taskOutput $taskFile)).Hash}}
 foreach($taskFile in @($taskPython,'D:\DevData\Rainee\build\stripmaker-scip\author\Release\clustering-solve.exe',(Join-Path $taskRepo 'integrations/cintiq-input-probe/models/aosp/motion_predictor_model.tflite'),'D:\DevData\Rainee\venvs\neube\Scripts\python.exe')){$taskManifest.runtimeChecks+=@{path=$taskFile;exists=(Test-Path -LiteralPath $taskFile -PathType Leaf)}}
 $taskManifest|ConvertTo-Json -Depth 8|Set-Content -LiteralPath (Join-Path $taskOutput 'build-manifest.json') -Encoding utf8
}finally{Pop-Location}
Write-Output "Consolidated local package: $taskOutput"
Get-FileHash -LiteralPath (Join-Path $taskOutput 'vectorcraft.exe') | Format-List Hash
