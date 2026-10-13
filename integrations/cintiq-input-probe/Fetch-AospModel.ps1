# Pinned optional research artifact download. No installed application/driver changes.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$taskCommit = '4f463a6b1de9198963dc6aff74154a504ba3f8f6'
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'models/aosp'))
New-Item -ItemType Directory -Force -Path $taskRoot | Out-Null
$taskArtifacts = @(
    @{Path='data/etc/input/motion_predictor_model.tflite'; Name='motion_predictor_model.tflite'; Hash='66dd372bbbc7717cf64067c1257117d35d6a2c727506ea39cb1cb51b83d03bd7'},
    @{Path='data/etc/input/motion_predictor_config.xml'; Name='motion_predictor_config.xml'; Hash='992778908d2dfa1bedbc202f8411c549080efc14edd123147807d40c349313d0'},
    @{Path='NOTICE'; Name='NOTICE'; Hash='5f28c11f8b1b3f97b9d42c74b85371ebf4127f241b799279bcecd69f7ee542dc'},
    @{Path='Android.bp'; Name='frameworks-native-Android.bp'; Hash='ee3f0f2a8cbce321a732b70bc886988efa7e8d8fb5d89ab7876d95ef709beca4'},
    @{Path='data/etc/input/Android.bp'; Name='Android.bp'; Hash='559aea26ee10eefbd34aa9a5534193ae5332d611314ac2ded2bee563a9b1ab64'}
)
foreach ($taskArtifact in $taskArtifacts) {
    $taskPath = Join-Path $taskRoot $taskArtifact.Name
    if (Test-Path -LiteralPath $taskPath) {
        if ((Get-FileHash -LiteralPath $taskPath).Hash.ToLowerInvariant() -ne $taskArtifact.Hash) {
            throw "Existing artifact differs: $taskPath. It was not replaced."
        }
        Write-Output "Verified existing $($taskArtifact.Name)"
        continue
    }
    $taskUrl = "https://android.googlesource.com/platform/frameworks/native/+/$taskCommit/$($taskArtifact.Path)?format=TEXT"
    $taskBytes = [Convert]::FromBase64String((Invoke-WebRequest -Uri $taskUrl).Content)
    $taskSha = [Security.Cryptography.SHA256]::Create()
    try { $taskHash = [Convert]::ToHexString($taskSha.ComputeHash($taskBytes)).ToLowerInvariant() }
    finally { $taskSha.Dispose() }
    if ($taskHash -ne $taskArtifact.Hash) { throw "Download SHA256 mismatch: $taskUrl" }
    $taskFile = [IO.File]::Open($taskPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
    try { $taskFile.Write($taskBytes, 0, $taskBytes.Length); $taskFile.Flush($true) }
    finally { $taskFile.Dispose() }
    Write-Output "Fetched and verified $($taskArtifact.Name)"
}
Write-Output 'Experimental replay only. This model is not enabled in the editor.'
