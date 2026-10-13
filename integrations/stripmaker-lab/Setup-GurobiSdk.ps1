[CmdletBinding()]
param([string]$InstallerPath='D:\DevData\Rainee\downloads\gurobi\Gurobi-13.0.3-win64.msi')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot '..\storage\Use-DDrive.ps1') -Target Editor
$installerHash='F8835DA03F204E1FBE49B8779AD53D7EC5B992396C57EC47A6930D69B520D070'
$sdkStage='D:\DevData\Rainee\tools\gurobi-sdk-13.0.3'
$sdkRoot=Join-Path $sdkStage 'gurobi1303\win64'
if((Get-FileHash -LiteralPath $InstallerPath -Algorithm SHA256).Hash -ne $installerHash){throw 'Installer differs from the verified Windows 13.0.3 package.'}
$signature=Get-AuthenticodeSignature -LiteralPath $InstallerPath
if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Gurobi'){throw 'Installer signature is not valid Gurobi.'}
if(-not(Test-Path -LiteralPath (Join-Path $sdkRoot 'include\gurobi_c++.h'))){
    New-Item -ItemType Directory -Path $sdkStage -Force | Out-Null
    $sdkLog='D:\DevData\Rainee\downloads\gurobi\sdk-extract.log'
    $process=Start-Process -FilePath 'C:\Windows\System32\msiexec.exe' -ArgumentList @('/a',('"'+$InstallerPath+'"'),'/qn',('TARGETDIR='+$sdkStage),'/L*v',$sdkLog) -WindowStyle Hidden -PassThru -Wait
    if($process.ExitCode -ne 0){throw "SDK administrative extraction failed: $($process.ExitCode)"}
}
foreach($sdkFile in @('include\gurobi_c++.h','include\gurobi_c.h','lib\gurobi_c++md2017.lib','lib\gurobi130.lib','bin\gurobi130.dll')){
    if(-not(Test-Path -LiteralPath (Join-Path $sdkRoot $sdkFile) -PathType Leaf)){throw "Incomplete SDK: $sdkFile"}
}
[pscustomobject]@{SdkRoot=$sdkRoot;InstallerSha256=$installerHash;SystemRegistration='none';LicenseActivation='not performed'}
