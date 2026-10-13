# Explicit, verified migration of this task's development storage. No broad drive cleanup.
[CmdletBinding()]
param(
    [switch]$ResumeExistingCopy,
    [ValidateSet('Development','ProjectData','ProjectTemp')][string]$StorageGroup = 'Development'
)
$ErrorActionPreference = 'Stop'
$taskPlans = if ($StorageGroup -eq 'Development') { @(
    @{ Source='C:\Users\User\.cargo'; Destination='D:\DevData\Rust\cargo'; Backup='C:\Users\User\.cargo.rainee-migration-2026-10-11'; Name='cargo' },
    @{ Source='C:\Users\User\.rustup'; Destination='D:\DevData\Rust\rustup'; Backup='C:\Users\User\.rustup.rainee-migration-2026-10-11'; Name='rustup' },
    @{ Source='E:\CodexBuild\Cintiq-Input-Probe-2026-10-11'; Destination='D:\DevData\Rainee\build\Cintiq-Input-Probe-2026-10-11'; Backup='E:\CodexBuild\Cintiq-Input-Probe-2026-10-11.rainee-migration'; Name='cintiq-build' }
) } elseif ($StorageGroup -eq 'ProjectData') { @(
    @{ Source='C:\Users\User\AppData\Roaming\VectorCraft'; Destination='D:\DevData\Rainee\appdata\VectorCraft'; Backup='C:\Users\User\AppData\Roaming\VectorCraft.rainee-migration-2026-10-11'; Name='vectorcraft-appdata' },
    @{ Source='C:\Users\User\AppData\Local\RaineeCodexBridge'; Destination='D:\DevData\Rainee\appdata\RaineeCodexBridge'; Backup='C:\Users\User\AppData\Local\RaineeCodexBridge.rainee-migration-2026-10-11'; Name='rainee-bridge' },
    @{ Source='C:\Users\User\Downloads\Expanded_Research_Package_Rainee_LineArt_Slayers'; Destination='D:\DevData\Rainee\reference-inbox\Expanded_Research_Package_Rainee_LineArt_Slayers'; Backup='C:\Users\User\Downloads\Expanded_Research_Package_Rainee_LineArt_Slayers.rainee-migration-2026-10-11'; Name='legacy-research' }
) } else {
    $taskTempRoot = 'C:\Users\User\AppData\Local\Temp'
    Get-ChildItem -LiteralPath $taskTempRoot -Directory | Where-Object Name -Match '^Rainee-ArtCraft-target$|^rainee-arch$|^rainee-process-mockup-source-render$|^VectorCraft Save for Web$|^vectorcraft-testkit-|^vectorcraft-ui-place-' | ForEach-Object {
        if (([IO.Path]::GetDirectoryName($_.FullName)) -ne $taskTempRoot) { throw 'Unexpected temporary source root.' }
        $taskIsBuild = $_.Name -eq 'Rainee-ArtCraft-target'
        @{
            Source=$_.FullName
            Destination=if ($taskIsBuild) { 'D:\DevData\Rainee\build\editor' } else { Join-Path 'D:\DevData\Rainee\temp-archive' $_.Name }
            Backup=$_.FullName + '.rainee-migration-2026-10-11'
            Name=$_.Name; ArchiveOnly=-not $taskIsBuild
        }
    }
}
$taskLogRoot = 'D:\DevData\MigrationLogs\Rainee-2026-10-11'
New-Item -ItemType Directory -Path $taskLogRoot -Force | Out-Null

