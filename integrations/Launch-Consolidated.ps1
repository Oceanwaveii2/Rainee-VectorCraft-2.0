[CmdletBinding()]
param([ValidateRange(0,65535)][int]$ControlPort=0,[switch]$NoPreferences)
$ErrorActionPreference='Stop'
$taskRoot=$PSScriptRoot
$taskManifest=Get-Content -LiteralPath (Join-Path $taskRoot 'build-manifest.json') -Raw|ConvertFrom-Json -Depth 10
foreach($taskFile in $taskManifest.package){if((Get-FileHash -LiteralPath (Join-Path $taskRoot $taskFile.path)).Hash -ne $taskFile.sha256){throw "Package file changed: $($taskFile.path)"}}
if($ControlPort -and (Get-NetTCPConnection -State Listen -LocalPort $ControlPort -ErrorAction SilentlyContinue)){throw 'Control port already used; choose a fresh port.'}
if(-not(Test-Path -LiteralPath $taskManifest.python -PathType Leaf)){throw 'Configured Python runtime unavailable; inspect build-manifest.json.'}
$taskNames=@('RAINEE_CODEX_KIT','RAINEE_CODEX_PYTHON','PYTHONPYCACHEPREFIX','TEMP','TMP','VECTORCRAFT_NO_PREFS','VECTORCRAFT_CONTROL_PORT')
$taskOld=@{};foreach($taskName in $taskNames){$taskOld[$taskName]=[Environment]::GetEnvironmentVariable($taskName,'Process')}
try{
 $env:RAINEE_CODEX_KIT=Join-Path $taskRoot 'toolkits/Codex_App_Bridge_Kit.zip'
 $env:RAINEE_CODEX_PYTHON=$taskManifest.python
 $env:PYTHONPYCACHEPREFIX='D:\DevData\Rainee\cache\python'
 $env:TEMP='D:\DevData\Rainee\temp';$env:TMP=$env:TEMP
 $env:VECTORCRAFT_CONTROL_PORT=$null
 if($NoPreferences){$env:VECTORCRAFT_NO_PREFS='1'}
 $taskArgs=if($ControlPort){@('--control',"$ControlPort")}else{@()}
 $taskStart=@{FilePath=(Join-Path $taskRoot 'vectorcraft.exe');WorkingDirectory=$taskRoot;WindowStyle='Hidden';PassThru=$true}
 if($taskArgs.Count){$taskStart.ArgumentList=$taskArgs}
 Start-Process @taskStart|Select-Object Id
}finally{foreach($taskName in $taskNames){[Environment]::SetEnvironmentVariable($taskName,$taskOld[$taskName],'Process')}}
