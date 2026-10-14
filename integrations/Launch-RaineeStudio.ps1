[CmdletBinding()]
param([ValidateRange(0,65535)][int]$ControlPort=0,[switch]$NoPreferences)
$ErrorActionPreference='Stop'
$taskManifest=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'SHA256-MANIFEST.json') -Raw|ConvertFrom-Json -Depth 20
$taskExe=Join-Path $PSScriptRoot 'vectorcraft.exe'
if((Get-FileHash -LiteralPath $taskExe).Hash-ne $taskManifest.exeSha256){throw 'Executable checksum differs from the verified build'}
if($ControlPort -and (Get-NetTCPConnection -State Listen -LocalPort $ControlPort -ErrorAction SilentlyContinue)){throw 'Control port occupied'}
$taskNames=@('VECTORCRAFT_CONTROL_PORT','VECTORCRAFT_NO_PREFS','RAINEE_CODEX_KIT','RAINEE_CODEX_PYTHON','PYTHONPYCACHEPREFIX','TEMP','TMP')
$taskOld=@{};foreach($taskName in $taskNames){$taskOld[$taskName]=[Environment]::GetEnvironmentVariable($taskName,'Process')}
try{
 $env:VECTORCRAFT_CONTROL_PORT=$null
 if($NoPreferences){$env:VECTORCRAFT_NO_PREFS='1'}
 $env:RAINEE_CODEX_KIT=Join-Path $PSScriptRoot 'toolkits/Codex_App_Bridge_Kit.zip'
 $env:RAINEE_CODEX_PYTHON='C:\Users\User\AppData\Local\Programs\Python\Python313\python.exe'
 $env:PYTHONPYCACHEPREFIX='D:\DevData\Rainee\cache\python'
 $env:TEMP='D:\DevData\Rainee\temp';$env:TMP=$env:TEMP
 New-Item -ItemType Directory -Path $env:TEMP -Force|Out-Null
 $taskLaunch=@{FilePath=$taskExe;WorkingDirectory=$PSScriptRoot;WindowStyle='Hidden';PassThru=$true}
 if($ControlPort){$taskLaunch.ArgumentList=@('--control',"$ControlPort")}
 Start-Process @taskLaunch|Select-Object Id
}finally{foreach($taskName in $taskNames){[Environment]::SetEnvironmentVariable($taskName,$taskOld[$taskName],'Process')}}