function Assert-RustIdle {
    param([string]$TaskCacheSource = '')
    if ($TaskCacheSource) {
        # A build in another checkout does not use this project's retired target/temp data.
        # Check both executable location and arguments before renaming this exact cache.
        $taskCacheUsers = @(Get-CimInstance Win32_Process | Where-Object {
            ($_.ExecutablePath -and $_.ExecutablePath.StartsWith($TaskCacheSource + '\', [StringComparison]::OrdinalIgnoreCase)) -or
            ($_.CommandLine -and $_.CommandLine.IndexOf($TaskCacheSource, [StringComparison]::OrdinalIgnoreCase) -ge 0)
        })
        if ($taskCacheUsers.Count -ne 0) { throw "A process uses $TaskCacheSource; preserve data and retry when it is idle." }
        return
    }
    $taskActive = @(Get-Process -Name cargo,rustc,rustup,rust-analyzer -ErrorAction SilentlyContinue)
    if ($taskActive.Count -ne 0) { throw 'A Rust process is running. Preserve data and retry when it is idle.' }
}

foreach ($taskPlan in $taskPlans) {
    if ($StorageGroup -eq 'ProjectTemp') { Assert-RustIdle -TaskCacheSource $taskPlan.Source } else { Assert-RustIdle }
    if ($StorageGroup -eq 'ProjectData' -and @(Get-Process -Name vectorcraft,vectorcraft-gemini -ErrorAction SilentlyContinue).Count -ne 0) {
        throw 'VectorCraft is running; keep preferences/recovery intact and retry after normal close.'
    }
    $taskSource = [IO.Path]::GetFullPath($taskPlan.Source)
    $taskDestination = [IO.Path]::GetFullPath($taskPlan.Destination)
    $taskBackup = [IO.Path]::GetFullPath($taskPlan.Backup)
    if (-not $taskDestination.StartsWith('D:\DevData\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Destination outside approved storage root.' }
    $taskSourceItem = Get-Item -LiteralPath $taskSource -Force
    if ($StorageGroup -eq 'ProjectTemp' -and @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($taskSource + '\', [StringComparison]::OrdinalIgnoreCase) }).Count -ne 0) {
        throw "A process uses $taskSource; preserve it and retry after normal close."
    }
    if ($taskSourceItem.Attributes -band [IO.FileAttributes]::ReparsePoint) {
        if ($taskSourceItem.LinkType -eq 'Junction' -and $taskSourceItem.Target -eq $taskDestination) {
            Write-Output "Already migrated: $taskSource -> $taskDestination"
            continue
        }
        throw "Unexpected source link: $taskSource"
    }
    if (Test-Path -LiteralPath $taskBackup) {
        throw "Existing backup requires inspection; nothing will be overwritten: $taskSource"
    }
    $taskDestinationExists = Test-Path -LiteralPath $taskDestination
    if ($taskDestinationExists -and -not $ResumeExistingCopy) { throw 'Existing destination requires explicit resume verification.' }
    if ($taskDestinationExists -and ((Get-Item -LiteralPath $taskDestination).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Unexpected destination link; stop.'
    }
    $taskReparse = @(Get-ChildItem -LiteralPath $taskSource -Recurse -Force -Attributes ReparsePoint)
    if ($taskReparse.Count -ne 0) { throw 'Unexpected nested links; stop before copying.' }
    $taskFiles = @(Get-ChildItem -LiteralPath $taskSource -Recurse -File -Force)
    $taskBytes = ($taskFiles | Measure-Object -Property Length -Sum).Sum
    $taskFree = (Get-PSDrive -Name D).Free
    if ($taskFree -lt ($taskBytes + 5GB)) { throw 'Insufficient D space including safety margin.' }
    if (-not $taskDestinationExists) {
        New-Item -ItemType Directory -Path $taskDestination -Force | Out-Null
        $taskCopyLog = Join-Path $taskLogRoot ($taskPlan.Name + '-copy.log')
        Write-Output "Copying $($taskFiles.Count) files ($taskBytes bytes): $taskSource -> $taskDestination"
        # Copy only. No /MOVE, /MIR, /PURGE, shell deletion, or existing-destination replacement.
        & robocopy.exe $taskSource $taskDestination /E /COPY:DAT /DCOPY:DAT /R:0 /W:0 /XJ /MT:8 /NFL /NDL /NP "/LOG:$taskCopyLog" | Out-Null
        if ($LASTEXITCODE -ge 8) { throw "Copy failed; source intact. Inspect $taskCopyLog" }
    } else {
        Write-Output "Verifying preserved existing copy (no overwrite): $taskSource -> $taskDestination"
    }
    $taskCopiedFiles = @(Get-ChildItem -LiteralPath $taskDestination -Recurse -File -Force)
    if ($taskCopiedFiles.Count -ne $taskFiles.Count) { throw 'File count mismatch; source intact.' }
    $taskHashes = [Collections.Generic.List[object]]::new()
    $taskDone = 0
    # Bound concurrent reads; each result still verifies source and destination contents.
    $taskFiles | ForEach-Object -Parallel {
        $ErrorActionPreference = 'Stop'
        $taskFile = $_
        $taskRelative = $taskFile.FullName.Substring($using:taskSource.Length + 1)
        $taskCopied = Join-Path $using:taskDestination $taskRelative
        try {
            # NTFS directory entries can have stale sizes for hardlinked incremental files.
            # Open-file lengths and hashes describe the actual data, not the cached listing.
            $taskSourceStream = [IO.File]::OpenRead($taskFile.FullName)
            try {
                $taskCopyStream = [IO.File]::OpenRead($taskCopied)
                try {
                    $taskActualBytes = $taskSourceStream.Length
                    if ($taskCopyStream.Length -ne $taskActualBytes) { throw 'Open-file length mismatch' }
                    $taskOriginalHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($taskSourceStream))
                    $taskCopyHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($taskCopyStream))
                } finally { $taskCopyStream.Dispose() }
            } finally { $taskSourceStream.Dispose() }
            if ($taskOriginalHash -ne $taskCopyHash) { throw 'Hash mismatch' }
            [pscustomobject]@{ Ok=$true; RelativePath=$taskRelative; Bytes=$taskActualBytes; SHA256=$taskOriginalHash }
        } catch {
            [pscustomobject]@{ Ok=$false; RelativePath=$taskRelative; Error=$_.Exception.Message }
        }
    } -ThrottleLimit 8 | ForEach-Object {
        if (-not $_.Ok) { throw "Verification failed: $($_.RelativePath): $($_.Error); source intact." }
        $taskHashes.Add([pscustomobject]@{ RelativePath=$_.RelativePath; Bytes=$_.Bytes; SHA256=$_.SHA256 })
        $taskDone++
        if ($taskDone % 10000 -eq 0) { Write-Output "Verified $taskDone / $($taskFiles.Count): $($taskPlan.Name)" }
    }
    if ($taskDone -ne $taskFiles.Count) { throw 'Incomplete hash verification; source intact.' }
    $taskBytes = ($taskHashes | Measure-Object -Property Bytes -Sum).Sum
    if ($StorageGroup -eq 'ProjectTemp') { Assert-RustIdle -TaskCacheSource $taskSource } else { Assert-RustIdle }
    $taskManifest = Join-Path $taskLogRoot ($taskPlan.Name + '-verified.json')
    if (Test-Path -LiteralPath $taskManifest) { throw 'Manifest already exists; preserve it for inspection.' }
    [pscustomobject]@{
        Source=$taskSource; Destination=$taskDestination; Backup=$taskBackup
        VerifiedAtUtc=[DateTime]::UtcNow.ToString('o'); FileCount=$taskFiles.Count; Bytes=$taskBytes
        Files=$taskHashes
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $taskManifest -Encoding utf8

    # Rename the original on its own drive, retain it until the compatibility link works.
    if ((Resolve-Path -LiteralPath $taskSource).Path -ne $taskPlan.Source) { throw 'Source path changed; stop.' }
    Move-Item -LiteralPath $taskSource -Destination $taskBackup
    try {
        if (-not $taskPlan.ArchiveOnly) {
            New-Item -ItemType Junction -Path $taskSource -Target $taskDestination | Out-Null
            $taskLink = Get-Item -LiteralPath $taskSource -Force
            if ($taskLink.LinkType -ne 'Junction' -or $taskLink.Target -ne $taskDestination) { throw 'Compatibility junction failed.' }
        }
        $taskReadRoot = if ($taskPlan.ArchiveOnly) { $taskDestination } else { $taskSource }
        $taskReadable = if ($taskHashes.Count -gt 0) { Join-Path $taskReadRoot $taskHashes[0].RelativePath } else { $taskReadRoot }
        if (-not (Test-Path -LiteralPath $taskReadable)) { throw 'Junction read failed.' }
    } catch {
        # The original is preserved at the explicit backup path; no automatic deletion on failure.
        throw "Link setup failed; original retained at $taskBackup. $($_.Exception.Message)"
    }
    $taskBackupItem = Get-Item -LiteralPath $taskBackup -Force
    $taskResolvedBackup = (Resolve-Path -LiteralPath $taskBackup).Path
    if ($taskResolvedBackup -ne $taskPlan.Backup -or ($taskBackupItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw 'Backup path guard failed; retain backup.'
    }
    # Exact verified original copy only. Every file remains hash-verified on D.
    $taskCleanupErrors = @()
    Remove-Item -LiteralPath $taskResolvedBackup -Recurse -Force -ErrorAction Continue -ErrorVariable taskCleanupErrors
    if (Test-Path -LiteralPath $taskResolvedBackup) {
        Write-Warning "Locked original copies remain at $taskResolvedBackup; verified D data and junction are intact. Do not stop unrelated processes."
        [pscustomobject]@{ Backup=$taskResolvedBackup; Destination=$taskDestination; Errors=@($taskCleanupErrors | ForEach-Object { $_.ToString() }) } |
            ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $taskLogRoot ($taskPlan.Name + '-pending-cleanup.json')) -Encoding utf8
    }
    $taskCleanupStatus = if (Test-Path -LiteralPath $taskResolvedBackup) { 'original backup partially retained' } else { 'original duplicate removed' }
    Write-Output "MIGRATED: $taskSource -> $taskDestination; manifest $taskManifest; $taskBytes verified bytes; $taskCleanupStatus."
}
