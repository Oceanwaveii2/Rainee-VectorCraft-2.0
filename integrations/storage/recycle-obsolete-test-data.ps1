# Explicitly authorized disposable test outputs only; never general project cleanup.
[CmdletBinding()]
param([switch]$Apply)
$ErrorActionPreference = 'Stop'
$taskArchiveRoot = 'D:\DevData\Rainee\temp-archive'
$taskLogRoot = 'D:\DevData\MigrationLogs\Rainee-2026-10-11'
$taskAllowed = '^rainee-arch$|^rainee-process-mockup-source-render$|^VectorCraft Save for Web$|^vectorcraft-testkit-|^vectorcraft-ui-place-'
$taskProcesses = @(Get-CimInstance Win32_Process)
$taskPlans = @(Get-ChildItem -LiteralPath $taskArchiveRoot -Directory -Force | ForEach-Object {
    $taskDirectory = $_
    $taskPath = (Resolve-Path -LiteralPath $taskDirectory.FullName).Path
    if ([IO.Path]::GetDirectoryName($taskPath) -ne $taskArchiveRoot -or $taskDirectory.Name -notmatch $taskAllowed) {
        throw "Not an explicitly allowed archived test directory: $taskPath"
    }
    if (($taskDirectory.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
        @(Get-ChildItem -LiteralPath $taskPath -Recurse -Force -Attributes ReparsePoint).Count) {
        throw "Linked data is not a recycle candidate: $taskPath"
    }
    $taskManifestPath = Join-Path $taskLogRoot ($taskDirectory.Name + '-verified.json')
    $taskManifest = Get-Content -LiteralPath $taskManifestPath -Raw | ConvertFrom-Json
    if ($taskManifest.Destination -ne $taskPath -or
        [IO.Path]::GetDirectoryName($taskManifest.Source) -ne 'C:\Users\User\AppData\Local\Temp') {
        throw "Manifest does not identify a migrated test directory: $taskPath"
    }
    foreach ($taskProcess in $taskProcesses) {
        foreach ($taskReference in @($taskPath,$taskManifest.Source)) {
            if (($taskProcess.ExecutablePath -and $taskProcess.ExecutablePath.StartsWith($taskReference + '\', [StringComparison]::OrdinalIgnoreCase)) -or
                ($taskProcess.CommandLine -and $taskProcess.CommandLine.IndexOf($taskReference, [StringComparison]::OrdinalIgnoreCase) -ge 0)) {
                throw "A process still references $taskReference; retain it."
            }
        }
    }
    $taskFiles = @(Get-ChildItem -LiteralPath $taskPath -Recurse -File -Force)
    if ($taskFiles.Count -ne $taskManifest.FileCount) { throw "Archived test data changed: $taskPath" }
    foreach ($taskEntry in $taskManifest.Files) {
        $taskFilePath = [IO.Path]::GetFullPath((Join-Path $taskPath $taskEntry.RelativePath))
        if (-not $taskFilePath.StartsWith($taskPath + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Manifest path escaped its test directory.' }
        if ((Get-FileHash -LiteralPath $taskFilePath -Algorithm SHA256).Hash -ne $taskEntry.SHA256) { throw "Changed test output: $taskFilePath" }
    }
    [pscustomobject]@{ Path=$taskPath; Name=$taskDirectory.Name; Files=$taskFiles.Count; Bytes=[long]$taskManifest.Bytes; Manifest=$taskManifestPath }
})
if (-not $Apply) {
    $taskPlans | ConvertTo-Json -Depth 3
    return
}
# SendToRecycleBin has no permanent-delete fallback here. Never empty the bin.
Add-Type -AssemblyName Microsoft.VisualBasic
$taskReceipt = [Collections.Generic.List[object]]::new()
foreach ($taskPlan in $taskPlans) {
    if ((Resolve-Path -LiteralPath $taskPlan.Path).Path -ne $taskPlan.Path) { throw 'Recycle target changed; stop.' }
    [Microsoft.VisualBasic.FileIO.FileSystem]::DeleteDirectory(
        $taskPlan.Path,
        [Microsoft.VisualBasic.FileIO.UIOption]::OnlyErrorDialogs,
        [Microsoft.VisualBasic.FileIO.RecycleOption]::SendToRecycleBin,
        [Microsoft.VisualBasic.FileIO.UICancelOption]::ThrowException
    )
    if (Test-Path -LiteralPath $taskPlan.Path) { throw "Recycle did not finish: $($taskPlan.Path)" }
    $taskReceipt.Add([pscustomobject]@{ OriginalPath=$taskPlan.Path; Files=$taskPlan.Files; Bytes=$taskPlan.Bytes; Manifest=$taskPlan.Manifest; RecycledAtUtc=[DateTime]::UtcNow.ToString('o') })
    $taskReceiptPath = Join-Path $taskLogRoot 'recycled-test-outputs.json'
    $taskReceipt | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath $taskReceiptPath -Encoding utf8
    Write-Output "RECYCLED: $($taskPlan.Path)"
}
Write-Output "Recycled $($taskReceipt.Count) obsolete test directories; restore through Windows Recycle Bin."
