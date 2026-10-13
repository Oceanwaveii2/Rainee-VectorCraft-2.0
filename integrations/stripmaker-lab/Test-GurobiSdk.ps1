[CmdletBinding()]
param([string]$SdkRoot='D:\DevData\Rainee\tools\gurobi-sdk-13.0.3\gurobi1303\win64')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '..\storage\Use-DDrive.ps1') -Target Editor
$probeBuild='D:\DevData\Rainee\build\gurobi-sdk-probe'
foreach($sdkFile in @('include\gurobi_c++.h','lib\gurobi_c++md2017.lib','lib\gurobi130.lib','bin\gurobi130.dll')) {
    if(-not (Test-Path -LiteralPath (Join-Path $SdkRoot $sdkFile) -PathType Leaf)) {throw "SDK file missing: $sdkFile"}
}
$sdkSignature=Get-AuthenticodeSignature -LiteralPath (Join-Path $SdkRoot 'bin\gurobi130.dll')
if($sdkSignature.Status -ne 'Valid' -or $sdkSignature.SignerCertificate.Subject -notmatch 'Gurobi') {throw 'Invalid Gurobi runtime signature.'}
$vsWhere='C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
$vsRoot=& $vsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if(-not $vsRoot){throw 'MSVC C++ tools required.'}
& (Join-Path $vsRoot 'Common7\Tools\Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation
New-Item -ItemType Directory -Path $probeBuild -Force | Out-Null
Push-Location $probeBuild
$savedPath=$env:PATH
try {
    & cl.exe /nologo /EHsc /MD /O2 (Join-Path $PSScriptRoot 'gurobi-probe.cpp') ('/I'+(Join-Path $SdkRoot 'include')) '/Fe:gurobi-sdk-probe.exe' /link (Join-Path $SdkRoot 'lib\gurobi_c++md2017.lib') (Join-Path $SdkRoot 'lib\gurobi130.lib')
    if($LASTEXITCODE){throw 'Native Gurobi SDK compile/link failed.'}
    $env:PATH=(Join-Path $SdkRoot 'bin')+';'+$env:PATH
    & (Join-Path $probeBuild 'gurobi-sdk-probe.exe')
    $probeExit=$LASTEXITCODE
    if($probeExit){throw "Native solve failed (exit $probeExit). Compilation is verified, but a usable C++ solver license is still required."}
} finally {
    $env:PATH=$savedPath
    Pop-Location
}
